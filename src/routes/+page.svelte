<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { taurpc } from "$lib/rpc";
  import { ProxyState } from "$lib/proxy.svelte";
  import Inspector from "$lib/components/Inspector.svelte";
  import ScriptPanel from "$lib/components/ScriptPanel.svelte";
  import Toggle from "$lib/components/Toggle.svelte";
  import { PaneGroup, Pane, PaneResizer } from "paneforge";
  import { windowState } from "$lib/window.svelte";
  import { Keymap } from "$lib/keymap.svelte";
  import {
    statusColor,
    methodColor,
    pathOnly,
    domainOnly,
    formatSize,
    formatTime,
    formatDuration,
  } from "$lib/utils";
  import ProxyContextMenu from "$lib/components/ProxyContextMenu.svelte";
  import SslBypassModal from "$lib/components/SslBypassModal.svelte";
  import SettingsDropdown from "$lib/components/SettingsDropdown.svelte";
  const proxy = new ProxyState();

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
    new Set(ALL_METHODS.filter((m) => m !== "CONNECT")),
  );
  function toggleMethod(m: string) {
    const next = new Set(activeMethods);
    if (next.has(m)) next.delete(m);
    else next.add(m);
    activeMethods = next;
  }

  let selectedId = $state<string | null>(null);
  let selectedReq = $derived(
    selectedId ? (proxy.reqMap.get(selectedId) ?? null) : null,
  );
  let selectedRes = $derived(
    selectedId ? (proxy.resMap.get(selectedId) ?? null) : null,
  );

  let filteredIds = $derived.by(() => {
    // console.log(`[Filter] Re-evaluating. Total IDs: ${proxy.orderedIds.length}`);
    return proxy.orderedIds.filter((id) => {
      const req = proxy.reqMap.get(id);
      if (!req) return false;
      const matchesSearch =
        searchQuery === "" ||
        req.uri.toLowerCase().includes(searchQuery.toLowerCase());
      const matchesMethod = activeMethods.has(req.method.toUpperCase());

      const result = matchesSearch && matchesMethod;
      if (!result) {
        // console.log(`[Filter] ID ${id} (${req.method} ${req.uri}) hidden by filter`);
      }
      return result;
    });
  });

  let requestList = $state<HTMLDivElement | null>(null);
  let isFollowing = $state(true);
  let displayLimit = $state(200);

  let renderedIds = $derived(filteredIds.slice(-displayLimit));

  function onListScroll() {
    if (!requestList) return;
    const { scrollTop, scrollHeight, clientHeight } = requestList;
    isFollowing = scrollHeight - scrollTop - clientHeight < 80;

    // Load more when scrolling near the top
    if (scrollTop < 100 && displayLimit < filteredIds.length) {
      const oldHeight = scrollHeight;
      displayLimit = Math.min(filteredIds.length, displayLimit + 200);
      
      // Preserve relative scroll position after the DOM updates
      setTimeout(() => {
        if (requestList) {
          requestList.scrollTop = requestList.scrollHeight - oldHeight + scrollTop;
        }
      }, 0);
    }
  }

  $effect(() => {
    proxy.orderedIds.length;
    if (isFollowing)
      setTimeout(
        () => requestList?.scrollTo({ top: requestList.scrollHeight }),
        0,
      );
  });

  onMount(() => {
    // Listen for theme changes from other windows
    let unlistenTheme: (() => void) | undefined;
    // Listen for popped out windows being closed
    let unlistenWindow: (() => void) | undefined;

    (async () => {
      const saved = localStorage.getItem("theme");
      const prefersDark = saved
        ? saved === "dark"
        : window.matchMedia("(prefers-color-scheme: dark)").matches;
      proxy.setTheme(prefersDark);
      await proxy.init();

      unlistenTheme = await taurpc.events.theme_changed.on((dark) => {
        proxy.setTheme(dark, false);
      });

      unlistenWindow = await taurpc.events.window_closed.on((label) => {
        if (label.startsWith("inspector-")) {
          const id = label.replace("inspector-", "");
          windowState.toggleInspector(id, false);
        } else if (label === "scripts-editor") {
          windowState.toggleScripts(false);
        }
      });
    })();

    const detachKeymap = keymap.attach();

    return () => {
      if (unlistenTheme) unlistenTheme();
      if (unlistenWindow) unlistenWindow();
      detachKeymap();
    };
  });

  let isSslBypassOpen = $state(false);

  async function toggleProxy() {
    await proxy.toggleProxy();
  }

  async function toggleSsl(val: boolean) {
    try {
      await proxy.toggleSsl(val);
    } catch (e: any) {
      proxy.errorMsg = "Failed to toggle SSL: " + e;
    }
  }

  async function toggleBlocked(val: boolean) {
    try {
      await proxy.toggleBlocked(val);
    } catch (e: any) {
      proxy.errorMsg = "Failed to toggle block: " + e;
    }
  }

  const keymap = new Keymap()
    .bind(
      "l",
      () => {
        const el = document.getElementById(
          "url-filter-input",
        ) as HTMLInputElement | null;
        el?.focus();
        el?.select();
      },
      { ctrl: true, global: true },
    )
    .bind(
      "k",
      () => {
        proxy.clearTraffic();
        displayLimit = 200;
      },
      { ctrl: true, global: true },
    )
    .bind(
      "j",
      (e) => {
        const t = document.activeElement as HTMLElement | null;
        if (t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement) {
          t.blur();
        }
        if (!selectedId && filteredIds.length > 0) {
          selectedId = filteredIds[0];
          setTimeout(
            () =>
              document
                .querySelector(`[data-id="${selectedId}"]`)
                ?.scrollIntoView({ block: "nearest" }),
            0,
          );
        }
      },
      { ctrl: true, global: true },
    )
    .bind(
      "Escape",
      (e) => {
        const t = e.target as HTMLElement;
        if (t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement) {
          t.blur();
        } else {
          selectedId = null;
        }
      },
      { global: true },
    )
    .bind("ArrowUp", () => {
      if (!selectedId || filteredIds.length === 0) return;
      const idx = filteredIds.indexOf(selectedId);
      if (idx <= 0) return;
      selectedId = filteredIds[idx - 1];
      setTimeout(
        () =>
          document
            .querySelector(`[data-id="${selectedId}"]`)
            ?.scrollIntoView({ block: "nearest" }),
        0,
      );
    })
    .bind("ArrowDown", () => {
      if (!selectedId || filteredIds.length === 0) return;
      const idx = filteredIds.indexOf(selectedId);
      if (idx === filteredIds.length - 1) return;
      selectedId = filteredIds[idx + 1];
      setTimeout(
        () =>
          document
            .querySelector(`[data-id="${selectedId}"]`)
            ?.scrollIntoView({ block: "nearest" }),
        0,
      );
    });

  let activeTab = $state<"requests" | "scripts">("requests");

  import type { CtxMenu } from "$lib/types";
  let ctxMenu = $state<CtxMenu>(null);
  function openCtxMenu(e: MouseEvent, id: string) {
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
</script>

<svelte:window
  onmousedown={closeCtxMenu}
  onmousemove={() => {}}
  onmouseup={() => {}}
/>

<div
  class="flex flex-col h-screen bg-white dark:bg-[#0d1117] text-slate-900 dark:text-slate-100 text-[13px] antialiased"
>
  <header
    class="flex items-center justify-between px-3 h-9 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0"
  >
    <div class="flex items-center gap-4 h-full font-sans">
      <span
        class="font-bold text-sm tracking-tight text-indigo-600 dark:text-indigo-400"
        >Debug Proxy</span
      >
      <nav class="flex h-full ml-4">
        <button
          onclick={() => (activeTab = "requests")}
          class="px-3 h-full text-[11px] font-bold transition-all border-b-2 {activeTab ===
          'requests'
            ? 'border-indigo-500 text-indigo-600 dark:text-indigo-400'
            : 'border-transparent text-slate-500 hover:text-slate-700'}"
          >Requests</button
        >
        {#if !windowState.isScriptsPoppedOut}
          <button
            onclick={() => (activeTab = "scripts")}
            class="px-3 h-full text-[11px] font-bold transition-all border-b-2 {activeTab ===
            'scripts'
              ? 'border-indigo-500 text-indigo-600 dark:text-indigo-400'
              : 'border-transparent text-slate-500 hover:text-slate-700'}"
            >Scripts</button
          >
        {/if}
      </nav>
    </div>

    <!-- Proxy Controls -->
    <div
      class="flex items-center gap-0 bg-slate-200/50 dark:bg-white/5 pl-2 pr-1 py-px rounded-full border border-slate-200 dark:border-white/10 shrink-0 mx-2 font-sans h-7 min-w-0 shadow-inner"
    >
      <!-- IP Section -->
      <div
        class="flex items-center gap-1.5 px-3 border-r border-slate-300 dark:border-white/10"
      >
        <span
          class="text-[13px] font-bold text-slate-400 uppercase tracking-wide"
          >IP</span
        >
        <span
          class="text-[13px] font-bold text-indigo-600 dark:text-indigo-400 uppercase select-all"
          >{proxy.ip}</span
        >
      </div>

      <!-- Port Section -->
      <div
        class="flex items-center gap-1.5 px-3 border-r border-slate-300 dark:border-white/10"
      >
        <span
          class="text-[13px] font-bold text-slate-400 uppercase tracking-wide"
          >Port</span
        >
        <input
          bind:value={proxy.port}
          disabled={proxy.isRunning}
          class="w-10 bg-transparent border-none p-0 text-[13px] font-mono font-bold focus:ring-0 text-slate-700 dark:text-slate-200 h-auto -translate-y-[0.5px]"
        />
      </div>

      <!-- SSL Section -->
      <div
        class="flex items-center px-3 border-r border-slate-300 dark:border-white/10"
      >
        <Toggle
          bind:checked={proxy.interceptSsl}
          onchange={toggleSsl}
          label="SSL"
          size="sm"
        />
        <button
          onclick={() => (isSslBypassOpen = true)}
          class="ml-1 p-1 text-slate-400 hover:text-indigo-500 transition-colors"
          title="SSL Bypass Settings"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="12"
            height="12"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><path
              d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"
            /><circle cx="12" cy="12" r="3" /></svg
          >
        </button>
      </div>

      <!-- Block Section -->
      <div
        class="flex items-center px-3 border-r border-slate-300 dark:border-white/10"
      >
        <Toggle
          bind:checked={proxy.isBlocked}
          onchange={toggleBlocked}
          label="Block"
          size="sm"
        />
      </div>

      <!-- Action Button -->
      <div class="px-1.5 flex items-center justify-center">
        <button
          onclick={toggleProxy}
          class="px-4 h-[22px] text-[11px] pt-1 font-black tracking-wider rounded-full transition-all shrink-0 flex items-center justify-center {proxy.isRunning
            ? 'bg-red-500 text-white hover:bg-red-600 shadow-sm shadow-red-500/20'
            : 'bg-emerald-500 text-white hover:bg-emerald-600 shadow-sm shadow-emerald-500/20'}"
        >
          {proxy.isRunning ? "STOP" : "START"}
        </button>
      </div>
    </div>

    <!-- Utils -->
    <div class="flex items-center gap-1 shrink-0 font-sans">
      <span
        class="text-[10px] font-bold text-slate-400 bg-slate-100 dark:bg-white/5 px-2 py-0.5 rounded mr-2"
        >Total: {proxy.orderedIds.length}</span
      >
      <SettingsDropdown {proxy} />
    </div>
  </header>

  {#if proxy.errorMsg}
    <div class="bg-red-600 text-white text-xs font-medium px-4 py-1.5 shrink-0">
      {proxy.errorMsg}
    </div>
  {/if}

  <!-- Main Content Area -->
  <div class="flex-1 flex flex-col overflow-hidden">
    {#if activeTab === "requests"}
      <PaneGroup direction="vertical" autoSaveId="requests-layout-v1">
        <!-- Top: Request List -->
        <Pane
          defaultSize={60}
          minSize={20}
          class="flex flex-col border-b border-slate-200 dark:border-[#30363d]"
        >
          <div
            class="flex-1 flex flex-col min-h-0 bg-white dark:bg-[#0d1117] border-r border-slate-200 dark:border-[#30363d]"
          >
            {#if proxy.errorMsg}
              <div
                class="px-3 py-2 bg-red-500/10 border-b border-red-500/20 flex items-center gap-3 animate-in fade-in slide-in-from-top-2 duration-300"
              >
                <div
                  class="w-1.5 h-1.5 rounded-full bg-red-500 animate-pulse"
                ></div>
                <div class="flex-1 min-w-0">
                  <p
                    class="text-[10px] font-bold text-red-500 uppercase tracking-tighter"
                  >
                    Proxy Error
                  </p>
                  <p class="text-[10px] text-red-400 font-mono truncate">
                    {proxy.errorMsg}
                  </p>
                </div>
                <button
                  onclick={() => proxy.startProxy()}
                  class="px-2 py-0.5 text-[9px] font-bold bg-red-500 hover:bg-red-600 text-white rounded transition-all active:scale-95"
                  >RETRY</button
                >
              </div>
            {/if}

            <div
              class="h-9 flex items-center gap-2 px-2 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0 font-sans"
            >
              <input
                id="url-filter-input"
                type="search"
                placeholder="Filter by URL... (Ctrl+L)"
                bind:value={searchQuery}
                class="w-48 px-2 py-0.5 text-[10px] border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-[#0d1117] focus:outline-none focus:border-indigo-500"
              />
              <div
                class="flex gap-0.5 bg-slate-200/50 dark:bg-white/5 p-0.5 rounded-md border border-slate-200 dark:border-white/10"
              >
                {#each ALL_METHODS as method}
                  <button
                    onclick={() => toggleMethod(method)}
                    class="px-1.5 py-0 text-[9px] font-mono font-bold rounded transition-colors {activeMethods.has(
                      method,
                    )
                      ? 'bg-white dark:bg-[#0d1117] text-indigo-600 dark:text-indigo-400'
                      : 'text-slate-500'}">{method}</button
                  >
                {/each}
              </div>
              <div class="flex-1"></div>
              <button
                onclick={() => {
                  proxy.clearTraffic();
                  displayLimit = 200;
                }}
                class="px-2 py-1 text-xs font-medium rounded border border-slate-300 dark:border-slate-600 hover:bg-slate-100 dark:hover:bg-[#21262d] transition-colors"
                >Clear</button
              >
            </div>

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

            <div
              class="flex-1 overflow-y-auto no-scrollbar"
              bind:this={requestList}
              onscroll={onListScroll}
            >
              {#if filteredIds.length > displayLimit}
                <div
                  class="py-4 text-center text-[10px] text-slate-400 font-mono border-b border-dashed border-slate-200 dark:border-[#30363d]"
                >
                  {filteredIds.length - displayLimit} more historical requests available
                  (scroll up to load)
                </div>
              {/if}
              {#each renderedIds as id (id)}
                {@const req = proxy.reqMap.get(id)}
                {@const res = proxy.resMap.get(id)}
                {#if req}
                  <div
                    class="flex items-center px-2 py-1.5 text-xs cursor-pointer border-b border-black/5 dark:border-white/5 hover:bg-slate-50 dark:hover:bg-[#1f2a3a] transition-colors {selectedId ===
                    id
                      ? 'bg-indigo-50 dark:bg-[#1f2a3a] outline outline-1 -outline-offset-1 outline-indigo-400'
                      : ''}"
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
                      class="w-16 shrink-0 font-mono font-black flex items-center gap-1.5 {methodColor(
                        req.method,
                      )}"
                    >
                      {req.method}
                      {#if req.script_id !== "0"}
                        <span
                          class="px-1 py-0 rounded-[3px] bg-amber-100 dark:bg-amber-500/20 text-amber-700 dark:text-amber-400 text-[9px] font-black leading-none border border-amber-200 dark:border-amber-500/30 shadow-sm"
                          title="Scripted">JS</span
                        >
                      {/if}
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
                      {formatTime(proxy.reqTime.get(id))}
                    </div>
                    <div class="w-16 shrink-0 text-right text-slate-400">
                      {formatDuration(
                        proxy.reqTime.get(id),
                        proxy.resTime.get(id),
                      )}
                    </div>
                    <div class="w-14 shrink-0 text-right text-slate-400">
                      {res ? formatSize(res.body) : "–"}
                    </div>
                  </div>
                {/if}
              {/each}
            </div>
          </div>
        </Pane>

        <PaneResizer
          class="h-1 bg-transparent hover:bg-indigo-500/20 cursor-row-resize transition-colors"
        />

        <!-- Bottom: Inspector -->
        <Pane
          defaultSize={40}
          minSize={10}
          class="flex flex-col min-h-0 bg-white dark:bg-[#0d1117]"
        >
          {#if selectedReq}
            {#if windowState.poppedOutInspectors.has(selectedReq.id)}
              <div
                class="flex-1 flex flex-col items-center justify-center text-slate-400 bg-indigo-50/5 dark:bg-transparent"
              >
                <p
                  class="text-xs font-bold uppercase tracking-widest text-indigo-500/60"
                >
                  Inspector Detached
                </p>
                <p class="text-[10px] opacity-40 mt-1">
                  This request is being inspected in a separate window
                </p>
                <button
                  onclick={() =>
                    windowState.toggleInspector(selectedReq!.id, false)}
                  class="mt-4 px-3 py-1 text-[10px] font-bold border border-indigo-500/30 rounded-full hover:bg-indigo-500/10 transition-colors"
                  >Restore to Dashboard</button
                >
              </div>
            {:else}
              <Inspector
                req={selectedReq}
                res={selectedRes || null}
                logs={proxy.scriptLogs}
              />
            {/if}
          {:else}
            <div
              class="flex-1 flex flex-col items-center justify-center text-slate-400 bg-slate-50/10 dark:bg-transparent"
            >
              <p
                class="text-xs font-medium uppercase tracking-widest opacity-60"
              >
                No Request Selected
              </p>
              <p class="text-[10px] opacity-40 mt-1 italic">
                Select traffic from the list above
              </p>
            </div>
          {/if}
        </Pane>
      </PaneGroup>
    {:else}
      <div class="flex-1 overflow-hidden flex flex-col">
        {#if windowState.isScriptsPoppedOut}
          <div
            class="flex-1 flex flex-col items-center justify-center text-slate-400"
          >
            <p
              class="text-xs font-bold uppercase tracking-widest text-amber-500/60"
            >
              Script Editor Detached
            </p>
            <p class="text-[10px] opacity-40 mt-1">
              Editing is currently active in a separate window
            </p>
            <button
              onclick={() => {
                windowState.toggleScripts(false);
                activeTab = "scripts";
              }}
              class="mt-4 px-3 py-1 text-[10px] font-bold border border-amber-500/30 rounded-full hover:bg-amber-500/10 transition-colors"
              >Restore to Dashboard</button
            >
          </div>
        {:else}
          <ScriptPanel scripts={proxy.scripts} />
        {/if}
      </div>
    {/if}
  </div>
</div>

{#if ctxMenu}
  {@const req = proxy.reqMap.get(ctxMenu.id)}
  {@const res = proxy.resMap.get(ctxMenu.id)}
  <ProxyContextMenu
    {req}
    {res}
    x={ctxMenu.x}
    y={ctxMenu.y}
    onClose={closeCtxMenu}
  />
{/if}

<SslBypassModal
  {proxy}
  isOpen={isSslBypassOpen}
  onClose={() => (isSslBypassOpen = false)}
/>
