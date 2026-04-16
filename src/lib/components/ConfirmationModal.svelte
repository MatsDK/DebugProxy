<script lang="ts">
  import { onMount } from "svelte";
  import { fade, scale } from "svelte/transition";

  type Props = {
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    onConfirm: () => void;
    onCancel: () => void;
  };

  let { 
    title, 
    message, 
    confirmText = "Delete", 
    cancelText = "Cancel", 
    onConfirm, 
    onCancel 
  }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onCancel();
    if (e.key === "Enter") onConfirm();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Backdrop -->
<div 
  class="fixed inset-0 z-[100] flex items-center justify-center p-4 bg-slate-900/40 dark:bg-black/60 backdrop-blur-[2px]"
  transition:fade={{ duration: 150 }}
  onclick={onCancel}
>
  <!-- Modal Card -->
  <div 
    class="w-full max-w-sm bg-white dark:bg-[#161b22] rounded-xl shadow-2xl border border-slate-200 dark:border-white/10 overflow-hidden"
    transition:scale={{ duration: 150, start: 0.98 }}
    onclick={(e) => e.stopPropagation()}
    role="dialog"
    aria-modal="true"
  >
    <div class="p-6">
      <div class="flex items-center gap-4 mb-4">
        <div class="w-10 h-10 rounded-full bg-red-50 dark:bg-red-500/10 flex items-center justify-center shrink-0">
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" class="text-red-500">
            <path d="M3 6h18"></path><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"></path><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"></path>
          </svg>
        </div>
        <div>
          <h3 class="text-base font-bold text-slate-900 dark:text-slate-100 leading-tight">{title}</h3>
          <p class="text-[11px] text-slate-500 dark:text-slate-400 mt-1 font-medium">{message}</p>
        </div>
      </div>
      
      <div class="flex items-center gap-2 mt-6">
        <button 
          onclick={onCancel}
          class="flex-1 px-4 py-2 text-xs font-bold text-slate-600 dark:text-slate-400 hover:bg-slate-100 dark:hover:bg-white/5 rounded-lg transition-colors border border-slate-200 dark:border-white/10"
        >
          {cancelText}
        </button>
        <button 
          onclick={onConfirm}
          class="flex-1 px-4 py-2 text-xs font-bold text-white bg-red-500 hover:bg-red-600 rounded-lg transition-all shadow-lg shadow-red-500/20 active:scale-95"
        >
          {confirmText}
        </button>
      </div>
    </div>
  </div>
</div>
