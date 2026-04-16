<script lang="ts">
  import type { ProxyEvent, ScriptLog } from "$lib/types";
  import JsonViewer from "./JsonViewer.svelte";
  import { PaneGroup, Pane, PaneResizer } from "paneforge";
  import { taurpc } from "$lib/rpc";
  import { windowState } from "$lib/window.svelte";

  let {
    req,
    res,
    logs,
  }: { req: ProxyEvent; res: ProxyEvent | null; logs: ScriptLog[] } = $props();

  let reqTab = $state<"headers" | "body">("headers");
  let resTab = $state<"headers" | "body">("headers");

  $effect(() => {
    if (req && req.body) {
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

  function formatBodyStr(data: Uint8Array | number[] | null): any {
    if (!data) return null;
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

  let reqBody = $derived(formatBodyStr(req.body));
  let resBody = $derived(res ? formatBodyStr(res.body) : null);

  // Container width observer
  let containerWidth = $state(0);
  let isNarrow = $derived(containerWidth > 0 && containerWidth < 1400);
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
    <span
      class="font-mono text-xs text-slate-500 dark:text-slate-400 truncate select-all"
      >{req.uri}</span
    >
    <div class="ml-auto flex items-center">
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
            </div>
            <div class="flex-1 overflow-y-auto p-2">
              {#if reqTab === "headers"}
                <div class="space-y-0.5">
                  {#each req.headers as [k, v]}
                    <div
                      class="flex text-xs font-mono border-b border-black/5 dark:border-white/5 py-0.5"
                    >
                      <span
                        class="w-32 shrink-0 text-indigo-600 dark:text-indigo-400 font-bold truncate"
                        >{k}</span
                      >
                      <span
                        class="flex-1 text-slate-600 dark:text-slate-300 break-all"
                        >{v}</span
                      >
                    </div>
                  {/each}
                </div>
              {:else if reqBody === null}
                <div class="text-slate-400 italic text-[11px]">No body.</div>
              {:else if typeof reqBody === "object"}
                <div class="text-xs">
                  <JsonViewer data={reqBody} />
                </div>
              {:else}
                <pre
                  class="text-[11px] font-mono whitespace-pre-wrap break-all">{reqBody}</pre>
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
                    class="px-2 h-full text-xs font-bold {resTab ===
                    'headers'
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
              {#if res}
                <span class="text-xs font-bold {statusColor(res.status)}"
                  >{res.status}</span
                >
              {/if}
            </div>
            <div class="flex-1 overflow-y-auto p-2">
              {#if res}
                {#if resTab === "headers"}
                  <div class="space-y-0.5">
                    {#each res.headers as [k, v]}
                      <div
                        class="flex text-xs font-mono border-b border-black/5 dark:border-white/5 py-0.5"
                      >
                        <span
                          class="w-32 shrink-0 text-indigo-600 dark:text-indigo-400 font-bold truncate"
                          >{k}</span
                        >
                        <span
                          class="flex-1 text-slate-600 dark:text-slate-300 break-all"
                          >{v}</span
                        >
                      </div>
                    {/each}
                  </div>
                {:else if resBody === null}
                  <div class="text-slate-400 italic text-[11px]">No body.</div>
                {:else if typeof resBody === "object"}
                  <div class="text-xs">
                    <JsonViewer data={resBody} />
                  </div>
                {:else}
                  <pre
                    class="text-[11px] font-mono whitespace-pre-wrap break-all">{resBody}</pre>
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
        defaultSize={50}
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
        </div>
        <div class="flex-1 overflow-y-auto p-2">
          {#if reqTab === "headers"}
            <div class="space-y-0.5">
              {#each req.headers as [k, v]}
                <div
                  class="flex text-xs font-mono border-b border-black/5 dark:border-white/5 py-0.5"
                >
                  <span
                    class="w-32 shrink-0 text-indigo-600 dark:text-indigo-400 font-bold truncate"
                    >{k}</span
                  >
                  <span
                    class="flex-1 text-slate-600 dark:text-slate-300 break-all"
                    >{v}</span
                  >
                </div>
              {/each}
            </div>
          {:else if reqBody === null}
            <div class="text-slate-400 italic text-[11px]">No body.</div>
          {:else if typeof reqBody === "object"}
            <div class="text-xs">
              <JsonViewer data={reqBody} />
            </div>
          {:else}
            <pre
              class="text-[11px] font-mono whitespace-pre-wrap break-all">{reqBody}</pre>
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
          {#if res}
            <span class="text-xs font-bold {statusColor(res.status)}"
              >{res.status}</span
            >
          {/if}
        </div>
        <div class="flex-1 overflow-y-auto p-2">
          {#if res}
            {#if resTab === "headers"}
              <div class="space-y-0.5">
                {#each res.headers as [k, v]}
                  <div
                    class="flex text-xs font-mono border-b border-black/5 dark:border-white/5 py-0.5"
                  >
                    <span
                      class="w-32 shrink-0 text-indigo-600 dark:text-indigo-400 font-bold truncate"
                      >{k}</span
                    >
                    <span
                      class="flex-1 text-slate-600 dark:text-slate-300 break-all"
                      >{v}</span
                    >
                  </div>
                {/each}
              </div>
            {:else if resBody === null}
              <div class="text-slate-400 italic text-[11px]">No body.</div>
            {:else if typeof resBody === "object"}
              <div class="text-xs">
                <JsonViewer data={resBody} />
              </div>
            {:else}
              <pre
                class="text-[11px] font-mono whitespace-pre-wrap break-all">{resBody}</pre>
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
</div>
