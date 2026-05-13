<script lang="ts">
  import Modal from "./Modal.svelte";

  type Props = {
    isOpen: boolean;
    title: string;
    message: string;
    confirmText?: string;
    cancelText?: string;
    onConfirm: () => void;
    onCancel: () => void;
  };

  let { 
    isOpen = true,
    title, 
    message, 
    confirmText = "Delete", 
    cancelText = "Cancel", 
    onConfirm, 
    onCancel 
  }: Props = $props();

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && isOpen) onConfirm();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<Modal {isOpen} onClose={onCancel} maxWidth="max-w-sm">
  <div class="p-6">
    <div class="flex items-center gap-4">
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
  </div>

  {#snippet footer()}
    <div class="flex items-center gap-2">
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
  {/snippet}
</Modal>
