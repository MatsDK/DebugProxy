<script lang="ts">
  import { ProxyState } from "$lib/proxy.svelte";
  import { X, Plus, Trash2 } from "lucide-svelte";

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
    if (e.key === "Escape") onClose();
  }
</script>

{#if isOpen}
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
    onclick={onClose}
    onkeydown={(e) => e.key === 'Escape' && onClose()}
    role="button"
    tabindex="-1"
    aria-label="Close modal"
  >
    <div
      class="w-full max-w-md bg-white dark:bg-[#161b22] rounded-lg shadow-xl border border-slate-200 dark:border-[#30363d] overflow-hidden"
      onclick={(e) => e.stopPropagation()}
      onkeydown={undefined}
      role="dialog"
      aria-modal="true"
      tabindex="-1"
    >
      <div class="px-4 py-3 border-b border-slate-200 dark:border-[#30363d] flex items-center justify-between bg-slate-50 dark:bg-[#0d1117]">
        <h3 class="text-xs font-bold uppercase tracking-wider text-slate-500">SSL Settings</h3>
        <button onclick={onClose} class="p-1 hover:bg-slate-200 dark:hover:bg-white/10 rounded transition-colors">
          <X size={14} />
        </button>
      </div>

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

      <div class="px-4 py-3 border-t border-slate-200 dark:border-[#30363d] flex justify-end bg-slate-50 dark:bg-[#0d1117]">
        <button
          onclick={onClose}
          class="px-4 py-1.5 text-xs font-bold text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 transition-colors"
        >
          CLOSE
        </button>
      </div>
    </div>
  </div>
{/if}
