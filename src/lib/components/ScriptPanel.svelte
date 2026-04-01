<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { ScriptConfig } from "../types";

  let {
    enabled = $bindable(false),
    scripts = $bindable([]),
  }: {
    enabled: boolean;
    scripts: ScriptConfig[];
  } = $props();

  let selectedId = $state<string | null>(null);
  let selectedScript = $derived(
    scripts.find((s) => s.id === selectedId) || null,
  );

  const DEFAULT_SCRIPT_TEMPLATE = `/**
 * Intercept and rewrite requests
 * @param {ProxyEvent} request
 * @param {ProxyApi} proxy
 */
export async function onRequest(request, proxy) {
  if (request.body_base64) {
    try {
      // Decode base64 body to string
      const decoded = atob(request.body_base64);
      const obj = JSON.parse(decoded);

      if (obj.Records && Array.isArray(obj.Records)) {
        for (const record of obj.Records) {
          try {
            // Decode Kinesis record data
            const data = JSON.parse(atob(record.Data));
            proxy.log(data, "info");
          } catch (e) {
             // Record decode failed, ignore or log as string
          }
        }
      } else {
        proxy.log(obj, "info");
      }
    } catch (e) {
      // Not JSON or decoding failed, ignore silently
    }
  }
  return { dropped: false };
}

/**
 * Intercept and rewrite responses
 */
export async function onResponse(response, proxy) {
  return {};
}

/**
 * Perform non-blocking analysis phase
 */
export async function afterResponse(request, response, proxy) {
  // Analytical work here...
}
`;

  onMount(() => {
    if (scripts.length === 0) {
      addScript();
    } else {
      selectedId = scripts[0].id;
    }
  });

  async function handleToggle() {
    try {
      await invoke("toggle_scripting", { enabled });
    } catch (e) {
      console.error(e);
      enabled = false;
    }
  }

  function addScript() {
    const id = Math.random().toString(36).slice(2);
    const newScript: ScriptConfig = {
      id,
      name: "Kinesis Decoder",
      pattern: "kinesis.eu-west-1.amazonaws.com",
      code: DEFAULT_SCRIPT_TEMPLATE,
      enabled: true,
    };
    scripts = [...scripts, newScript];
    selectedId = id;
  }

  function deleteScript(id: string) {
    scripts = scripts.filter((s) => s.id !== id);
    if (selectedId === id) {
      selectedId = scripts[0]?.id || null;
    }
  }
</script>

<div
  class="flex flex-col h-full bg-white dark:bg-[#0d1117] text-slate-900 dark:text-slate-100 font-sans"
>
  <!-- Toolbar -->
  <div
    class="h-8 flex items-center justify-between px-2 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0 font-sans"
  >
    <div class="flex items-center gap-3">
      <label class="flex items-center gap-2 cursor-pointer select-none group">
        <input
          type="checkbox"
          bind:checked={enabled}
          onchange={handleToggle}
          class="w-3 h-3 rounded border-slate-300 dark:border-slate-600 bg-white dark:bg-[#0d1117] text-indigo-600 focus:ring-indigo-500"
        />
        <span
          class="text-[10px] font-bold uppercase tracking-tight {enabled
            ? 'text-indigo-600 dark:text-indigo-400'
            : 'text-slate-400 group-hover:text-slate-600'}"
        >
          {enabled ? "Scripting Active" : "Scripting Disabled"}
        </span>
      </label>
      <div class="h-3 w-px bg-slate-300 dark:bg-slate-700"></div>
      <span
        class="text-[9px] text-slate-400 font-medium uppercase tracking-tighter"
        >Regex Scoped Execution</span
      >
    </div>
  </div>

  <div class="flex-1 flex overflow-hidden">
    <!-- Script Sidebar -->
    <div
      class="w-64 border-r border-slate-200 dark:border-[#30363d] flex flex-col bg-slate-50 dark:bg-[#0d1117]"
    >
      <div
        class="flex items-center justify-between p-2 bg-slate-100 dark:bg-[#161b22] border-b border-slate-200 dark:border-[#30363d]"
      >
        <span
          class="text-[10px] font-bold text-slate-500 uppercase tracking-widest"
          >Your Scripts</span
        >
        <button
          onclick={addScript}
          class="p-1 hover:text-indigo-600 dark:hover:text-indigo-400 transition-colors"
          title="Add Script"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><line x1="12" y1="5" x2="12" y2="19"></line><line
              x1="5"
              y1="12"
              x2="19"
              y2="12"
            ></line></svg
          >
        </button>
      </div>
      <div class="flex-1 overflow-y-auto">
        {#each scripts as s}
          <button
            onclick={() => (selectedId = s.id)}
            class="w-full text-left p-3 border-b border-slate-100 dark:border-[#21262d] transition-colors flex flex-col gap-1
              {selectedId === s.id
              ? 'bg-indigo-50 dark:bg-[#1f2a3a] border-r-2 border-r-indigo-500 dark:border-r-indigo-400'
              : 'hover:bg-slate-100 dark:hover:bg-[#161b22]'}"
          >
            <div class="flex items-center justify-between">
              <div class="flex items-center gap-1.5 min-w-0">
                <span
                  class="text-xs font-bold truncate {s.enabled
                    ? 'text-slate-900 dark:text-slate-100'
                    : 'text-slate-400'}">{s.name}</span
                >
                {#if s.compileError}
                  <div
                    class="w-1.5 h-1.5 rounded-full bg-red-500 shrink-0 animate-pulse"
                    title="Compilation Error"
                  ></div>
                {/if}
              </div>
              <input
                type="checkbox"
                bind:checked={s.enabled}
                onclick={(e) => e.stopPropagation()}
                class="w-3 h-3 rounded border-slate-300 dark:border-slate-600 bg-white dark:bg-[#0d1117] text-indigo-600 focus:ring-indigo-500"
              />
            </div>
            <code class="text-[10px] text-slate-400 truncate font-mono"
              >{s.pattern || ".*"}</code
            >
          </button>
        {/each}
      </div>
    </div>

    <!-- Editor -->
    {#if selectedScript}
      <div class="flex-1 flex flex-col min-w-0">
        <!-- Script Settings Bar -->
        <div
          class="flex items-center gap-4 p-2 bg-slate-50 dark:bg-[#161b22] border-b border-slate-200 dark:border-[#30363d] shrink-0"
        >
          <div class="flex flex-col gap-1">
            <span
              class="text-[9px] font-bold text-slate-500 uppercase tracking-tighter"
              >Name</span
            >
            <input
              bind:value={selectedScript.name}
              class="bg-white dark:bg-[#0d1117] border border-slate-300 dark:border-[#30363d] rounded px-2 py-1 text-xs focus:ring-1 focus:ring-indigo-500 focus:outline-none w-48 text-slate-900 dark:text-slate-100"
            />
          </div>
          <div class="flex flex-col gap-1 flex-1">
            <span
              class="text-[9px] font-bold text-slate-500 uppercase tracking-tighter"
              >URI Match Pattern (Regex)</span
            >
            <input
              bind:value={selectedScript.pattern}
              class="bg-white dark:bg-[#0d1117] border border-slate-300 dark:border-[#30363d] rounded px-2 py-1 text-xs font-mono focus:ring-1 focus:ring-indigo-500 focus:outline-none w-full text-slate-900 dark:text-slate-100"
              placeholder=".*"
            />
          </div>
          <button
            onclick={() => {
              if (selectedScript) selectedScript.code = DEFAULT_SCRIPT_TEMPLATE;
            }}
            class="mt-4 px-2 py-1 text-[10px] bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 border border-indigo-500/20 rounded hover:bg-indigo-500/20 transition-colors font-bold"
          >
            Reset Template
          </button>

          <button
            onclick={() => deleteScript(selectedScript.id)}
            class="mt-4 p-2 text-slate-400 hover:text-red-500 transition-colors"
            title="Delete Script"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="16"
              height="16"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              ><polyline points="3 6 5 6 21 6"></polyline><path
                d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
              ></path><line x1="10" y1="11" x2="10" y2="17"></line><line
                x1="14"
                y1="11"
                x2="14"
                y2="17"
              ></line></svg
            >
          </button>
        </div>

        <!-- Code Area -->
        <textarea
          bind:value={selectedScript.code}
          spellcheck="false"
          class="flex-1 w-full bg-white dark:bg-[#0d1117] p-4 font-mono text-xs leading-relaxed text-slate-800 dark:text-slate-300 focus:outline-none resize-none selection:bg-indigo-500/20"
          placeholder="Write your JS here..."
        ></textarea>
      </div>
    {:else}
      <div
        class="flex-1 flex items-center justify-center text-slate-400 italic text-sm bg-white dark:bg-[#0d1117]"
      >
        Select or create a script to begin.
      </div>
    {/if}
  </div>
</div>
