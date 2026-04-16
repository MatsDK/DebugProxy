<script lang="ts">
  let { 
    id, 
    checked = $bindable(false), 
    disabled = false,
    label = "",
    size = "md",
    onchange
  }: { 
    id?: string;
    checked: boolean;
    disabled?: boolean;
    label?: string;
    size?: "sm" | "md";
    onchange?: (val: boolean) => void;
  } = $props();

  const sizeClasses = {
    sm: {
      track: "w-8 h-4.5",
      thumb: "w-3.5 h-3.5",
      translate: "translate-x-3.5"
    },
    md: {
      track: "w-10 h-5.5",
      thumb: "w-4.5 h-4.5",
      translate: "translate-x-4.5"
    }
  };

  const currentSize = $derived(sizeClasses[size]);
</script>

<label class="inline-flex items-center gap-2 cursor-pointer select-none {disabled ? 'opacity-50 cursor-not-allowed' : ''}">
  <div class="relative flex items-center h-full">
    <input 
      type="checkbox" 
      {id}
      bind:checked 
      {disabled}
      class="sr-only peer"
      onchange={() => onchange?.(checked)}
    />
    <div class="{currentSize.track} bg-slate-200 dark:bg-[#30363d] rounded-full peer peer-checked:bg-indigo-600 transition-colors"></div>
    <div class="absolute {currentSize.thumb} bg-white rounded-full left-[2px] top-[2px] transition-all {checked ? currentSize.translate : ''} shadow-sm border border-black/5"></div>
  </div>
  {#if label}
    <span class="text-[13px] font-bold text-slate-500 dark:text-slate-400 leading-none">{label}</span>
  {/if}
</label>
