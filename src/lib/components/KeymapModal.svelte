<script lang="ts">
  import type { Keymap } from "$lib/keymap.svelte";
  import { Keyboard } from "lucide-svelte";
  import Modal from "./Modal.svelte";

  let { keymap, isOpen, onClose }: { keymap: Keymap; isOpen: boolean; onClose: () => void } = $props();

  const bindings = $derived(keymap.getBindings());
</script>

<Modal {isOpen} {onClose} maxWidth="max-w-md">
  {#snippet header()}
    <div class="flex items-center gap-3">
      <div class="p-2 rounded-xl bg-slate-100 dark:bg-white/5 text-slate-600 dark:text-slate-400">
        <Keyboard size={20} />
      </div>
      <div>
        <h3 class="text-sm font-bold text-slate-900 dark:text-white">Keyboard Shortcuts</h3>
        <p class="text-[10px] text-slate-500 font-medium">Boost your productivity</p>
      </div>
    </div>
  {/snippet}

  <div class="p-3 max-h-[60vh]">
    <div class="divide-y divide-slate-50 dark:divide-white/[0.03]">
      {#each bindings as b}
        <div class="flex items-center justify-between py-3 px-2 rounded-lg hover:bg-slate-50/50 dark:hover:bg-white/[0.02] transition-colors group">
          <span class="text-[11px] font-medium text-slate-500 dark:text-slate-400 group-hover:text-slate-700 dark:group-hover:text-slate-300 transition-colors">
            {b.description}
          </span>
          <div class="flex items-center gap-1.5">
            {#if b.ctrl}
              <kbd class="min-w-[32px] h-6 flex items-center justify-center px-1.5 rounded-md bg-white dark:bg-[#1c2128] border border-slate-200 dark:border-[#30363d] border-b-2 text-[10px] font-bold text-slate-500 dark:text-slate-400 shadow-sm">
                Ctrl
              </kbd>
            {/if}
            {#if b.shift}
              <kbd class="min-w-[32px] h-6 flex items-center justify-center px-1.5 rounded-md bg-white dark:bg-[#1c2128] border border-slate-200 dark:border-[#30363d] border-b-2 text-[10px] font-bold text-slate-500 dark:text-slate-400 shadow-sm">
                Shift
              </kbd>
            {/if}
            <kbd class="min-w-[24px] h-6 flex items-center justify-center px-2 rounded-md bg-white dark:bg-[#1c2128] border border-slate-200 dark:border-[#30363d] border-b-2 text-[10px] font-bold text-slate-900 dark:text-slate-100 shadow-sm uppercase font-mono">
              {b.key === " " ? "Space" : b.key}
            </kbd>
          </div>
        </div>
      {/each}
    </div>
  </div>

  {#snippet footer()}
    <div class="flex justify-end">
      <button 
        onclick={onClose}
        class="px-5 py-2 text-[11px] font-bold text-slate-600 dark:text-slate-300 hover:bg-slate-200 dark:hover:bg-white/10 rounded-lg transition-all active:scale-95"
      >
        Dismiss
      </button>
    </div>
  {/snippet}
</Modal>
