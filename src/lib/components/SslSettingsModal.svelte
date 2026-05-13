<script lang="ts">
  import { ProxyState } from "$lib/proxy.svelte";
  import { Plus, Trash2, Shield } from "lucide-svelte";
  import Modal from "./Modal.svelte";

  type Props = {
    proxy: ProxyState;
    isOpen: boolean;
    onClose: () => void;
  };

  let { proxy, isOpen, onClose }: Props = $props();
  let newPattern = $state("");

  function addPattern() {
    if (newPattern.trim()) {
      proxy.addSslExceptionPattern(newPattern.trim());
      newPattern = "";
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") addPattern();
  }
</script>

<Modal {isOpen} {onClose} maxWidth="max-w-md">
  {#snippet header()}
    <div class="flex items-center gap-2">
      <div class="p-1.5 rounded-lg bg-amber-500/10 text-amber-500">
        <Shield size={18} />
      </div>
      <h3 class="text-sm font-bold text-slate-900 dark:text-white">SSL Exceptions</h3>
    </div>
  {/snippet}

  <div class="p-4 flex flex-col gap-4">
    <p class="text-[10px] text-slate-500 italic">
      Traffic to these hosts will NOT be decrypted even if SSL interception is enabled. 
      Use this for services with pinned certificates (e.g., Apple, Microsoft).
    </p>

    <div class="flex gap-2">
      <input
        type="text"
        bind:value={newPattern}
        onkeydown={handleKeydown}
        placeholder="e.g. itunes.apple.com"
        class="flex-1 px-3 py-1.5 text-xs bg-slate-50 dark:bg-[#0d1117] border border-slate-200 dark:border-[#30363d] rounded focus:outline-none focus:ring-1 focus:ring-indigo-500"
      />
      <button
        onclick={addPattern}
        class="px-3 py-1.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded text-xs font-bold flex items-center gap-1 transition-colors"
      >
        <Plus size={14} />
        <span>ADD</span>
      </button>
    </div>

    <div class="max-h-60 overflow-y-auto border border-slate-200 dark:border-[#30363d] rounded bg-slate-50/50 dark:bg-black/20">
      {#if proxy.sslExceptionPatterns.length === 0}
        <div class="p-8 text-center text-[10px] text-slate-400 italic">
          No exception patterns defined
        </div>
      {:else}
        {#each proxy.sslExceptionPatterns as pattern}
          <div class="flex items-center justify-between px-3 py-2 border-b last:border-0 border-slate-200 dark:border-[#30363d] hover:bg-slate-100 dark:hover:bg-white/5 group">
            <span class="text-xs font-mono">{pattern}</span>
            <button
              onclick={() => proxy.removeSslExceptionPattern(pattern)}
              class="p-1 text-slate-400 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-all"
            >
              <Trash2 size={14} />
            </button>
          </div>
        {/each}
      {/if}
    </div>
  </div>

  {#snippet footer()}
    <div class="flex justify-end">
      <button
        onclick={onClose}
        class="px-4 py-1.5 text-xs font-bold text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 transition-colors"
      >
        CLOSE
      </button>
    </div>
  {/snippet}
</Modal>
