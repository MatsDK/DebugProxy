<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { page } from "$app/state";
  import Inspector from "$lib/components/Inspector.svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { ProxyEvent } from "$lib/types";

  let id = $derived(page.url.searchParams.get("id"));
  let event = $state<ProxyEvent | null>(null);
  let error = $state("");

  async function loadEvent() {
    if (!id) return;
    try {
      // Use raw invoke until bindings are updated
      event = await invoke<ProxyEvent>("get_event_by_id", { id });
      if (!event) {
        error = "Request not found in history cache.";
      }
    } catch (e: any) {
      error = "Failed to load request: " + e;
    }
  }

  onMount(() => {
    const saved = localStorage.getItem("theme");
    const prefersDark = saved
      ? saved === "dark"
      : window.matchMedia("(prefers-color-scheme: dark)").matches;
    document.documentElement.classList.toggle("dark", prefersDark);
    loadEvent();

    let unlisten: any;
    (async () => {
      unlisten = await listen<boolean>("theme-changed", (event) => {
        document.documentElement.classList.toggle("dark", event.payload);
      });
    })();

    return () => {
      if (unlisten) unlisten();
    };
  });
</script>

<div class="h-screen bg-slate-50 dark:bg-[#0d1117] text-slate-900 dark:text-slate-100 flex flex-col font-sans antialiased">
  <div class="h-10 border-b border-slate-200 dark:border-[#30363d] bg-white dark:bg-[#161b22] flex items-center px-4 shrink-0 transition-colors">
    <div class="flex items-center gap-2">
      <div class="w-2 h-2 rounded-full bg-indigo-500 animate-pulse"></div>
      <span class="text-xs font-bold uppercase tracking-wider text-slate-500 dark:text-slate-400">Detached Inspector</span>
      {#if event}
        <span class="text-[10px] px-1.5 py-0.5 rounded bg-slate-100 dark:bg-white/5 font-mono border border-slate-200 dark:border-white/10 text-slate-500">ID: {event.id}</span>
      {/if}
    </div>
  </div>

  <div class="flex-1 overflow-hidden relative">
    {#if error}
      <div class="absolute inset-0 flex items-center justify-center p-8 text-center">
        <div class="max-w-xs">
          <div class="text-red-500 mb-2 font-bold text-sm">Error Loading Request</div>
          <div class="text-xs text-slate-500 dark:text-slate-400 leading-relaxed font-medium">{error}</div>
          <button 
            onclick={loadEvent}
            class="mt-4 px-3 py-1.5 text-xs font-semibold rounded-md bg-indigo-500 hover:bg-indigo-600 text-white shadow-sm transition-all active:scale-95"
          >
            Retry
          </button>
        </div>
      </div>
    {:else if !event}
      <div class="absolute inset-0 flex items-center justify-center">
        <div class="flex flex-col items-center gap-3">
          <div class="w-8 h-8 border-2 border-indigo-500/20 border-t-indigo-500 rounded-full animate-spin"></div>
          <span class="text-xs font-medium text-slate-500 animate-pulse">Loading Payload...</span>
        </div>
      </div>
    {:else}
      <Inspector req={event} res={null} logs={[]} />
    {/if}
  </div>
</div>
