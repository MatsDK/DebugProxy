<script lang="ts">
  import { onMount } from "svelte";
  import { SvelteMap } from "svelte/reactivity";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { ProxyEvent } from "$lib/types";
  import Inspector from "$lib/components/Inspector.svelte";

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
  let activeMethods = $state(
    new Set(ALL_METHODS),
  );

  function toggleMethod(m: string) {
    const next = new Set(activeMethods);
    if (next.has(m)) next.delete(m);
    else next.add(m);
    activeMethods = next;
  }

  let selectedReq = $derived(selectedId ? (reqMap.get(selectedId) ?? null) : null);
  let selectedRes = $derived(selectedId ? (resMap.get(selectedId) ?? null) : null);

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
    await listen<ProxyEvent>("proxy_response", (event) => {
      resMap.set(event.payload.id, event.payload);
      resTime.set(event.payload.id, Date.now());
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
        await new Promise(r => setTimeout(r, 100));
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
</script>

<svelte:window onkeydown={handleKeydown} onmousedown={closeCtxMenu} />

<div
  class="flex flex-col h-screen bg-white dark:bg-[#0d1117] text-slate-900 dark:text-slate-100 text-[13px] antialiased"
>
  <!-- Toolbar -->
  <header
    class="flex items-center justify-between px-4 py-1.5 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0"
  >
    <div class="flex items-center gap-3">
      <span class="font-bold text-sm">Debug Proxy</span>
      <div class="w-px h-4 bg-slate-300 dark:bg-slate-600"></div>

      <span class="text-xs font-semibold text-slate-500 uppercase">IP:</span>
      <span
        class="font-mono text-xs text-blue-600 dark:text-blue-400 font-medium"
        >{ip}</span
      >

      <span class="text-xs font-semibold text-slate-500 uppercase">Port:</span>
      <label for="port-input" class="sr-only">Port</label>
      <input
        id="port-input"
        type="number"
        bind:value={port}
        disabled={isRunning}
        class="w-20 px-2 py-1 bg-white dark:bg-[#0d1117] border border-slate-300 dark:border-[#30363d] rounded text-xs focus:ring-1 focus:ring-blue-500 focus:border-blue-500 disabled:opacity-50 font-mono"
      />

      <div class="w-px h-4 bg-slate-300 dark:bg-slate-600 mx-2"></div>
      
      <label class="flex items-center gap-1.5 cursor-pointer text-[11px] font-bold text-slate-500 tracking-wide uppercase select-none hover:text-slate-700 dark:hover:text-slate-300 transition-colors">
        <input 
          type="checkbox" 
          checked={interceptSsl} 
          onchange={(e) => {
            interceptSsl = e.currentTarget.checked;
            toggleSsl();
          }} 
          class="w-3.5 h-3.5 rounded border-slate-300 dark:border-[#30363d] text-blue-600 focus:ring-blue-500 bg-white dark:bg-[#0d1117] cursor-pointer" 
        />
        <span>SSL MITM</span>
      </label>

      <button
        onclick={toggleProxy}
        class="px-3 py-1 text-xs font-semibold rounded border transition-colors {isRunning
          ? 'bg-red-600 text-white border-transparent hover:bg-red-700'
          : 'bg-blue-600 text-white border-transparent hover:bg-blue-700'}"
      >
        {isRunning ? "Stop Proxy" : "Start Proxy"}
      </button>
    </div>

    <button
      onclick={exportCert}
      class="px-3 py-1 text-xs font-semibold rounded border border-slate-300 dark:border-slate-600 bg-slate-50 dark:bg-[#161b22] hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
    >
      Download Root CA
    </button>
    <button
      onclick={() => setDark(!isDark)}
      title={isDark ? "Switch to light mode" : "Switch to dark mode"}
      class="px-2 py-1 text-xs font-medium rounded border border-slate-300 dark:border-slate-600 bg-slate-50 dark:bg-[#161b22] hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
      >{isDark ? "☀ Light" : "☾ Dark"}</button
    >
  </header>

  {#if errorMsg}
    <div class="bg-red-600 text-white text-xs font-medium px-4 py-1.5 shrink-0">
      {errorMsg}
    </div>
  {/if}

  <!-- Main: Stack top/bottom -->
  <div class="flex flex-col flex-1 min-h-0">
    <!-- Request List (top 45%) -->
    <div
      class="flex flex-col border-b border-slate-200 dark:border-[#30363d]"
      style="height: 45%"
    >
      <!-- Filter bar -->
      <div
        class="flex items-center gap-2 px-2 py-1.5 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0"
      >
        <input
          type="search"
          placeholder="Filter by URL..."
          bind:value={searchQuery}
          class="w-64 px-2 py-1 text-xs border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-[#0d1117] text-slate-900 dark:text-slate-100 focus:outline-none focus:border-blue-500"
        />

        <!-- Method toggles -->
        <div
          class="flex gap-0.5 border border-slate-300 dark:border-slate-600 rounded p-0.5"
        >
          {#each ALL_METHODS as method}
            <button
              onclick={() => toggleMethod(method)}
              class="px-2 py-0.5 text-[10px] font-mono font-semibold rounded-sm transition-colors
                {activeMethods.has(method)
                ? 'bg-slate-300 dark:bg-slate-600 text-slate-900 dark:text-slate-100'
                : 'text-slate-400 dark:text-slate-500 hover:bg-slate-100 dark:hover:bg-[#21262d]'}"
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
            reqMap.clear();
            resMap.clear();
            orderedIds = [];
            selectedId = null;
          }}
          class="px-2 py-1 text-xs font-medium rounded border border-slate-300 dark:border-slate-600 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
          >Clear</button
        >
      </div>

      <!-- Column headers -->
      <div
        class="flex px-2 py-1 text-[11px] font-semibold uppercase text-slate-500 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0"
      >
        <div class="w-14 shrink-0">Status</div>
        <div class="w-16 shrink-0">Method</div>
        <div
          class="w-56 shrink-0 overflow-hidden text-ellipsis whitespace-nowrap"
        >
          Host
        </div>
        <div class="flex-1 min-w-0">Path & Query</div>
        <div class="w-20 shrink-0 text-right">Time</div>
        <div class="w-16 shrink-0 text-right">Duration</div>
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
              class="flex items-center px-2 py-1.5 text-xs cursor-pointer border-b border-slate-100 dark:border-[#21262d] hover:bg-slate-50 dark:hover:bg-[#21262d] transition-colors
                {selectedId === id
                ? 'bg-indigo-50 dark:bg-[#1f2a3a] outline outline-1 -outline-offset-1 outline-indigo-400 dark:outline-[#388bfd]'
                : ''}"
              data-id={id}
              onclick={() => (selectedId = id)}
              oncontextmenu={(e) => openCtxMenu(e, id)}
              role="button"
              tabindex="0"
              onkeydown={(e) => e.key === "Enter" && (selectedId = id)}
            >
              <div
                class="w-14 shrink-0 font-semibold {statusColor(
                  res?.status ?? null,
                )}"
              >
                {#if res}{res.status}{:else}<span
                    class="text-amber-400 font-normal">…</span
                  >{/if}
              </div>
              <div
                class="w-16 shrink-0 font-mono font-semibold {methodColor(
                  req.method,
                )}"
              >
                {req.method}
              </div>
              <div
                class="w-56 shrink-0 overflow-hidden text-ellipsis whitespace-nowrap text-slate-500 dark:text-slate-400"
                title={domainOnly(req.uri)}
              >
                {domainOnly(req.uri)}
              </div>
              <div
                class="flex-1 min-w-0 overflow-hidden text-ellipsis whitespace-nowrap font-mono font-medium"
                title={pathOnly(req.uri)}
              >
                {pathOnly(req.uri)}
              </div>
              <div class="w-20 shrink-0 text-right font-mono text-slate-400">
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
        {#if orderedIds.length === 0}
          <div class="text-center text-slate-400 italic py-8">
            Waiting for proxy traffic...
          </div>
        {:else if filteredIds.length === 0}
          <div class="text-center text-slate-400 italic py-8">
            No results match current filters.
          </div>
        {/if}
      </div>
    </div>

    <!-- Details pane (bottom) -->
    <div class="flex flex-col flex-1 min-h-0">
      {#if selectedId && selectedReq}
        <Inspector req={selectedReq} res={selectedRes} />
      {:else}
        <div
          class="flex-1 flex items-center justify-center text-slate-400 italic text-sm border-t border-slate-200 dark:border-[#30363d] bg-white dark:bg-[#0d1117] min-h-[300px]"
        >
          Select a request from the list above to inspect details.
        </div>
      {/if}
    </div>
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
      <button onclick={() => copy(req.uri)} class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors">Copy URL</button>
      <button onclick={() => copy(JSON.stringify(req.headers, null, 2))} class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors">Copy Request Headers</button>
      {#if req.body_base64}
        <button onclick={() => copy(atob(req.body_base64!))} class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors">Copy Request Body</button>
      {/if}
      <button onclick={() => copy(buildCurl(req))} class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors">Copy as cURL</button>
    {/if}
    
    {#if res}
      <div class="h-px bg-slate-200 dark:bg-[#30363d] my-1"></div>
      <button onclick={() => copy(JSON.stringify(res.headers, null, 2))} class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors">Copy Response Headers</button>
      {#if res.body_base64}
        <button onclick={() => copy(atob(res.body_base64!))} class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors">Copy Response Body</button>
      {/if}
    {/if}
  </div>
{/if}
