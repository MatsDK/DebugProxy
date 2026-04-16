<script lang="ts">
  import { ScriptsState } from "$lib/scripts.svelte";
  import { windowState } from "$lib/window.svelte";
  import { onMount } from "svelte";
  import { PaneGroup, Pane, PaneResizer } from "paneforge";
  import CodeEditor from "$lib/components/CodeEditor.svelte";
  import Toggle from "$lib/components/Toggle.svelte";
  import ConfirmationModal from "$lib/components/ConfirmationModal.svelte";
  import { taurpc } from "$lib/rpc";
  import { Trash2, Plus } from "lucide-svelte";

  import type { ScriptConfig } from "$lib/types";
  let { scripts } = $props<{ scripts: ScriptsState }>();

  let selectedId = $state<string | null>(null);
  let selected = $derived(
    scripts.list.find((s: ScriptConfig) => s.id === selectedId) || null,
  );

  let searchTerm = $state("");
  let filteredScripts = $derived(
    scripts.list.filter(
      (s: any) =>
        s.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        s.filters.some((f: any) =>
          f.filterHost.toLowerCase().includes(searchTerm.toLowerCase()),
        ),
    ),
  );

  onMount(() => {
    if (scripts.list.length > 0 && !selectedId) {
      selectedId = scripts.list[0].id;
    }
  });

  let isDark = $state(document.documentElement.classList.contains("dark"));
  let showDocs = $state(false);

  $effect(() => {
    const observer = new MutationObserver(() => {
      isDark = document.documentElement.classList.contains("dark");
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => observer.disconnect();
  });

  function addScript() {
    scripts.addScript();
    selectedId = scripts.list[scripts.list.length - 1].id;
  }

  let scriptToDelete = $state<any | null>(null);

  function removeScript(id: string) {
    const script = (scripts.list as any[]).find((s) => s.id === id);
    if (script) scriptToDelete = script;
  }

  function confirmDelete() {
    if (scriptToDelete) {
      const id = scriptToDelete.id;
      if (selectedId === id) selectedId = null;
      scripts.removeScript(id);
      scriptToDelete = null;
    }
  }

  function cancelDelete() {
    scriptToDelete = null;
  }

  async function popOut() {
    windowState.toggleScripts(true);
    await taurpc.open_detached_window(
      "scripts-editor",
      "Script Editor",
      "/scripts",
    );
  }

  function handlePaste(e: ClipboardEvent, filter: any) {
    const text = e.clipboardData?.getData("text") || "";
    if (!text.startsWith("http") && !text.startsWith("all://")) return;

    try {
      const urlText = text.startsWith("all://")
        ? text.replace("all://", "http://")
        : text;
      const url = new URL(urlText);

      filter.filterProtocol = text.startsWith("all://")
        ? "all"
        : (url.protocol.slice(0, -1) as any);
      filter.filterHost = url.hostname;

      let port = url.port;
      if (
        (filter.filterProtocol === "https" && port === "443") ||
        (filter.filterProtocol === "http" && port === "80")
      ) {
        port = "";
      }
      filter.filterPort = port;

      const pathPart = url.pathname;
      const queryPart = url.search ? url.search.slice(1) : "";

      filter.filterPath = pathPart === "/" ? "/*" : pathPart;
      filter.filterQuery = queryPart;

      if (!text.includes("://")) filter.filterProtocol = "all";

      e.preventDefault();
    } catch (err) {
      // malformed URL
    }
  }

  let selectedFilterId = $state<string | null>(null);
  let activeFilter = $derived(
    selected?.filters.find((f: any) => f.id === selectedFilterId) ||
      selected?.filters[0] ||
      null,
  );

  // When the selected script changes, ensure we select the first filter of that script
  $effect(() => {
    if (selected) {
      if (
        !selectedFilterId ||
        !selected.filters.some((f: any) => f.id === selectedFilterId)
      ) {
        selectedFilterId = selected.filters[0]?.id || null;
      }
    } else {
      selectedFilterId = null;
    }
  });

  function addFilter() {
    if (selected) {
      const newId = Math.random().toString(36).slice(2);
      selected.filters.push({
        id: newId,
        filterProtocol: "all",
        filterHost: "",
        filterPort: "",
        filterPath: "",
        filterQuery: "",
      });
      selectedFilterId = newId;
    }
  }

  function removeFilter(filterId: string) {
    if (selected && selected.filters.length > 1) {
      selected.filters = selected.filters.filter((f: any) => f.id !== filterId);
      if (selectedFilterId === filterId) {
        selectedFilterId = selected.filters[0].id;
      }
    }
  }

  $effect(() => {
    if (selected) {
      selected.pattern = scripts.compileToRegex(selected.filters);
    }
  });
</script>

<div
  class="flex h-full bg-white dark:bg-[#0d1117] font-sans border-t border-slate-200 dark:border-[#30363d] overflow-hidden"
>
  <PaneGroup direction="horizontal" autoSaveId="script-panel-v3">
    <!-- Left Sidebar: Script List -->
    <Pane
      defaultSize={20}
      minSize={10}
      class="flex flex-col border-r border-slate-200 dark:border-[#30363d] bg-[#f8fafc] dark:bg-[#0d1117]"
    >
      <div
        class="h-10 px-3 border-b border-slate-200 dark:border-[#30363d] flex items-center justify-between bg-white dark:bg-[#0d1117] shrink-0"
      >
        <h2
          class="text-xs font-bold text-slate-500 dark:text-slate-400 tracking-widest flex items-center gap-2"
        >
          Scripts
          <Toggle bind:checked={scripts.enabled} size="sm" />
        </h2>
        <div class="flex items-center gap-1">
          <button
            onclick={popOut}
            class="p-1 text-slate-400 hover:text-indigo-500 hover:bg-slate-100 dark:hover:bg-slate-800 rounded transition-colors"
            title="Pop Out"
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              width="14"
              height="14"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
              stroke-linejoin="round"
              ><path d="M15 3h6v6" /><path d="M10 14 21 3" /><path
                d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"
              /></svg
            >
          </button>
          <button
            onclick={addScript}
            class="p-1 text-slate-400 hover:text-indigo-500 hover:bg-slate-100 dark:hover:bg-slate-800 rounded transition-colors"
            title="Add Script"
          >
            <Plus size={16} />
          </button>
        </div>
      </div>

      <div
        class="px-3 py-2 border-b border-slate-200 dark:border-[#30363d] bg-white dark:bg-[#0d1117]"
      >
        <div class="relative">
          <input
            type="text"
            bind:value={searchTerm}
            placeholder="Search scripts..."
            class="w-full px-3 py-1.5 text-xs bg-slate-50 dark:bg-[#161b22] border border-slate-200 dark:border-[#30363d] rounded-md outline-none focus:ring-1 focus:ring-indigo-500/50 transition-all placeholder:text-slate-400"
          />
        </div>
      </div>

      <div class="flex-1 overflow-y-auto custom-scrollbar">
        {#each filteredScripts as script (script.id)}
          <div
            class="w-full text-left p-3 border-b border-slate-100 dark:border-[#21262d] transition-all relative group cursor-pointer {selectedId ===
            script.id
              ? 'bg-white dark:bg-[#161b22]'
              : 'hover:bg-slate-50 dark:hover:bg-[#161b22]/50'}"
            onclick={() => (selectedId = script.id)}
            role="button"
            tabindex="0"
            onkeydown={(e) => e.key === "Enter" && (selectedId = script.id)}
          >
            {#if selectedId === script.id}
              <div
                class="absolute left-0 top-0 bottom-0 w-0.5 bg-indigo-500"
              ></div>
            {/if}
            <div class="flex items-center justify-between mb-1">
              <div class="flex items-center gap-2 truncate min-w-0">
                <span
                  class="text-xs font-bold truncate {selectedId === script.id
                    ? 'text-indigo-600 dark:text-indigo-400'
                    : 'text-slate-600 dark:text-slate-300'}"
                >
                  {script.name}
                </span>
              </div>
              <div class="flex items-center gap-2 shrink-0">
                <button
                  onclick={(e) => {
                    e.stopPropagation();
                    removeScript(script.id);
                  }}
                  class="opacity-0 group-hover:opacity-100 p-1 text-slate-400 hover:text-red-500 dark:hover:text-red-400 transition-all"
                >
                  <Trash2 size={12} />
                </button>
                <div onclick={(e) => e.stopPropagation()} role="presentation">
                  <Toggle bind:checked={script.enabled} size="sm" />
                </div>
              </div>
            </div>
            <div
              class="text-[11px] text-slate-400 dark:text-slate-500 truncate font-mono flex items-center gap-1"
            >
              {script.filters[0]?.filterHost || "*"}
              {#if script.filters.length > 1}
                <span
                  class="text-[9px] px-1 bg-slate-100 dark:bg-slate-800 rounded"
                  >+{script.filters.length - 1}</span
                >
              {/if}
            </div>
          </div>
        {/each}
        {#if filteredScripts.length === 0}
          <div
            class="p-8 text-center text-[11px] text-slate-400 dark:text-slate-600 italic"
          >
            No scripts found
          </div>
        {/if}
      </div>
    </Pane>

    <PaneResizer
      class="w-px bg-slate-200 dark:bg-[#30363d] hover:bg-indigo-500/50 transition-colors shrink-0"
    />

    <!-- Center Pane: Editor -->
    <Pane
      defaultSize={50}
      class="flex flex-col min-w-0 overflow-hidden bg-white dark:bg-[#0d1117]"
    >
      {#if selected}
        <div
          class="h-10 px-4 border-b border-slate-200 dark:border-[#30363d] flex items-center justify-between bg-white dark:bg-[#0d1117] shrink-0"
        >
          <div class="flex-1 flex items-center min-w-0">
            <input
              id="center-script-name"
              bind:value={selected.name}
              class="bg-transparent border-none outline-none text-xs font-bold text-slate-700 dark:text-slate-200 w-full placeholder:text-slate-400 placeholder:font-normal"
              placeholder="Script name..."
            />
          </div>
          <div class="flex items-center gap-3 shrink-0 ml-4">
            <span class="text-xs font-bold text-slate-400 tracking-tight"
              >{selected.enabled ? "Active" : "Paused"}</span
            >
            <Toggle bind:checked={selected.enabled} size="sm" />
          </div>
        </div>

        {#if selected.compileError}
          <div
            class="bg-red-50 dark:bg-red-950/30 border-b border-red-200 dark:border-red-500/30 px-3 py-2 shrink-0"
          >
            <div class="flex items-center gap-2 text-red-600 dark:text-red-400">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
                ><circle cx="12" cy="12" r="10"></circle><line
                  x1="12"
                  y1="8"
                  x2="12"
                  y2="12"
                ></line><line x1="12" y1="16" x2="12.01" y2="16"></line></svg
              >
              <span class="text-[10px] font-bold uppercase tracking-tight"
                >Compilation Error</span
              >
            </div>
            <pre
              class="mt-1 text-[10px] font-mono text-red-800 dark:text-red-300/80 whitespace-pre-wrap leading-relaxed">{selected.compileError}</pre>
          </div>
        {/if}

        <div class="flex-1 relative overflow-hidden">
          <CodeEditor
            value={selected.code}
            onchange={(val) => (selected.code = val)}
            darkMode={isDark}
          />
        </div>
      {:else}
        <div
          class="flex-1 flex flex-col items-center justify-center text-slate-300 dark:text-slate-800 space-y-4"
        >
          <div
            class="p-6 bg-slate-50 dark:bg-[#161b22] rounded-full border border-dashed border-slate-200 dark:border-[#30363d]"
          >
            <span class="text-xs uppercase font-bold tracking-widest opacity-20"
              >No Selection</span
            >
          </div>
          <p class="text-sm italic font-medium">
            Select a script to start debugging
          </p>
        </div>
      {/if}
    </Pane>

    <PaneResizer
      class="w-px bg-slate-200 dark:bg-[#30363d] hover:bg-indigo-500/50 transition-colors shrink-0"
    />

    <!-- Right Sidebar: Settings -->
    <Pane
      defaultSize={30}
      minSize={20}
      class="flex flex-col bg-[#f8fafc] dark:bg-[#0d1117] border-l border-slate-200 dark:border-[#30363d] overflow-hidden"
    >
      <div
        class="h-10 px-3 flex items-center border-b border-slate-200 dark:border-[#30363d] bg-white dark:bg-[#161b22] shrink-0"
      >
        <span
          class="text-xs font-bold text-slate-500 dark:text-slate-400 uppercase tracking-widest"
        >
          Settings
        </span>
      </div>

      {#if selected}
        <div class="flex-1 p-4 space-y-6 overflow-y-auto custom-scrollbar">
          <!-- Filters Section -->
          <div class="space-y-4">
            <div class="flex items-center justify-between">
              <span
                class="text-[11px] font-bold text-slate-400 dark:text-slate-500 uppercase tracking-wider"
              >
                Patterns ({selected.filters.length})
              </span>
              <button
                onclick={addFilter}
                class="text-xs flex items-center gap-1 px-2 py-1 bg-indigo-50 dark:bg-indigo-900/30 text-indigo-600 dark:text-indigo-400 rounded hover:bg-indigo-100 dark:hover:bg-indigo-800/40 border border-indigo-200 dark:border-indigo-800/50 transition-colors font-bold"
              >
                <Plus size={12} /> Add
              </button>
            </div>

            <!-- Pattern Selection List -->
            <div
              class="space-y-1.5 max-h-48 overflow-y-auto custom-scrollbar p-0.5"
            >
              {#each selected.filters as filter (filter.id)}
                <div class="flex items-center gap-1 group/item">
                  <button
                    onclick={() => (selectedFilterId = filter.id)}
                    class="flex-1 text-left px-2.5 py-2 rounded border transition-all text-[11px] font-mono truncate {selectedFilterId ===
                    filter.id
                      ? 'bg-indigo-50 dark:bg-indigo-900/30 border-indigo-200 dark:border-indigo-800 text-indigo-700 dark:text-indigo-300'
                      : 'bg-white dark:bg-[#161b22] border-slate-200 dark:border-[#30363d] text-slate-500 hover:bg-slate-50 dark:hover:bg-[#1f262d]'}"
                  >
                    <span class="opacity-50"
                      >[{filter.filterProtocol.toUpperCase()}]</span
                    >
                    {filter.filterHost || "*"}{filter.filterPath || "/*"}
                  </button>
                  {#if selected.filters.length > 1}
                    <button
                      onclick={() => removeFilter(filter.id)}
                      class="p-1.5 text-slate-400 hover:text-red-500 dark:hover:text-red-400 opacity-0 group-hover/item:opacity-100 transition-all shrink-0"
                      title="Remove Pattern"
                    >
                      <Trash2 size={12} />
                    </button>
                  {/if}
                </div>
              {/each}
            </div>

            <!-- Focused Pattern Editor -->
            {#if activeFilter}
              <div
                class="p-4 bg-white dark:bg-[#161b22] border border-slate-200 dark:border-[#30363d] rounded-lg shadow-sm space-y-4 animate-in fade-in slide-in-from-top-1 duration-200"
              >
                <div class="flex items-center gap-2 mb-1">
                  <span
                    class="text-[11px] font-bold uppercase tracking-widest text-slate-500"
                    >Edit Selected Pattern</span
                  >
                </div>

                <div class="grid grid-cols-12 gap-x-2 gap-y-3">
                  <div class="col-span-4">
                    <label
                      for="f-proto-{activeFilter.id}"
                      class="block text-[9px] text-slate-400 dark:text-slate-500 mb-1 font-bold uppercase cursor-pointer"
                      >Protocol</label
                    >
                    <select
                      id="f-proto-{activeFilter.id}"
                      bind:value={activeFilter.filterProtocol}
                      class="w-full h-8 px-1.5 text-xs bg-slate-50 dark:bg-[#0d1117] border border-slate-200 dark:border-[#30363d] rounded text-slate-700 dark:text-slate-300 outline-none focus:ring-1 focus:ring-indigo-500 transition-all color-scheme-dark cursor-pointer font-bold"
                    >
                      <option value="all">Any</option>
                      <option value="https">HTTPS</option>
                      <option value="http">HTTP</option>
                    </select>
                  </div>

                  <div class="col-span-8">
                    <label
                      for="f-host-{activeFilter.id}"
                      class="block text-[9px] text-slate-400 dark:text-slate-500 mb-1 font-bold uppercase cursor-pointer"
                      >Host / Domain</label
                    >
                    <input
                      id="f-host-{activeFilter.id}"
                      type="text"
                      bind:value={activeFilter.filterHost}
                      onpaste={(e) => handlePaste(e, activeFilter)}
                      placeholder="*.google.com"
                      class="w-full h-8 px-2 text-xs bg-slate-50 dark:bg-[#0d1117] border border-slate-200 dark:border-[#30363d] rounded text-indigo-600 dark:text-indigo-400 font-mono focus:ring-1 focus:ring-indigo-500 outline-none transition-all placeholder:italic"
                    />
                  </div>

                  <div class="col-span-12 grid grid-cols-3 gap-2">
                    <div class="col-span-1">
                      <label
                        for="f-port-{activeFilter.id}"
                        class="block text-[9px] text-slate-400 dark:text-slate-500 mb-1 font-bold uppercase cursor-pointer"
                        >Port</label
                      >
                      <input
                        id="f-port-{activeFilter.id}"
                        type="text"
                        bind:value={activeFilter.filterPort}
                        placeholder="*"
                        class="w-full h-8 px-2 text-xs bg-slate-50 dark:bg-[#0d1117] border border-slate-200 dark:border-[#30363d] rounded text-slate-700 dark:text-slate-300 font-mono focus:ring-1 focus:ring-indigo-500 outline-none transition-all"
                      />
                    </div>
                    <div class="col-span-2">
                      <label
                        for="f-path-{activeFilter.id}"
                        class="block text-[9px] text-slate-400 dark:text-slate-500 mb-1 font-bold uppercase cursor-pointer"
                        >Path</label
                      >
                      <input
                        id="f-path-{activeFilter.id}"
                        type="text"
                        bind:value={activeFilter.filterPath}
                        placeholder="/*"
                        class="w-full h-8 px-2 text-xs bg-slate-50 dark:bg-[#0d1117] border border-slate-200 dark:border-[#30363d] rounded text-indigo-600 dark:text-indigo-400 font-mono focus:ring-1 focus:ring-indigo-500 outline-none transition-all"
                      />
                    </div>
                  </div>

                  <div class="col-span-12">
                    <label
                      for="f-query-{activeFilter.id}"
                      class="block text-[9px] text-slate-400 dark:text-slate-500 mb-1 font-bold uppercase cursor-pointer"
                      >Query Params</label
                    >
                    <div class="relative flex items-center">
                      <span
                        class="absolute left-2 text-[9px] font-mono text-slate-400"
                        >?</span
                      >
                      <input
                        id="f-query-{activeFilter.id}"
                        type="text"
                        bind:value={activeFilter.filterQuery}
                        placeholder="*"
                        class="w-full h-8 pl-4 pr-2 text-xs bg-slate-50 dark:bg-[#0d1117] border border-slate-200 dark:border-[#30363d] rounded text-indigo-600 dark:text-indigo-400 font-mono focus:ring-1 focus:ring-indigo-500 outline-none transition-all"
                      />
                    </div>
                  </div>
                </div>
                <p
                  class="text-[8px] text-slate-400 italic px-0.5 leading-tight opacity-60"
                >
                  Paste full URL above to auto-split.
                </p>
              </div>
            {/if}
          </div>

          <!-- Notes Field -->
          <div class="space-y-1.5">
            <label
              for="script-notes"
              class="text-[11px] font-bold text-slate-400 dark:text-slate-500 uppercase tracking-wider cursor-pointer"
            >
              Notes
            </label>
            <textarea
              id="script-notes"
              bind:value={selected.description}
              rows="3"
              class="w-full bg-white dark:bg-[#161b22] border border-slate-200 dark:border-[#30363d] rounded px-3 py-2 text-xs text-slate-600 dark:text-slate-300 focus:ring-1 focus:ring-indigo-500 outline-none transition-all shadow-sm leading-relaxed"
              placeholder="Surgical field notes..."
            ></textarea>
          </div>

          <!-- Compiled Pattern Preview -->
          <div class="space-y-1.5">
            <span
              class="text-[11px] font-bold text-slate-400 dark:text-slate-500 uppercase tracking-wider"
            >
              Compiled Matching Regex
            </span>
            <div
              class="p-2 bg-slate-100 dark:bg-[#0d1117] border border-slate-200 dark:border-[#30363d] rounded font-mono text-[9px] text-slate-500 dark:text-slate-400 break-all leading-tight"
            >
              {selected.pattern}
            </div>
          </div>

          <!-- API Documentation -->
          <div class="space-y-1.5">
            <button
              onclick={() => (showDocs = !showDocs)}
              class="w-full flex items-center justify-between text-[11px] font-bold text-indigo-500 dark:text-indigo-400 uppercase tracking-wider hover:text-indigo-600 dark:hover:text-indigo-300 transition-colors"
            >
              <span>API Reference</span>
              <svg
                xmlns="http://www.w3.org/2000/svg"
                width="12"
                height="12"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
                class="transition-transform {showDocs ? 'rotate-180' : ''}"
                ><path d="m6 9 6 6 6-6" /></svg
              >
            </button>

            {#if showDocs}
              <div
                class="space-y-3 animate-in fade-in slide-in-from-top-1 duration-200"
              >
                <!-- Handlers -->
                <div
                  class="p-3 bg-white dark:bg-[#161b22] border border-slate-200 dark:border-[#30363d] rounded-lg"
                >
                  <div
                    class="text-[10px] font-black text-slate-500 dark:text-slate-400 uppercase tracking-widest mb-2"
                  >
                    Handlers
                  </div>
                  <pre
                    class="text-[10px] font-mono text-slate-600 dark:text-slate-300 whitespace-pre leading-relaxed">onRequest(req, proxy)
onResponse(res, proxy)</pre>
                </div>

                <!-- Request/Response Object -->
                <div
                  class="p-3 bg-white dark:bg-[#161b22] border border-slate-200 dark:border-[#30363d] rounded-lg"
                >
                  <div
                    class="text-[10px] font-black text-slate-500 dark:text-slate-400 uppercase tracking-widest mb-2"
                  >
                    req / res Object
                  </div>
                  <div
                    class="space-y-1.5 text-[10px] font-mono text-slate-600 dark:text-slate-300"
                  >
                    <div>
                      <span class="text-indigo-500">.url</span>
                      <span class="text-slate-400">→ URL object</span>
                    </div>
                    <pre
                      class="text-slate-400 pl-2 leading-relaxed">.hostname .pathname .searchParams</pre>
                    <div>
                      <span class="text-indigo-500">.headers</span><span
                        class="text-emerald-500">[key]</span
                      > <span class="text-slate-400">→ get/set/delete</span>
                    </div>
                    <div>
                      <span class="text-indigo-500">.json</span>
                      <span class="text-slate-400"
                        >→ deep proxy (auto-saves)</span
                      >
                    </div>
                    <div>
                      <span class="text-indigo-500">.body</span>
                      <span class="text-slate-400">→ string get/set</span>
                    </div>
                    <div>
                      <span class="text-indigo-500">.raw</span>
                      <span class="text-slate-400">→ Uint8Array get/set</span>
                    </div>
                    <div>
                      <span class="text-indigo-500">.formData</span>
                      <span class="text-slate-400">→ URLSearchParams</span>
                    </div>
                    <div>
                      <span class="text-indigo-500">.contentType</span>
                      <span class="text-slate-400">→ string | null</span>
                    </div>
                    <div>
                      <span class="text-indigo-500">.status</span>
                      <span class="text-slate-400">→ number (responses)</span>
                    </div>
                    <div>
                      <span class="text-indigo-500">.method</span>
                      <span class="text-slate-400">→ string (read-only)</span>
                    </div>
                    <div>
                      <span class="text-indigo-500">.uri</span>
                      <span class="text-slate-400">→ full URL string</span>
                    </div>
                  </div>
                </div>

                <!-- Proxy Object -->
                <div
                  class="p-3 bg-white dark:bg-[#161b22] border border-slate-200 dark:border-[#30363d] rounded-lg"
                >
                  <div
                    class="text-[10px] font-black text-slate-500 dark:text-slate-400 uppercase tracking-widest mb-2"
                  >
                    proxy Object
                  </div>
                  <div
                    class="space-y-2 text-[10px] font-mono text-slate-600 dark:text-slate-300"
                  >
                    <div>
                      <div>
                        <span class="text-indigo-500">proxy.log</span>(msg,
                        level?)
                      </div>
                      <div class="text-slate-400 pl-2">
                        level: "info" | "warn" | "error"
                      </div>
                    </div>
                    <div>
                      <div>
                        <span class="text-emerald-500">proxy.mock</span>(status,
                        body?, headers?)
                      </div>
                      <div class="text-slate-400 pl-2">
                        Return fake response, skip server
                      </div>
                    </div>
                    <div>
                      <div>
                        <span class="text-amber-500">proxy.delay</span>(ms)
                      </div>
                      <div class="text-slate-400 pl-2">
                        Real latency (holds connection)
                      </div>
                    </div>
                    <div>
                      <div><span class="text-red-500">proxy.drop</span>()</div>
                      <div class="text-slate-400 pl-2">
                        Block request (returns 403)
                      </div>
                    </div>
                    <div>
                      <div>
                        <span class="text-indigo-500">proxy.store</span
                        >.get/set/delete/has/clear
                      </div>
                      <div class="text-slate-400 pl-2">
                        Persistent KV across requests
                      </div>
                    </div>
                  </div>
                </div>

                <!-- Examples -->
                <div
                  class="p-3 bg-white dark:bg-[#161b22] border border-slate-200 dark:border-[#30363d] rounded-lg"
                >
                  <div
                    class="text-[10px] font-black text-slate-500 dark:text-slate-400 uppercase tracking-widest mb-2"
                  >
                    Quick Examples
                  </div>
                  <pre
                    class="text-[10px] font-mono text-slate-600 dark:text-slate-300 whitespace-pre leading-relaxed overflow-x-auto">// Modify JSON body
if (req.json) req.json.debug = true;

// Add query param
req.url.searchParams.set("v", "2");

// Mock a 200 response
proxy.mock(200, {"{"}ok: true{"}"});

// Capture token across requests
proxy.store.set("t", res.json?.token);
req.headers["Auth"] =
  proxy.store.get("t");

// Simulate slow network
await proxy.delay(2000);

// Block analytics
if (req.url.hostname.includes(
  "analytics")) proxy.drop();</pre>
                </div>

                <!-- Body Channel Warning -->
                <div
                  class="p-2.5 bg-amber-50 dark:bg-amber-950/20 border border-amber-200 dark:border-amber-800/40 rounded-lg"
                >
                  <div
                    class="text-[9px] font-bold text-amber-600 dark:text-amber-400 uppercase tracking-wider mb-1"
                  >
                    ⚠ Body Channels
                  </div>
                  <p
                    class="text-[9px] text-amber-700 dark:text-amber-300/80 leading-relaxed"
                  >
                    <code
                      class="bg-amber-100 dark:bg-amber-900/30 px-0.5 rounded"
                      >json</code
                    >,
                    <code
                      class="bg-amber-100 dark:bg-amber-900/30 px-0.5 rounded"
                      >body</code
                    >,
                    <code
                      class="bg-amber-100 dark:bg-amber-900/30 px-0.5 rounded"
                      >raw</code
                    >, and
                    <code
                      class="bg-amber-100 dark:bg-amber-900/30 px-0.5 rounded"
                      >formData</code
                    > share the same backing store. If you switch between them in
                    one handler, the last write wins. Stick to one per handler.
                  </p>
                </div>
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <div
          class="flex-1 flex flex-col items-center justify-center p-8 space-y-4 opacity-30 select-none grayscale"
        >
          <span class="text-[10px] font-bold uppercase tracking-tight"
            >No Selection</span
          >
        </div>
      {/if}
    </Pane>
  </PaneGroup>
</div>

{#if scriptToDelete}
  <ConfirmationModal
    title="Delete Script?"
    message="Are you sure you want to delete '{scriptToDelete.name}'? All filters and code will be permanently removed."
    onConfirm={confirmDelete}
    onCancel={cancelDelete}
  />
{/if}

<style>
  .custom-scrollbar::-webkit-scrollbar {
    width: 4px;
  }
  .custom-scrollbar::-webkit-scrollbar-track {
    background: transparent;
  }
  .custom-scrollbar::-webkit-scrollbar-thumb {
    background: #cbd5e1;
    border-radius: 10px;
  }
</style>
