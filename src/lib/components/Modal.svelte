<script lang="ts">
  import { fade, scale } from "svelte/transition";
  import type { Snippet } from "svelte";

  type Props = {
    isOpen: boolean;
    onClose: () => void;
    title?: string;
    children: Snippet;
    header?: Snippet;
    footer?: Snippet;
    maxWidth?: string;
  };

  let { 
    isOpen, 
    onClose, 
    title, 
    children, 
    header, 
    footer, 
    maxWidth = "max-w-md" 
  }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape" && isOpen) {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if isOpen}
  <!-- Backdrop -->
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div 
    class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-slate-900/40 dark:bg-black/60 backdrop-blur-[2px]"
    transition:fade={{ duration: 150 }}
    onclick={onClose}
  >
    <!-- Modal Card -->
    <div 
      class="w-full {maxWidth} bg-white dark:bg-[#161b22] rounded-xl shadow-2xl border border-slate-200 dark:border-[#30363d] overflow-hidden flex flex-col"
      transition:scale={{ duration: 150, start: 0.98 }}
      onclick={(e) => e.stopPropagation()}
    >
      {#if header || title}
        <div class="flex items-center justify-between px-5 py-4 border-b border-slate-100 dark:border-[#30363d] bg-slate-50/50 dark:bg-white/5">
          {#if header}
            {@render header()}
          {:else if title}
            <h3 class="text-sm font-bold text-slate-900 dark:text-white">{title}</h3>
          {/if}
          
          <button 
            onclick={onClose}
            class="p-1 rounded-md text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 hover:bg-slate-100 dark:hover:bg-white/10 transition-colors"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M18 6 6 18"></path><path d="m6 6 12 12"></path>
            </svg>
          </button>
        </div>
      {/if}

      <div class="flex-1 overflow-y-auto">
        {@render children()}
      </div>

      {#if footer}
        <div class="p-4 bg-slate-50 dark:bg-[#0d1117] border-t border-slate-100 dark:border-[#30363d]">
          {@render footer()}
        </div>
      {/if}
    </div>
  </div>
{/if}
