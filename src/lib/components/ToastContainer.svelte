<script lang="ts">
  import { toast } from "$lib/toast.svelte";
  import { fly } from "svelte/transition";
  import { flip } from "svelte/animate";
</script>

<div class="fixed bottom-6 right-6 z-[9999] flex flex-col gap-3 pointer-events-none w-80">
  {#each toast.toasts as t (t.id)}
    <div
      animate:flip={{ duration: 300 }}
      in:fly={{ y: 20, opacity: 0, duration: 400 }}
      out:fly={{ x: 20, opacity: 0, duration: 300 }}
      class="pointer-events-auto bg-white/80 dark:bg-[#161b22]/95 backdrop-blur-md border border-slate-200 dark:border-white/10 rounded-xl p-4 shadow-2xl flex items-start gap-4 group relative overflow-hidden"
    >
      <!-- Type Indicator Bar -->
      <div class="absolute left-0 top-0 bottom-0 w-1 {
        t.type === 'success' ? 'bg-emerald-500' :
        t.type === 'error' ? 'bg-rose-500' :
        t.type === 'warning' ? 'bg-amber-500' : 'bg-indigo-500'
      }"></div>

      <!-- Icon-like dot -->
      <div class="mt-2 w-2 h-2 rounded-full {
        t.type === 'success' ? 'bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]' :
        t.type === 'error' ? 'bg-rose-500 shadow-[0_0_8px_rgba(244,63,94,0.5)]' :
        t.type === 'warning' ? 'bg-amber-500 shadow-[0_0_8px_rgba(245,158,11,0.5)]' : 
        'bg-indigo-500 shadow-[0_0_8px_rgba(99,102,241,0.5)]'
      } flex-shrink-0"></div>

      <div class="flex-1 min-w-0">
        <p class="text-[9px] uppercase tracking-[0.1em] font-bold text-slate-400 dark:text-slate-500 leading-none">
          {t.type}
        </p>
        <p class="text-xs font-semibold text-slate-800 dark:text-slate-100 mt-2 leading-snug break-words">
          {t.message}
        </p>
      </div>

      <button
        onclick={() => toast.remove(t.id)}
        class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-all font-bold text-lg leading-none -mt-1 h-6 w-6 flex items-center justify-center rounded-lg hover:bg-slate-100 dark:hover:bg-white/5"
      >
        ×
      </button>
    </div>
  {/each}
</div>
