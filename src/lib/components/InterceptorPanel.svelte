<script lang="ts">
  import { breakpointState } from "$lib/breakpoints.svelte";
  import { methodColor, statusColor, formatTime } from "$lib/utils";
  import { Play, XCircle, Trash2, ArrowRight, RefreshCw } from "lucide-svelte";
  import { taurpc } from "$lib/rpc";
  import Inspector from "./Inspector.svelte";
  import type { ProxyState } from "$lib/proxy.svelte";
  import { PaneGroup, Pane, PaneResizer } from "paneforge";

  let { proxy }: { proxy: ProxyState } = $props();

  let selectedId = $state<string | null>(null);
  let activeBreakpoint = $derived(
    selectedId ? breakpointState.active.get(selectedId) : null
  );

  // Auto-select first breakpoint if none selected
  $effect(() => {
    if (!selectedId && breakpointState.count > 0) {
      selectedId = Array.from(breakpointState.active.keys())[0];
    }
  });

  function resume(id: string) {
    const bp = breakpointState.active.get(id);
    if (bp) {
      breakpointState.resolve(id, bp.event);
      if (selectedId === id) selectedId = null;
    }
  }

  function drop(id: string) {
    breakpointState.drop(id);
    if (selectedId === id) selectedId = null;
  }

  function resumeAll() {
    for (const id of breakpointState.active.keys()) {
      resume(id);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (selectedId && (e.ctrlKey || e.metaKey) && e.key === "Enter") {
      e.preventDefault();
      resume(selectedId);
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="flex flex-1 min-h-0 bg-white dark:bg-[#0d1117] font-sans">
  {#if breakpointState.count === 0}
    <div class="flex-1 flex flex-col items-center justify-center text-slate-400 p-8 text-center">
      <div class="w-12 h-12 rounded-full bg-slate-100 dark:bg-white/5 flex items-center justify-center mb-4">
        <Play size={24} class="opacity-20" />
      </div>
      <h3 class="text-sm font-bold text-slate-600 dark:text-slate-300">No active breakpoints</h3>
      <p class="text-[11px] mt-1 max-w-[200px]">
        Traffic paused via <code>proxy.breakpoint()</code> will appear here for manual modification.
      </p>
    </div>
  {:else}
    <PaneGroup direction="horizontal">
      <Pane defaultSize={30} minSize={20} class="flex flex-col border-r border-slate-200 dark:border-[#30363d]">
        <div class="h-8 px-3 flex items-center justify-between bg-slate-50 dark:bg-[#161b22] border-b border-slate-200 dark:border-[#30363d] shrink-0">
          <span class="text-[10px] font-bold tracking-wide text-slate-500">Paused calls</span>
          <div class="flex items-center gap-2">
            <button 
              onclick={() => taurpc.trigger_breakpoint_sync()}
              class="p-1 hover:bg-slate-200 dark:hover:bg-white/10 rounded transition-colors text-slate-400 hover:text-indigo-500"
              title="Refresh Sync"
            >
              <RefreshCw size={12} />
            </button>
            <button 
              onclick={resumeAll}
              class="text-[9px] font-bold text-indigo-600 dark:text-indigo-400 hover:underline"
            >
              Resume All
            </button>
          </div>
        </div>
        
        <div class="flex-1 overflow-y-auto">
          {#each Array.from(breakpointState.active.values()) as bp (bp.id)}
            <button
              onclick={() => selectedId = bp.id}
              class="w-full flex flex-col p-2.5 text-left border-b border-slate-100 dark:border-white/5 transition-colors {selectedId === bp.id ? 'bg-indigo-50 dark:bg-indigo-500/10' : 'hover:bg-slate-50 dark:hover:bg-white/5'}"
            >
              <div class="flex items-center justify-between mb-1">
                <span class="text-[10px] font-mono font-bold {methodColor(bp.event.method)}">{bp.event.method}</span>
                <span class="text-[9px] font-bold px-1.5 py-0.5 rounded {bp.type === 'request' ? 'bg-amber-100 dark:bg-amber-900/40 text-amber-600' : 'bg-emerald-100 dark:bg-emerald-900/40 text-emerald-600'}">
                  {bp.type.toUpperCase()}
                </span>
              </div>
              <div class="text-[11px] font-medium truncate text-slate-700 dark:text-slate-300 mb-1">{bp.event.uri}</div>
              <div class="text-[9px] text-slate-400">{formatTime(Number(bp.event.timestamp))}</div>
            </button>
          {/each}
        </div>
      </Pane>

      <PaneResizer class="w-1 bg-transparent hover:bg-indigo-500/30 cursor-col-resize shrink-0 transition-colors" />

      <Pane defaultSize={70} class="flex flex-col overflow-hidden relative">
        {#if activeBreakpoint}
          <div class="flex-1 overflow-hidden flex flex-col relative">
            <!-- Editable Inspector -->
             <Inspector 
                req={activeBreakpoint.type === 'response' 
                  ? (proxy.reqMap.get(activeBreakpoint.id) || activeBreakpoint.event) 
                  : activeBreakpoint.event} 
                res={activeBreakpoint.type === 'response' ? activeBreakpoint.event : null} 
                logs={proxy.scriptLogs}
                editable={true}
                showPopout={false}
                onAbort={() => drop(activeBreakpoint!.id)}
                onExecute={() => resume(activeBreakpoint!.id)}
             />
          </div>
        {:else}
          <div class="flex-1 flex items-center justify-center text-slate-400 italic text-[11px]">
            Select a call to inspect and modify
          </div>
        {/if}
      </Pane>
    </PaneGroup>
  {/if}
</div>
