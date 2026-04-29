<script lang="ts">
  import type { ProxyEvent, ScriptLog } from "$lib/types";
  import JsonViewer from "./JsonViewer.svelte";
  import { PaneGroup, Pane, PaneResizer } from "paneforge";
  import { taurpc } from "$lib/rpc";
  import { windowState } from "$lib/window.svelte";
  import { XCircle, Sparkles, Play, Trash2 } from "lucide-svelte";
  import CodeEditor from "./CodeEditor.svelte";
  import * as prettier from "prettier/standalone";
  import * as babel from "prettier/plugins/babel";
  import * as estree from "prettier/plugins/estree";
  import { toast } from "$lib/toast.svelte";

  let {
    req,
    res: resProp,
    logs,
    editable = false,
    showPopout = true,
    onAbort,
    onExecute,
  }: {
    req: ProxyEvent;
    res: ProxyEvent | null;
    logs: ScriptLog[];
    editable?: boolean;
    showPopout?: boolean;
    onAbort?: () => void;
    onExecute?: () => void;
  } = $props();

  let res = $derived(resProp || (req.is_response ? req : null));
  let isResponseOnly = $derived(req.is_response && (!resProp || resProp === req));

  let reqTab = $state<"headers" | "body">("headers");
  let resTab = $state<"headers" | "body">("headers");

  $effect(() => {
    if (req && req.body && !req.is_response) {
      reqTab = "body";
    } else {
      reqTab = "headers";
    }
  });

  $effect(() => {
    if (res && res.body) {
      resTab = "body";
    } else {
      resTab = "headers";
    }
  });

  let scriptLogs = $derived(logs.filter((l) => l.requestId === req.id));

  function statusColor(status: number | null) {
    if (!status) return "text-slate-400";
    if (status < 300) return "text-emerald-500";
    if (status < 400) return "text-amber-500";
    return "text-red-500";
  }

  function methodColor(m: string) {
    return (
      {
        GET: "text-blue-500",
        POST: "text-emerald-500",
        PUT: "text-amber-500",
        DELETE: "text-red-500",
      }[m.toUpperCase()] ?? "text-slate-400"
    );
  }

  async function popOut() {
    windowState.toggleInspector(req.id, true);
    await taurpc.open_detached_window(
      `inspector-${req.id}`,
      `Inspector - ${req.method} ${req.uri}`,
      `/inspector?id=${req.id}`,
    );
  }

  function formatBodyStr(data: Uint8Array | number[] | string | null): any {
    if (!data) return null;
    if (typeof data === "string") return data;
    const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
    if (bytes.length === 0) return null;

    try {
      // Direct decode, then attempt JSON parse
      const str = new TextDecoder().decode(bytes);
      try {
        const trimmed = str.trim();
        if (trimmed.startsWith("{") || trimmed.startsWith("[")) {
          return JSON.parse(str);
        }
      } catch {}
      return str;
    } catch {
      return "[Binary Data]";
    }
  }

  let reqBody = $derived(!req.is_response ? formatBodyStr(req.body) : null);
  let resBody = $derived(res ? formatBodyStr(res.body) : null);

  // Container width observer
  let containerWidth = $state(0);
  let isNarrow = $derived(containerWidth > 0 && containerWidth < 1400);

  // Theme detection for CodeEditor
  let isDark = $state(typeof document !== 'undefined' && document.documentElement.classList.contains("dark"));
  $effect(() => {
    if (typeof document === 'undefined') return;
    const observer = new MutationObserver(() => {
      isDark = document.documentElement.classList.contains("dark");
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => observer.disconnect();
  });

  // Editable body text helper
  let editableBody = $state("");
  let editableResBody = $state("");

  $effect(() => {
    if (editable && req && !req.is_response) {
      if ((req as any).text !== undefined) {
        let val = (req as any).text;
        try {
          const obj = JSON.parse(val);
          editableBody = JSON.stringify(obj, null, 2);
        } catch {
          editableBody = val;
        }
      } else if (req.body) {
        if (typeof req.body === "string") {
          try {
            const obj = JSON.parse(req.body);
            editableBody = JSON.stringify(obj, null, 2);
          } catch {
            editableBody = req.body;
          }
        } else {
          const bytes =
            req.body instanceof Uint8Array
              ? req.body
              : new Uint8Array(req.body as any as number[]);
          try {
            const str = new TextDecoder().decode(bytes);
            try {
              const obj = JSON.parse(str);
              editableBody = JSON.stringify(obj, null, 2);
            } catch {
              editableBody = str;
            }
          } catch {
            editableBody = "[Binary]";
          }
        }
      }
    }
  });

  $effect(() => {
    if (editable && res) {
      if ((res as any).text !== undefined) {
        let val = (res as any).text;
        try {
          const obj = JSON.parse(val);
          editableResBody = JSON.stringify(obj, null, 2);
        } catch {
          editableResBody = val;
        }
      } else if (res.body) {
        if (typeof res.body === "string") {
          try {
            const obj = JSON.parse(res.body);
            editableResBody = JSON.stringify(obj, null, 2);
          } catch {
            editableResBody = res.body;
          }
        } else {
          const bytes =
            res.body instanceof Uint8Array
              ? res.body
              : new Uint8Array(res.body as any as number[]);
          try {
            const str = new TextDecoder().decode(bytes);
            try {
              const obj = JSON.parse(str);
              editableResBody = JSON.stringify(obj, null, 2);
            } catch {
              editableResBody = str;
            }
          } catch {
            editableResBody = "[Binary]";
          }
        }
      }
    }
  });

function onBodyChange(val: string) {
  editableBody = val;
  if ((req as any).text !== undefined) {
    (req as any).text = val;
  } else {
    // Sync back to body as bytes
    req.body = Array.from(new TextEncoder().encode(val)) as any;
  }
}

function onResBodyChange(val: string) {
  editableResBody = val;
  if (res) {
    if ((res as any).text !== undefined) {
      (res as any).text = val;
    } else {
      // Sync back to body as bytes
      res.body = Array.from(new TextEncoder().encode(val)) as any;
    }
  }
}

async function formatEditableBody() {
  try {
    const isJson =
      editableBody.trim().startsWith("{") ||
      editableBody.trim().startsWith("[");
    const formatted = await prettier.format(editableBody, {
      parser: isJson ? "json" : "babel",
      plugins: [babel, estree],
      semi: true,
      singleQuote: false,
      tabWidth: 2,
    });
    onBodyChange(formatted);
  } catch (err: any) {
    console.error("Format error:", err);
    toast.error("Format failed: " + err.message);
  }
}

async function formatEditableResBody() {
  try {
    const isJson =
      editableResBody.trim().startsWith("{") ||
      editableResBody.trim().startsWith("[");
    const formatted = await prettier.format(editableResBody, {
      parser: isJson ? "json" : "babel",
      plugins: [babel, estree],
      semi: true,
      singleQuote: false,
      tabWidth: 2,
    });
    onResBodyChange(formatted);
  } catch (err: any) {
    console.error("Format error:", err);
    toast.error("Format failed: " + err.message);
  }
}


  // Unified headers access
  let headers = $derived.by(() => {
    if (req.is_response) return [];
    if (Array.isArray(req.headers)) return req.headers;
    if ((req as any).rawHeaders) return (req as any).rawHeaders;
    return Object.entries(req.headers || {});
  });

  let resHeaders = $derived.by(() => {
    if (!res) return [];
    if (Array.isArray(res.headers)) return res.headers;
    if ((res as any).rawHeaders) return (res as any).rawHeaders;
    return Object.entries(res.headers || {});
  });

  function updateHeader(index: number, key: string, value: string) {
    const h = (req as any).rawHeaders || req.headers;
    if (Array.isArray(h)) {
      h[index] = [key, value];
    } else {
      // It's the headersProxy (Object)
      const oldKey = Object.keys(h)[index];
      if (oldKey && oldKey !== key) {
        delete h[oldKey];
      }
      h[key] = value;
    }
  }

  function updateResHeader(index: number, key: string, value: string) {
    if (!res) return;
    const h = (res as any).rawHeaders || res.headers;
    if (Array.isArray(h)) {
      h[index] = [key, value];
    } else {
      const oldKey = Object.keys(h)[index];
      if (oldKey && oldKey !== key) {
        delete h[oldKey];
      }
      h[key] = value;
    }
  }

  function addHeader() {
    const h = (req as any).rawHeaders || req.headers;
    if (Array.isArray(h)) {
      h.push(["New-Header", "value"]);
    } else {
      h["New-Header"] = "value";
    }
  }

  function addResHeader() {
    if (!res) return;
    const h = (res as any).rawHeaders || res.headers;
    if (Array.isArray(h)) {
      h.push(["New-Header", "value"]);
    } else {
      h["New-Header"] = "value";
    }
  }

  function removeHeader(index: number) {
    const h = (req as any).rawHeaders || req.headers;
    if (Array.isArray(h)) {
      h.splice(index, 1);
    } else {
      const key = Object.keys(h)[index];
      if (key) delete h[key];
    }
  }

  function removeResHeader(index: number) {
    if (!res) return;
    const h = (res as any).rawHeaders || res.headers;
    if (Array.isArray(h)) {
      h.splice(index, 1);
    } else {
      const key = Object.keys(h)[index];
      if (key) delete h[key];
    }
  }
</script>

<div
  class="flex flex-col flex-1 min-h-0 bg-white dark:bg-[#0d1117] font-sans"
  bind:clientWidth={containerWidth}
>
  <!-- Details header -->
  <div
    class="h-9 border-y border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0 flex items-center px-3 gap-3"
  >
    <span
      class="font-mono font-bold text-xs {methodColor(
        req.method,
      )} uppercase tracking-tighter shrink-0">{req.method}</span
    >
    {#if editable}
      <div class="flex-1 flex items-center gap-2">
        <input
          bind:value={req.uri}
          class="flex-1 bg-slate-100 dark:bg-black/20 border border-slate-200 dark:border-white/10 px-2 py-0.5 rounded text-xs font-mono focus:outline-none focus:ring-1 focus:ring-indigo-500"
        />
        <button
          onclick={resTab === "body" && res ? formatEditableResBody : formatEditableBody}
          class="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-bold bg-indigo-50 dark:bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 rounded border border-indigo-200/50 dark:border-indigo-500/20 hover:bg-indigo-100 dark:hover:bg-indigo-500/20 transition-all active:scale-95 shrink-0"
        >
          <Sparkles size={12} />
          Format
        </button>
      </div>
    {:else}
      <span
        class="font-mono text-xs text-slate-500 dark:text-slate-400 truncate select-all"
        >{req.uri}</span
      >
    {/if}
    <div class="ml-auto flex items-center">
      {#if showPopout}
        <button
          onclick={popOut}
          class="p-1 hover:bg-slate-200 dark:hover:bg-white/10 rounded transition-colors"
          title="Pop out into new window"
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
            class="text-slate-500 dark:text-slate-400"
            ><path d="M15 3h6v6" /><path d="M10 14 21 3" /><path
              d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"
            /></svg
          >
        </button>
      {/if}
    </div>
  </div>

  <PaneGroup
    direction={isNarrow ? "vertical" : "horizontal"}
    class="flex flex-1 min-h-0 overflow-hidden"
  >
    {#if isNarrow}
      <Pane defaultSize={50} minSize={20} class="flex min-h-0 overflow-hidden">
        <PaneGroup
          direction="horizontal"
          class="flex flex-1 min-h-0 overflow-hidden"
        >
          <Pane
            defaultSize={50}
            minSize={20}
            class="flex flex-col border-r border-slate-200 dark:border-[#30363d] overflow-hidden"
          >
            <div
              class="h-8 bg-slate-100/50 dark:bg-[#1c2128] border-b border-slate-200 dark:border-[#30363d] flex items-center px-2 shrink-0"
            >
              <span
                class="text-xs font-black text-slate-500 uppercase tracking-widest"
                >Request</span
              >
              {#if !isResponseOnly}
                <div class="flex h-full gap-1 ml-4">
                  <button
                    class="px-2 h-full text-xs font-bold transition-all {reqTab ===
                    'headers'
                      ? 'text-indigo-600 dark:text-indigo-400 border-b-2 border-indigo-500'
                      : 'text-slate-400'}"
                    onclick={() => (reqTab = "headers")}
                  >
                    <span class="translate-y-[0.5px]">Headers</span>
                  </button>
                  <button
                    class="px-2 h-full text-xs font-bold transition-all {reqTab ===
                    'body'
                      ? 'text-indigo-600 dark:text-indigo-400 border-b-2 border-indigo-500'
                      : 'text-slate-400'}"
                    onclick={() => (reqTab = "body")}
                  >
                    <span class="translate-y-[0.5px]">Body</span>
                  </button>
                </div>
              {/if}
            </div>
            <div class="flex-1 overflow-y-auto">
              {#if isResponseOnly}
                <div class="p-4 flex flex-col items-center justify-center text-slate-400 italic text-[11px] text-center gap-2">
                  <p>Original request details are not available in this window.</p>
                  <p class="text-[9px] opacity-60">This can happen if the interceptor window was opened after the request was sent.</p>
                </div>
              {:else if reqTab === "headers"}
                <div class="p-2 space-y-0.5">
                  {#each headers as [k, v], i}
                    <div
                      class="flex text-xs font-mono border-b border-black/5 dark:border-white/5 py-0.5 gap-2 items-center"
                    >
                      {#if editable}
                        <input
                          value={k}
                          oninput={(e) =>
                            updateHeader(i, e.currentTarget.value, v)}
                          class="w-32 shrink-0 bg-transparent border-none p-0 text-indigo-600 dark:text-indigo-400 font-bold focus:ring-0"
                        />
                        <input
                          value={v}
                          oninput={(e) =>
                            updateHeader(i, k, e.currentTarget.value)}
                          class="flex-1 bg-transparent border-none p-0 text-slate-600 dark:text-slate-300 focus:ring-0"
                        />
                        <button
                          onclick={() => removeHeader(i)}
                          class="p-0.5 text-slate-400 hover:text-red-500"
                        >
                          <XCircle size={12} />
                        </button>
                      {:else}
                        <span
                          class="w-32 shrink-0 text-indigo-600 dark:text-indigo-400 font-bold truncate"
                          >{k}</span
                        >
                        <span
                          class="flex-1 text-slate-600 dark:text-slate-300 break-all"
                          >{v}</span
                        >
                      {/if}
                    </div>
                  {/each}
                  {#if editable}
                    <button
                      onclick={addHeader}
                      class="mt-2 text-[10px] font-bold text-indigo-600 dark:text-indigo-400 hover:underline"
                    >
                      + Add Header
                    </button>
                  {/if}
                </div>
              {:else if reqBody === null}
                <div class="text-slate-400 italic text-[11px] p-2">No body.</div>
              {:else if typeof reqBody === "object" && !editable}
                <div class="text-xs">
                  <JsonViewer data={reqBody} />
                </div>
              {:else if editable}
                <div class="h-full min-h-[300px]">
                   <CodeEditor 
                    value={editableBody} 
                    onchange={onBodyChange} 
                    darkMode={isDark} 
                   />
                </div>
              {:else}
                <pre
                  class="text-[11px] font-mono whitespace-pre-wrap break-all px-1">{reqBody}</pre>
              {/if}
            </div>
          </Pane>
          <PaneResizer
            class="w-1 bg-transparent hover:bg-indigo-500/30 cursor-col-resize shrink-0 transition-colors"
          />
          <Pane
            defaultSize={50}
            minSize={20}
            class="flex flex-col border-r border-slate-200 dark:border-[#30363d] overflow-hidden"
          >
            <div
              class="h-8 bg-slate-100/50 dark:bg-[#1c2128] border-b border-slate-200 dark:border-[#30363d] flex items-center justify-between px-2 shrink-0"
            >
              <div class="flex items-center h-full">
                <span
                  class="text-xs font-black text-slate-500 uppercase tracking-widest"
                  >Response</span
                >
                <div class="flex h-full gap-1 ml-4">
                  <button
                    class="px-2 h-full text-xs font-bold transition-all {resTab ===
                    'headers'
                      ? 'text-indigo-600 dark:text-indigo-400 border-b-2 border-indigo-500'
                      : 'text-slate-400'}"
                    onclick={() => (resTab = "headers")}
                  >
                    <span>Headers</span>
                  </button>
                  <button
                    class="px-2 h-full text-xs font-bold transition-all {resTab ===
                    'body'
                      ? 'text-indigo-600 dark:text-indigo-400 border-b-2 border-indigo-500'
                      : 'text-slate-400'}"
                    onclick={() => (resTab = "body")}
                  >
                    <span>Body</span>
                  </button>
                </div>
              </div>
              {#if editable && res}
                <input
                  type="number"
                  bind:value={res.status}
                  class="w-12 bg-slate-100 dark:bg-black/20 border border-slate-200 dark:border-white/10 px-1 py-0.5 rounded text-xs font-bold text-center focus:outline-none focus:ring-1 focus:ring-indigo-500"
                />
              {:else if res}
                <span class="text-xs font-bold {statusColor(res.status)}"
                  >{res.status}</span
                >
              {/if}
            </div>
            <div class="flex-1 overflow-y-auto">
              {#if res}
                {#if resTab === "headers"}
                  <div class="p-2 space-y-0.5">
                    {#each resHeaders as [k, v], i}
                      <div
                        class="flex text-xs font-mono border-b border-black/5 dark:border-white/5 py-0.5 gap-2 items-center"
                      >
                        {#if editable}
                          <input
                            value={k}
                            oninput={(e) =>
                              updateResHeader(i, e.currentTarget.value, v)}
                            class="w-32 shrink-0 bg-transparent border-none p-0 text-indigo-600 dark:text-indigo-400 font-bold focus:ring-0"
                          />
                          <input
                            value={v}
                            oninput={(e) =>
                              updateResHeader(i, k, e.currentTarget.value)}
                            class="flex-1 bg-transparent border-none p-0 text-slate-600 dark:text-slate-300 focus:ring-0"
                          />
                          <button
                            onclick={() => removeResHeader(i)}
                            class="p-0.5 text-slate-400 hover:text-red-500"
                          >
                            <XCircle size={12} />
                          </button>
                        {:else}
                          <span
                            class="w-32 shrink-0 text-indigo-600 dark:text-indigo-400 font-bold truncate"
                            >{k}</span
                          >
                          <span
                            class="flex-1 text-slate-600 dark:text-slate-300 break-all"
                            >{v}</span
                          >
                        {/if}
                      </div>
                    {/each}
                    {#if editable}
                      <button
                        onclick={addResHeader}
                        class="mt-2 text-[10px] font-bold text-indigo-600 dark:text-indigo-400 hover:underline"
                      >
                        + Add Header
                      </button>
                    {/if}
                  </div>
                {:else if resBody === null}
                  <div class="text-slate-400 italic text-[11px] p-2">No body.</div>
                {:else if typeof resBody === "object" && !editable}
                  <div class="text-xs">
                    <JsonViewer data={resBody} />
                  </div>
                {:else if editable}
                  <div class="h-full min-h-[300px]">
                    <CodeEditor
                      value={editableResBody}
                      onchange={onResBodyChange}
                      darkMode={isDark}
                    />
                  </div>
                {:else}
                  <pre
                    class="text-[11px] font-mono whitespace-pre-wrap break-all px-1">{resBody}</pre>
                {/if}
              {:else}
                <div class="text-slate-400 italic text-[11px] py-4 text-center">
                  Pending...
                </div>
              {/if}
            </div>
          </Pane>
        </PaneGroup>
      </Pane>
      <PaneResizer
        class="h-1 bg-transparent hover:bg-indigo-500/30 cursor-row-resize shrink-0 transition-colors border-t border-slate-200 dark:border-[#30363d]"
      />
      <Pane
        defaultSize={30}
        minSize={20}
        class="flex flex-col bg-slate-50/50 dark:bg-black/20 overflow-hidden"
      >
        <div
          class="h-8 bg-slate-100/50 dark:bg-[#1c2128] border-b border-slate-200 dark:border-[#30363d] flex items-center px-2 shrink-0"
        >
          <span
            class="text-xs font-black text-indigo-600 dark:text-indigo-400 uppercase tracking-widest"
            >Script Logs</span
          >
          {#if scriptLogs.length > 0}
            <span
              class="ml-2 px-1.5 py-0.5 rounded bg-indigo-100 dark:bg-indigo-900/40 text-indigo-600 dark:text-indigo-400 text-[9px] font-bold"
              >{scriptLogs.length}</span
            >
          {/if}
        </div>
        <div class="flex-1 overflow-y-auto p-2 space-y-1.5">
          {#if scriptLogs.length === 0}
            <div class="text-slate-400 italic text-[10px] py-10 text-center">
              No script output.
            </div>
          {:else}
            {#each scriptLogs as log}
              <div
                class="flex flex-col border-b border-black/5 dark:border-white/5 pb-1 last:border-0 border-slate-100 dark:border-white/5"
              >
                <div class="flex gap-2 items-baseline">
                  <span class="text-slate-400 shrink-0 text-[10px] font-mono"
                    >{new Date(log.timestamp)
                      .toLocaleTimeString()
                      ?.split(" ")[0]}</span
                  >
                  <span
                    class="font-bold {log.level === 'error'
                      ? 'text-red-500'
                      : 'text-indigo-500'} uppercase text-[10px]"
                  >
                    {log.level}
                  </span>
                  {#if typeof log.message === "string"}
                    <span
                      class="text-xs font-mono text-slate-700 dark:text-slate-300 break-all leading-tight"
                      >{log.message}</span
                    >
                  {/if}
                </div>
                {#if typeof log.message === "object"}
                  <div
                    class="mt-1 border border-black/5 dark:border-white/10 rounded overflow-hidden"
                  >
                    <JsonViewer data={log.message} />
                  </div>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
      </Pane>
    {:else}
      <Pane
        defaultSize={33}
        minSize={15}
        class="flex flex-col border-r border-slate-200 dark:border-[#30363d] overflow-hidden"
      >
        <div
          class="h-8 bg-slate-100/50 dark:bg-[#1c2128] border-b border-slate-200 dark:border-[#30363d] flex items-center px-2 shrink-0"
        >
          <span
            class="text-xs font-black text-slate-500 uppercase tracking-widest"
            >Request</span
          >
          {#if !isResponseOnly}
            <div class="flex h-full gap-1 ml-4">
              <button
                class="px-2 h-full text-xs font-bold {reqTab === 'headers'
                  ? 'text-indigo-600 dark:text-indigo-400 border-b-2 border-indigo-500'
                  : 'text-slate-400'}"
                onclick={() => (reqTab = "headers")}>Headers</button
              >
              <button
                class="px-2 h-full text-xs font-bold {reqTab === 'body'
                  ? 'text-indigo-600 dark:text-indigo-400 border-b-2 border-indigo-500'
                  : 'text-slate-400'}"
                onclick={() => (reqTab = "body")}>Body</button
              >
            </div>
          {/if}
        </div>
        <div class="flex-1 overflow-y-auto">
          {#if isResponseOnly}
            <div class="p-4 flex flex-col items-center justify-center text-slate-400 italic text-[11px] text-center gap-2">
              <p>Original request details are not available in this window.</p>
              <p class="text-[9px] opacity-60">This can happen if the interceptor window was opened after the request was sent.</p>
            </div>
          {:else if reqTab === "headers"}
            <div class="p-2 space-y-0.5">
              {#each headers as [k, v], i}
                <div
                  class="flex text-xs font-mono border-b border-black/5 dark:border-white/5 py-0.5 gap-2 items-center"
                >
                  {#if editable}
                    <input
                      value={k}
                      oninput={(e) => updateHeader(i, e.currentTarget.value, v)}
                      class="w-32 shrink-0 bg-transparent border-none p-0 text-indigo-600 dark:text-indigo-400 font-bold focus:ring-0"
                    />
                    <input
                      value={v}
                      oninput={(e) => updateHeader(i, k, e.currentTarget.value)}
                      class="flex-1 bg-transparent border-none p-0 text-slate-600 dark:text-slate-300 focus:ring-0"
                    />
                    <button
                      onclick={() => removeHeader(i)}
                      class="p-0.5 text-slate-400 hover:text-red-500"
                    >
                      <XCircle size={12} />
                    </button>
                  {:else}
                    <span
                      class="w-32 shrink-0 text-indigo-600 dark:text-indigo-400 font-bold truncate"
                      >{k}</span
                    >
                    <span
                      class="flex-1 text-slate-600 dark:text-slate-300 break-all"
                      >{v}</span
                    >
                  {/if}
                </div>
              {/each}
              {#if editable}
                <button
                  onclick={addHeader}
                  class="mt-2 text-[10px] font-bold text-indigo-600 dark:text-indigo-400 hover:underline"
                >
                  + Add Header
                </button>
              {/if}
            </div>
          {:else if reqBody === null}
            <div class="text-slate-400 italic text-[11px] p-2">No body.</div>
          {:else if typeof reqBody === "object" && !editable}
            <div class="text-xs">
              <JsonViewer data={reqBody} />
            </div>
          {:else if editable}
            <div class="h-full min-h-[300px]">
              <CodeEditor
                value={editableBody}
                onchange={onBodyChange}
                darkMode={isDark}
              />
            </div>
          {:else}
            <pre
              class="text-[11px] font-mono whitespace-pre-wrap break-all px-1">{reqBody}</pre>
          {/if}
        </div>
      </Pane>
      <PaneResizer
        class="w-1 bg-transparent hover:bg-indigo-500/30 cursor-col-resize shrink-0 transition-colors"
      />

      <Pane
        defaultSize={34}
        minSize={15}
        class="flex flex-col border-r border-slate-200 dark:border-[#30363d] overflow-hidden"
      >
        <div
          class="h-8 bg-slate-100/50 dark:bg-[#1c2128] border-b border-slate-200 dark:border-[#30363d] flex items-center justify-between px-2 shrink-0"
        >
          <div class="flex items-center h-full">
            <span
              class="text-xs font-black text-slate-500 uppercase tracking-widest"
              >Response</span
            >
            <div class="flex h-full gap-1 ml-4">
              <button
                class="px-2 h-full text-xs font-bold {resTab === 'headers'
                  ? 'text-indigo-600 dark:text-indigo-400 border-b-2 border-indigo-500'
                  : 'text-slate-400'}"
                onclick={() => (resTab = "headers")}>Headers</button
              >
              <button
                class="px-2 h-full text-xs font-bold {resTab === 'body'
                  ? 'text-indigo-600 dark:text-indigo-400 border-b-2 border-indigo-500'
                  : 'text-slate-400'}"
                onclick={() => (resTab = "body")}>Body</button
              >
            </div>
          </div>
          {#if editable && res}
            <input
              type="number"
              bind:value={res.status}
              class="w-12 bg-slate-100 dark:bg-black/20 border border-slate-200 dark:border-white/10 px-1 py-0.5 rounded text-xs font-bold text-center focus:outline-none focus:ring-1 focus:ring-indigo-500"
            />
          {:else if res}
            <span class="text-xs font-bold {statusColor(res.status)}"
              >{res.status}</span
            >
          {/if}
        </div>
        <div class="flex-1 overflow-y-auto">
          {#if res}
            {#if resTab === "headers"}
              <div class="p-2 space-y-0.5">
                {#each resHeaders as [k, v], i}
                  <div
                    class="flex text-xs font-mono border-b border-black/5 dark:border-white/5 py-0.5 gap-2 items-center"
                  >
                    {#if editable}
                      <input
                        value={k}
                        oninput={(e) =>
                          updateResHeader(i, e.currentTarget.value, v)}
                        class="w-32 shrink-0 bg-transparent border-none p-0 text-indigo-600 dark:text-indigo-400 font-bold focus:ring-0"
                      />
                      <input
                        value={v}
                        oninput={(e) =>
                          updateResHeader(i, k, e.currentTarget.value)}
                        class="flex-1 bg-transparent border-none p-0 text-slate-600 dark:text-slate-300 focus:ring-0"
                      />
                      <button
                        onclick={() => removeResHeader(i)}
                        class="p-0.5 text-slate-400 hover:text-red-500"
                      >
                        <XCircle size={12} />
                      </button>
                    {:else}
                      <span
                        class="w-32 shrink-0 text-indigo-600 dark:text-indigo-400 font-bold truncate"
                        >{k}</span
                      >
                      <span
                        class="flex-1 text-slate-600 dark:text-slate-300 break-all"
                        >{v}</span
                      >
                    {/if}
                  </div>
                {/each}
                {#if editable}
                  <button
                    onclick={addResHeader}
                    class="mt-2 text-[10px] font-bold text-indigo-600 dark:text-indigo-400 hover:underline"
                  >
                    + Add Header
                  </button>
                {/if}
              </div>
            {:else if resBody === null}
              <div class="text-slate-400 italic text-[11px] p-2">No body.</div>
            {:else if typeof resBody === "object" && !editable}
              <div class="text-xs">
                <JsonViewer data={resBody} />
              </div>
            {:else if editable}
              <div class="h-full min-h-[300px]">
                <CodeEditor
                  value={editableResBody}
                  onchange={onResBodyChange}
                  darkMode={isDark}
                />
              </div>
            {:else}
              <pre
                class="text-[11px] font-mono whitespace-pre-wrap break-all px-1">{resBody}</pre>
            {/if}
          {:else}
            <div class="text-slate-400 italic text-[11px] py-4 text-center">
              Pending...
            </div>
          {/if}
        </div>
      </Pane>
      <PaneResizer
        class="w-1 bg-transparent hover:bg-indigo-500/30 cursor-col-resize shrink-0 transition-colors"
      />

      <Pane
        defaultSize={33}
        minSize={15}
        class="flex flex-col bg-slate-50/50 dark:bg-black/20 overflow-hidden"
      >
        <div
          class="h-8 bg-slate-100/50 dark:bg-[#1c2128] border-b border-slate-200 dark:border-[#30363d] flex items-center px-2 shrink-0"
        >
          <span
            class="text-[10px] font-black text-indigo-600 dark:text-indigo-400 uppercase tracking-widest"
            >Script Logs</span
          >
          {#if scriptLogs.length > 0}
            <span
              class="ml-2 px-1.5 py-0.5 rounded bg-indigo-100 dark:bg-indigo-900/40 text-indigo-600 dark:text-indigo-400 text-[9px] font-bold"
              >{scriptLogs.length}</span
            >
          {/if}
        </div>
        <div class="flex-1 overflow-y-auto p-2 space-y-1.5">
          {#if scriptLogs.length === 0}
            <div class="text-slate-400 italic text-[10px] py-10 text-center">
              No script output.
            </div>
          {:else}
            {#each scriptLogs as log}
              <div
                class="flex flex-col border-b border-black/5 dark:border-white/5 pb-1 last:border-0 border-slate-100 dark:border-white/5"
              >
                <div class="flex gap-2 items-baseline">
                  <span class="text-slate-400 shrink-0 text-[10px] font-mono"
                    >{new Date(log.timestamp)
                      .toLocaleTimeString()
                      ?.split(" ")[0]}</span
                  >
                  <span
                    class="font-bold {log.level === 'error'
                      ? 'text-red-500'
                      : 'text-indigo-500'} uppercase text-[10px]"
                  >
                    {log.level}
                  </span>
                  {#if typeof log.message === "string"}
                    <span
                      class="text-xs font-mono text-slate-700 dark:text-slate-300 break-all leading-tight"
                      >{log.message}</span
                    >
                  {/if}
                </div>
                {#if typeof log.message === "object"}
                  <div
                    class="mt-1 border border-black/5 dark:border-white/10 rounded overflow-hidden"
                  >
                    <JsonViewer data={log.message} />
                  </div>
                {/if}
              </div>
            {/each}
          {/if}
        </div>
      </Pane>
    {/if}
  </PaneGroup>

  {#if editable && (onAbort || onExecute)}
    <div class="h-12 border-t border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0 flex items-center justify-center px-4 gap-4">
      <div class="flex-1"></div>
      <div class="flex items-center gap-3">
        <button 
          onclick={onAbort}
          class="flex items-center gap-1.5 px-4 py-1.5 text-red-500 hover:bg-red-50 dark:hover:bg-red-500/10 rounded-md text-[11px] font-bold transition-all active:scale-95"
        >
          <XCircle size={14} />
          Abort
        </button>
        <button 
          onclick={onExecute}
          class="flex items-center gap-2 px-8 py-1.5 bg-indigo-600 hover:bg-indigo-700 text-white rounded-md text-[11px] font-bold transition-all active:scale-95 shadow-lg shadow-indigo-500/20"
        >
          <Play size={14} fill="currentColor" />
          Execute
        </button>
      </div>
      <div class="flex-1 text-right">
        <span class="text-[9px] text-slate-400 font-medium">Ctrl+Enter to Execute</span>
      </div>
    </div>
  {/if}
</div>
