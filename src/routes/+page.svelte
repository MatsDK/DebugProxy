<script lang="ts">
  import { onMount } from "svelte";
  import { SvelteMap } from "svelte/reactivity";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type {
    ProxyEvent,
    ScriptResult,
    ScriptLog,
    ScriptConfig,
  } from "$lib/types";
  import Inspector from "$lib/components/Inspector.svelte";
  import ScriptPanel from "$lib/components/ScriptPanel.svelte";
  import JsonViewer from "$lib/components/JsonViewer.svelte";

  let ip = $state("Detecting...");
  let port = $state(8080);
  let isRunning = $state(false);
  let errorMsg = $state("");
  let interceptSsl = $state(true);

  let reqMap = new SvelteMap<number, ProxyEvent>();
  let resMap = new SvelteMap<number, ProxyEvent>();
  let orderedIds = $state<number[]>([]);
  let selectedId = $state<number | null>(null);
  // Timing: ms since epoch, recorded client-side when events arrive
  let reqTime = new SvelteMap<number, number>();
  let resTime = new SvelteMap<number, number>();

  let searchQuery = $state("");
  const ALL_METHODS = [
    "GET",
    "POST",
    "PUT",
    "DELETE",
    "PATCH",
    "OPTIONS",
    "CONNECT",
  ];
  let activeTab = $state<"requests" | "scripts">("requests");
  let scriptEnabled = $state(
    localStorage.getItem("script_enabled") !== "false",
  );
  let scripts = $state<ScriptConfig[]>(
    JSON.parse(localStorage.getItem("proxy_scripts") || "[]"),
  );
  let scriptLogs = $state<ScriptLog[]>([]);
  let scriptModules = new SvelteMap<string, any>();
  const scriptUrls = new Map<string, string>();

  // Layout state
  let inspectorHeight = $state(400);
  let isResizingHeight = $state(false);
  let leftWidth = $state(280); // For script sidebar
  let isResizingLeft = $state(false);

  // Compile scripts when they change (debounced)
  let compilationTimeout: any;
  $effect(() => {
    // Make reactive to code changes
    scripts.forEach((s) => ({
      code: s.code,
      enabled: s.enabled,
      pattern: s.pattern,
    }));

    clearTimeout(compilationTimeout);
    compilationTimeout = setTimeout(async () => {
      for (const s of scripts) {
        if (!s.enabled || !s.code) {
          scriptModules.delete(s.id);
          const oldUrl = scriptUrls.get(s.id);
          if (oldUrl) {
            URL.revokeObjectURL(oldUrl);
            scriptUrls.delete(s.id);
          }
          s.compileError = undefined;
          continue;
        }

        const blob = new Blob([s.code], { type: "application/javascript" });
        const url = URL.createObjectURL(blob);
        try {
          // Use vite-ignore for dynamic import
          const mod = await import(/* @vite-ignore */ url);

          // Revoke old URL if it exists
          const oldUrl = scriptUrls.get(s.id);
          if (oldUrl) URL.revokeObjectURL(oldUrl);

          scriptUrls.set(s.id, url);
          scriptModules.set(s.id, mod);
          s.compileError = undefined; // Success!
        } catch (e: any) {
          // Revoke the failing URL immediately
          URL.revokeObjectURL(url);
          s.compileError = e.message;
          proxyApi.log(
            `[System] ${s.name} compilation error: ${e.message}`,
            0,
            "error",
          );
          console.error("Script compile fail", s.name, e);
        }
      }
    }, 300);
  });

  $effect(() => {
    localStorage.setItem("proxy_scripts", JSON.stringify(scripts));
  });
  $effect(() => {
    localStorage.setItem("script_enabled", String(scriptEnabled));
    invoke("toggle_scripting", { enabled: scriptEnabled }).catch(console.error);
  });

  $effect(() => {
    const patterns = scripts
      .filter((s) => s.enabled && s.pattern)
      .map((s) => s.pattern);
    invoke("set_script_patterns", { patterns }).catch(console.error);
  });

  const proxyApi = {
    log: (
      msg: any,
      requestId: number,
      level: "info" | "warn" | "error" = "info",
    ) => {
      scriptLogs.push({
        id: Math.random().toString(36).slice(2),
        requestId,
        level,
        message: msg,
        timestamp: Date.now(),
      });
      if (scriptLogs.length > 2000) scriptLogs.shift();
    },
  };
  (globalThis as any).proxy = proxyApi; // Legacy/Basic support

  let activeMethods = $state(
    new Set(ALL_METHODS.filter((m) => m !== "CONNECT")),
  );

  function toggleMethod(m: string) {
    const next = new Set(activeMethods);
    if (next.has(m)) next.delete(m);
    else next.add(m);
    activeMethods = next;
  }

  let selectedReq = $derived(
    selectedId ? (reqMap.get(selectedId) ?? null) : null,
  );
  let selectedRes = $derived(
    selectedId ? (resMap.get(selectedId) ?? null) : null,
  );

  let filteredIds = $derived.by(() =>
    orderedIds.filter((id) => {
      const req = reqMap.get(id);
      if (!req) return false;
      const matchesSearch =
        searchQuery === "" ||
        req.uri.toLowerCase().includes(searchQuery.toLowerCase());
      return matchesSearch && activeMethods.has(req.method.toUpperCase());
    }),
  );

  let requestList = $state<HTMLDivElement | null>(null);
  let isFollowing = $state(true);

  function onListScroll() {
    if (!requestList) return;
    const { scrollTop, scrollHeight, clientHeight } = requestList;
    // Resume auto-scroll when within 80px of bottom
    isFollowing = scrollHeight - scrollTop - clientHeight < 80;
  }

  let isDark = $state(false);

  function setDark(value: boolean) {
    isDark = value;
    document.documentElement.classList.toggle("dark", value);
    localStorage.setItem("theme", value ? "dark" : "light");
  }

  onMount(async () => {
    // Restore theme preference
    const saved = localStorage.getItem("theme");
    const prefersDark = saved
      ? saved === "dark"
      : window.matchMedia("(prefers-color-scheme: dark)").matches;
    setDark(prefersDark);

    try {
      ip = await invoke("get_local_ip");
    } catch {
      ip = "127.0.0.1";
    }

    try {
      interceptSsl = await invoke("is_ssl_intercept_enabled");
    } catch {}

    try {
      await invoke("start_proxy", { port });
      isRunning = true;
    } catch (e: any) {
      if (e === "Proxy is already running") {
        isRunning = true;
      } else {
        errorMsg = "Auto-start failed: " + e;
      }
    }

    const savedPort = localStorage.getItem("proxy_port");
    if (savedPort) port = parseInt(savedPort);

    await invoke("toggle_scripting", { enabled: scriptEnabled });

    await listen<ProxyEvent>("proxy_request", (event) => {
      reqMap.set(event.payload.id, event.payload);
      reqTime.set(event.payload.id, Date.now());
      orderedIds.push(event.payload.id);
      if (isFollowing)
        setTimeout(
          () => requestList?.scrollTo({ top: requestList.scrollHeight }),
          0,
        );
    });
    await listen<ProxyEvent>("proxy_response", async (event) => {
      resMap.set(event.payload.id, event.payload);
      resTime.set(event.payload.id, Date.now());

      if (scriptEnabled) {
        const id = event.payload.id;
        const sessionProxy = {
          log: (msg: string, level: any) =>
            proxyApi.log(msg, id, level || "info"),
        };
        for (const s of scripts) {
          if (!s.enabled) continue;
          const mod = scriptModules.get(s.id);
          if (mod?.afterResponse) {
            try {
              const isMatch =
                !s.pattern ||
                event.payload.uri.match(new RegExp(s.pattern, "i"));
              if (isMatch) {
                await mod.afterResponse(
                  reqMap.get(id),
                  event.payload,
                  sessionProxy,
                );
              }
            } catch (e) {
              proxyApi.log(
                `[${s.name}] afterResponse error: ${e}`,
                id,
                "error",
              );
            }
          }
        }
      }
    });

    await listen<ProxyEvent>("proxy_script_request", async (event) => {
      const id = event.payload.id;
      let result = { dropped: false };
      let matchingScripts = 0;

      try {
        if (scriptEnabled) {
          const sessionProxy = {
            log: (msg: string, level: any) =>
              proxyApi.log(msg, id, level || "info"),
          };
          for (const s of scripts) {
            if (!s.enabled) continue;
            const mod = scriptModules.get(s.id);
            const pattern = s.pattern || ".*";

            if (mod?.onRequest) {
              try {
                const regex = new RegExp(pattern, "i");
                const isMatch = event.payload.uri.match(regex);

                console.log(
                  `[Script: ${s.name}] Testing "${pattern}" on ${event.payload.uri} -> ${!!isMatch}`,
                );

                if (isMatch) {
                  matchingScripts++;
                  proxyApi.log(
                    `[System: ${s.name}] Matched "${pattern}"`,
                    id,
                    "info",
                  );
                  const r = await mod.onRequest(event.payload, sessionProxy);
                  if (r) result = { ...result, ...r };
                } else if (scripts.length < 5) {
                  // Only log skips if there aren't too many scripts to avoid noise
                  proxyApi.log(
                    `[System: ${s.name}] Skip (No match for "${pattern}")`,
                    id,
                    "info",
                  );
                }
              } catch (e: any) {
                proxyApi.log(
                  `[${s.name}] onRequest error: ${e.message}`,
                  id,
                  "error",
                );
              }
            }
          }

          if (matchingScripts === 0 && scripts.some((s) => s.enabled)) {
            proxyApi.log(
              "[System] No enabled scripts matched this URI pattern.",
              id,
              "info",
            );
          }
        }
      } catch (e) {
        console.error("Script engine error:", e);
      } finally {
        try {
          await invoke("submit_script_result", {
            script_id: event.payload.script_id,
            result,
          });
        } catch (e) {}
      }
    });

    await listen<ProxyEvent>("proxy_script_response", async (event) => {
      const id = event.payload.id;
      let result = { dropped: false };
      try {
        if (scriptEnabled) {
          const sessionProxy = {
            log: (msg: string, level: any) =>
              proxyApi.log(msg, id, level || "info"),
          };
          for (const s of scripts) {
            if (!s.enabled) continue;
            const mod = scriptModules.get(s.id);
            const pattern = s.pattern || ".*";
            if (mod?.onResponse) {
              try {
                const regex = new RegExp(pattern, "i");
                if (event.payload.uri.match(regex)) {
                  const r = await mod.onResponse(event.payload, sessionProxy);
                  if (r) result = { ...result, ...r };
                }
              } catch (e: any) {
                proxyApi.log(
                  `[${s.name}] onResponse error: ${e.message}`,
                  id,
                  "error",
                );
              }
            }
          }
        }
      } catch (e) {
        console.error("Script engine error (onResponse):", e);
      } finally {
        try {
          await invoke("submit_script_result", {
            script_id: event.payload.script_id,
            result,
          });
        } catch (e) {}
      }
    });
  });

  function handleKeydown(e: KeyboardEvent) {
    if (!selectedId || filteredIds.length === 0) return;
    if (
      e.target instanceof HTMLInputElement ||
      e.target instanceof HTMLTextAreaElement
    )
      return;
    if (e.key !== "ArrowUp" && e.key !== "ArrowDown") return;
    e.preventDefault();
    const idx = filteredIds.indexOf(selectedId);
    if (idx === -1) return;
    const next =
      e.key === "ArrowUp"
        ? Math.max(0, idx - 1)
        : Math.min(filteredIds.length - 1, idx + 1);
    selectedId = filteredIds[next];
    setTimeout(
      () =>
        document
          .querySelector(`[data-id="${selectedId}"]`)
          ?.scrollIntoView({ block: "nearest" }),
      0,
    );
  }

  async function toggleProxy() {
    errorMsg = "";
    if (isRunning) {
      try {
        await invoke("stop_proxy");
        isRunning = false;
      } catch (e: any) {
        errorMsg = e.toString();
      }
      return;
    }
    try {
      await invoke("start_proxy", { port });
      isRunning = true;
    } catch (e: any) {
      errorMsg = e.toString();
    }
  }

  async function toggleSsl() {
    try {
      await invoke("toggle_ssl_intercept", { enabled: interceptSsl });
      if (isRunning) {
        // Restart proxy to force-close old connections and apply new intercept state
        await invoke("stop_proxy");
        await new Promise((r) => setTimeout(r, 100));
        await invoke("start_proxy", { port });
      }
    } catch (e: any) {
      errorMsg = "Failed to toggle SSL intercept: " + e;
    }
  }

  async function exportCert() {
    try {
      const cert: string | null = await invoke("get_ca_cert");
      if (!cert) {
        errorMsg = "Certificate not ready. Start proxy first.";
        return;
      }
      const a = document.createElement("a");
      a.href = URL.createObjectURL(
        new Blob([cert], { type: "application/x-pem-file" }),
      );
      a.download = "debugger_ca.crt";
      a.click();
    } catch (e: any) {
      errorMsg = e.toString();
    }
  }

  function statusColor(status: number | null) {
    if (!status) return "text-slate-400";
    if (status < 300) return "text-emerald-500";
    if (status < 400) return "text-amber-500";
    return "text-red-500";
  }

  function methodColor(m: string) {
    return (
      {
        GET: "text-blue-500",
        POST: "text-emerald-500",
        PUT: "text-amber-500",
        DELETE: "text-red-500",
      }[m.toUpperCase()] ?? "text-slate-400"
    );
  }

  function pathOnly(uri: string) {
    try {
      const u = new URL(uri);
      return u.pathname + u.search;
    } catch {
      return uri;
    }
  }
  function domainOnly(uri: string) {
    try {
      return new URL(uri).hostname;
    } catch {
      return uri;
    }
  }
  function formatBody(b64: string | null): string {
    if (!b64) return "No body captured.";
    try {
      return JSON.stringify(JSON.parse(atob(b64)), null, 2);
    } catch {
      return atob(b64);
    }
  }
  function formatSize(b64: string | null): string {
    if (!b64) return "–";
    const n = atob(b64).length;
    if (n < 1024) return n + " B";
    if (n < 1048576) return (n / 1024).toFixed(1) + " KB";
    return (n / 1048576).toFixed(1) + " MB";
  }
  function formatTime(ms: number | undefined): string {
    if (!ms) return "–";
    const d = new Date(ms);
    return (
      d.toLocaleTimeString("en-US", {
        hour12: false,
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      }) +
      "." +
      String(d.getMilliseconds()).padStart(3, "0")
    );
  }
  function formatDuration(id: number): string {
    const t0 = reqTime.get(id);
    const t1 = resTime.get(id);
    if (!t0 || !t1) return "–";
    const ms = t1 - t0;
    if (ms < 1000) return ms + "ms";
    return (ms / 1000).toFixed(2) + "s";
  }

  // Context menu
  type CtxMenu = { x: number; y: number; id: number } | null;
  let ctxMenu = $state<CtxMenu>(null);

  function openCtxMenu(e: MouseEvent, id: number) {
    e.preventDefault();
    selectedId = id;
    ctxMenu = { x: e.clientX, y: e.clientY, id };
  }

  function closeCtxMenu() {
    ctxMenu = null;
  }

  function copy(text: string) {
    navigator.clipboard.writeText(text).catch(() => {});
    closeCtxMenu();
  }

  function buildCurl(req: ProxyEvent): string {
    const headers = req.headers
      .map(([k, v]) => `-H "${k}: ${v.replace(/"/g, '\\"')}"`)
      .join(" \\\n  ");
    const body = req.body_base64 ? `-d '${formatBody(req.body_base64)}'` : "";
    return `curl -X ${req.method} "${req.uri}" \\\n  ${headers}${body ? " \\\n  " + body : ""}`;
  }

  function handleResize(e: MouseEvent) {
    if (isResizingHeight) {
      inspectorHeight = Math.max(
        150,
        Math.min(window.innerHeight - e.clientY, window.innerHeight - 100),
      );
    } else if (isResizingLeft && activeTab === "scripts") {
      leftWidth = Math.max(150, Math.min(e.clientX, 500));
    }
  }

  function stopResize() {
    isResizingHeight = false;
    isResizingLeft = false;
  }
</script>

<svelte:window
  onkeydown={handleKeydown}
  onmousedown={closeCtxMenu}
  onmousemove={handleResize}
  onmouseup={stopResize}
/>

<div
  class="flex flex-col h-screen bg-white dark:bg-[#0d1117] text-slate-900 dark:text-slate-100 text-[13px] antialiased"
>
  <!-- Toolbar -->
  <header
    class="flex items-center justify-between px-3 h-9 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0"
  >
    <!-- Left: Navigation -->
    <div class="flex items-center gap-4 h-full font-sans">
      <span
        class="font-bold text-sm tracking-tight text-indigo-600 dark:text-indigo-400"
        >Debug Proxy</span
      >

      <nav class="flex h-full">
        <button
          onclick={() => (activeTab = "requests")}
          class="px-3 h-full text-[11px] font-bold transition-all border-b-2 {activeTab ===
          'requests'
            ? 'border-indigo-500 text-indigo-600 dark:text-indigo-400 shadow-[inset_0_-2px_0_0_rgba(99,102,241,1)]'
            : 'border-transparent text-slate-500 hover:text-slate-700 dark:hover:text-slate-300'}"
        >
          Requests
        </button>
        <button
          onclick={() => (activeTab = "scripts")}
          class="px-3 h-full text-[11px] font-bold transition-all border-b-2 {activeTab ===
          'scripts'
            ? 'border-indigo-500 text-indigo-600 dark:text-indigo-400 shadow-[inset_0_-2px_0_0_rgba(99,102,241,1)]'
            : 'border-transparent text-slate-500 hover:text-slate-700 dark:hover:text-slate-300'}"
        >
          Scripts
        </button>
      </nav>
    </div>

    <!-- Center: Proxy Controls -->
    <div
      class="flex items-center gap-3.5 bg-slate-200/50 dark:bg-white/5 px-3 py-0.5 rounded-full border border-slate-200 dark:border-white/10 shrink-0 mx-2 font-sans"
    >
      <div
        class="flex items-center gap-2 pr-3 border-l border-slate-200 dark:border-[#30363d] h-full pl-3"
      >
        <span
          class="text-[10px] font-bold text-slate-400 uppercase tracking-tight"
          >IP</span
        >
        <span
          class="text-[11px] font-mono font-bold text-[var(--color-accent)] dark:text-[var(--color-accent-dark)] select-all"
          >{ip}</span
        >
      </div>

      <div class="w-px h-3 bg-slate-300 dark:bg-slate-700 shrink-0"></div>

      <div class="flex items-center gap-1.5 shrink-0">
        <span
          class="text-[10px] font-bold text-slate-400 uppercase tracking-tight"
          >Port</span
        >
        <input
          bind:value={port}
          disabled={isRunning}
          class="w-12 bg-transparent border-none p-0 text-[11px] font-mono focus:ring-0 disabled:opacity-50 text-slate-700 dark:text-slate-300"
        />
      </div>

      <div class="w-px h-3 bg-slate-300 dark:bg-slate-700 shrink-0"></div>

      <label
        class="flex items-center gap-1.5 cursor-pointer group shrink-0 select-none"
      >
        <input
          type="checkbox"
          bind:checked={interceptSsl}
          onchange={toggleSsl}
          class="w-3 h-3 rounded border-slate-300 dark:border-slate-600 text-indigo-600 focus:ring-indigo-500 bg-white dark:bg-[#0d1117]"
        />
        <span
          class="text-[10px] font-bold text-slate-500 group-hover:text-slate-700 dark:group-hover:text-slate-300 uppercase tracking-tight"
          >SSL</span
        >
      </label>

      <button
        onclick={toggleProxy}
        class="px-3 py-0.5 text-[10px] font-black rounded-full transition-all shrink-0 {isRunning
          ? 'bg-red-500 text-white hover:bg-red-600'
          : 'bg-emerald-500 text-white hover:bg-emerald-600'}"
      >
        {isRunning ? "STOP" : "START"}
      </button>
    </div>

    <!-- Right: Utils -->
    <div class="flex items-center gap-1 shrink-0 font-sans">
      <button
        onclick={exportCert}
        title="Download Root CA"
        class="p-1.5 text-slate-500 hover:text-indigo-600 dark:hover:text-indigo-400 transition-colors"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          ><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline
            points="7 10 12 15 17 10"
          ></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg
        >
      </button>
      <button
        onclick={() => setDark(!isDark)}
        title="Toggle Theme"
        class="p-1.5 text-slate-500 hover:text-amber-500 transition-colors"
      >
        {#if isDark}
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><circle cx="12" cy="12" r="5"></circle><line
              x1="12"
              y1="1"
              x2="12"
              y2="3"
            ></line><line x1="12" y1="21" x2="12" y2="23"></line><line
              x1="4.22"
              y1="4.22"
              x2="5.64"
              y2="5.64"
            ></line><line x1="18.36" y1="18.36" x2="19.78" y2="19.78"
            ></line><line x1="1" y1="12" x2="3" y2="12"></line><line
              x1="21"
              y1="12"
              x2="23"
              y2="12"
            ></line><line x1="4.22" y1="19.78" x2="5.64" y2="18.36"></line><line
              x1="18.36"
              y1="5.64"
              x2="19.78"
              y2="4.22"
            ></line></svg
          >
        {:else}
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"
            ></path></svg
          >
        {/if}
      </button>
    </div>
  </header>

  {#if errorMsg}
    <div class="bg-red-600 text-white text-xs font-medium px-4 py-1.5 shrink-0">
      {errorMsg}
    </div>
  {/if}

  <!-- Main Content Area -->
  <div class="flex-1 flex flex-col overflow-hidden">
    {#if activeTab === "requests"}
      <div class="flex-1 flex flex-col min-h-0">
        <!-- Request List (Top) -->
        <div class="flex-1 flex flex-col min-h-0">
          <!-- Toolbar -->
          <div
            class="h-9 flex items-center gap-2 px-2 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0 font-sans"
          >
            <input
              type="search"
              placeholder="Filter by URL..."
              bind:value={searchQuery}
              class="w-48 px-2 py-0.5 text-[10px] border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-[#0d1117] text-slate-900 dark:text-slate-100 focus:outline-none focus:border-indigo-500"
            />
            <div
              class="flex gap-0.5 bg-slate-200/50 dark:bg-white/5 p-0.5 rounded-md border border-slate-200 dark:border-white/10"
            >
              {#each ALL_METHODS as method}
                <button
                  onclick={() => toggleMethod(method)}
                  class="px-1.5 py-0 text-[9px] font-mono font-bold rounded transition-colors
                    {activeMethods.has(method)
                    ? 'bg-white dark:bg-[#0d1117] text-indigo-600 dark:text-indigo-400 shadow-sm'
                    : 'text-slate-500 hover:bg-slate-100 dark:hover:bg-[#21262d]'}"
                  >{method}</button
                >
              {/each}
            </div>
            <div class="flex-1"></div>
            <span class="text-[11px] text-slate-400"
              >Showing {filteredIds.length}</span
            >
            <button
              onclick={() => {
                orderedIds = [];
                reqMap.clear();
                resMap.clear();
                selectedId = null;
              }}
              class="px-2 py-1 text-xs font-medium rounded border border-slate-300 dark:border-slate-600 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
              >Clear</button
            >
          </div>

          <!-- Column Headers -->
          <div
            class="flex px-2 py-1 text-[11px] font-semibold uppercase text-slate-500 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0"
          >
            <div class="w-14 shrink-0">Status</div>
            <div class="w-16 shrink-0">Method</div>
            <div class="w-56 shrink-0 truncate">Host</div>
            <div class="flex-1 min-w-0">Path & Query</div>
            <div class="w-20 shrink-0 text-right">Time</div>
            <div class="w-16 shrink-0 text-right">Dur</div>
            <div class="w-14 shrink-0 text-right">Size</div>
          </div>

          <!-- Rows -->
          <div
            class="flex-1 overflow-y-auto"
            bind:this={requestList}
            onscroll={onListScroll}
          >
            {#each filteredIds as id (id)}
              {@const req = reqMap.get(id)}
              {@const res = resMap.get(id)}
              {#if req}
                <div
                  class="flex items-center px-2 py-1.5 text-xs cursor-pointer border-b border-black/5 dark:border-white/5 hover:bg-slate-50 dark:hover:bg-[#1f2a3a] transition-colors
                    {selectedId === id
                    ? 'bg-indigo-50 dark:bg-[#1f2a3a] outline outline-1 -outline-offset-1 outline-indigo-400'
                    : ''}"
                  data-id={id}
                  onclick={() => (selectedId = id)}
                  oncontextmenu={(e) => openCtxMenu(e, id)}
                >
                  <div
                    class="w-14 shrink-0 font-semibold {statusColor(
                      res?.status ?? null,
                    )}"
                  >
                    {res?.status || "..."}
                  </div>
                  <div
                    class="w-16 shrink-0 font-mono font-black {methodColor(
                      req.method,
                    )}"
                  >
                    {req.method}
                  </div>
                  <div
                    class="w-56 shrink-0 truncate text-slate-500 dark:text-slate-400"
                  >
                    {domainOnly(req.uri)}
                  </div>
                  <div class="flex-1 min-w-0 truncate font-mono font-medium">
                    {pathOnly(req.uri)}
                  </div>
                  <div
                    class="w-20 shrink-0 text-right font-mono text-slate-400"
                  >
                    {formatTime(reqTime.get(id))}
                  </div>
                  <div class="w-16 shrink-0 text-right text-slate-400">
                    {formatDuration(id)}
                  </div>
                  <div class="w-14 shrink-0 text-right text-slate-400">
                    {res ? formatSize(res.body_base64) : "–"}
                  </div>
                </div>
              {/if}
            {/each}
          </div>
        </div>

        <!-- Horizontal Divider -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div
          class="h-1 bg-transparent hover:bg-indigo-500/20 cursor-row-resize transition-colors shrink-0"
          onmousedown={() => (isResizingHeight = true)}
        ></div>

        <!-- Details pane (Bottom) -->
        <div
          class="shrink-0 flex flex-col min-h-0 bg-white dark:bg-[#0d1117]"
          style="height: {inspectorHeight}px"
        >
          {#if selectedReq}
            <Inspector req={selectedReq} res={selectedRes} logs={scriptLogs} />
          {:else}
            <div
              class="flex-1 flex items-center justify-center text-slate-400 italic text-sm border-t border-slate-200 dark:border-[#30363d]"
            >
              Select a request from the list above to inspect details.
            </div>
          {/if}
        </div>
      </div>
    {:else if activeTab === "scripts"}
      <div class="flex-1 overflow-hidden">
        <ScriptPanel bind:enabled={scriptEnabled} bind:scripts />
      </div>
    {/if}
  </div>
</div>

{#if ctxMenu}
  {@const req = reqMap.get(ctxMenu.id)}
  {@const res = resMap.get(ctxMenu.id)}
  <div
    class="fixed z-50 bg-white dark:bg-[#161b22] border border-slate-200 dark:border-[#30363d] rounded shadow-lg py-1 min-w-[160px] text-xs"
    style="left: {ctxMenu.x}px; top: {ctxMenu.y}px;"
    onmousedown={(e) => e.stopPropagation()}
    role="presentation"
  >
    {#if req}
      <button
        onclick={() => copy(req.uri)}
        class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
        >Copy URL</button
      >
      <button
        onclick={() => copy(JSON.stringify(req.headers, null, 2))}
        class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
        >Copy Request Headers</button
      >
      {#if req.body_base64}
        <button
          onclick={() => copy(atob(req.body_base64!))}
          class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
          >Copy Request Body</button
        >
      {/if}
      <button
        onclick={() => copy(buildCurl(req))}
        class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
        >Copy as cURL</button
      >
    {/if}

    {#if res}
      <div class="h-px bg-slate-200 dark:bg-[#30363d] my-1"></div>
      <button
        onclick={() => copy(JSON.stringify(res.headers, null, 2))}
        class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
        >Copy Response Headers</button
      >
      {#if res.body_base64}
        <button
          onclick={() => copy(atob(res.body_base64!))}
          class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
          >Copy Response Body</button
        >
      {/if}
    {/if}
  </div>
{/if}
