<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { ProxyState } from "$lib/proxy.svelte";
  import ScriptPanel from "$lib/components/ScriptPanel.svelte";

  const proxy = new ProxyState();

  onMount(() => {
    const saved = localStorage.getItem("theme");
    const prefersDark = saved
      ? saved === "dark"
      : window.matchMedia("(prefers-color-scheme: dark)").matches;
    document.documentElement.classList.toggle("dark", prefersDark);
    proxy.init();

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

<div
  class="h-screen bg-white dark:bg-[#0d1117] overflow-hidden flex flex-col antialiased"
>
  <!-- <div class="h-9 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0 flex items-center px-4 justify-between">
    <div class="flex items-center gap-2">
      <div class="w-1.5 h-1.5 rounded-full bg-indigo-500 animate-pulse"></div>
      <span class="text-[10px] font-bold uppercase tracking-widest text-slate-500 dark:text-slate-400">Detached Script Editor</span>
    </div>
    <div class="flex items-center gap-3">
        <div class="flex items-center gap-1.5">
            <div class="w-2 h-2 rounded-full {proxy.isRunning ? 'bg-emerald-500' : 'bg-red-500'}"></div>
            <span class="text-[10px] font-mono text-slate-400 uppercase">{proxy.isRunning ? 'Proxy Active' : 'Proxy Stopped'}</span>
        </div>
    </div>
  </div> -->
  <div class="flex-1 overflow-hidden">
    <ScriptPanel scripts={proxy.scripts} />
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    overflow: hidden;
  }
</style>
