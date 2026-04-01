<script lang="ts">
  import type { ProxyEvent } from "$lib/types";
  import JsonViewer from "./JsonViewer.svelte";

  let { req, res }: { req: ProxyEvent; res: ProxyEvent | null } = $props();

  let reqTab = $state<"headers" | "body">("body");
  let resTab = $state<"headers" | "body">("body");

  let prevReqId = $state(-1);
  let prevResId = $state(-1);

  $effect(() => {
    if (req && req.id !== prevReqId) {
      prevReqId = req.id;
      reqTab = req.body_base64 && req.body_base64.length > 0 ? "body" : "headers";
    }
  });

  $effect(() => {
    if (res && res.id !== prevResId) {
      prevResId = res.id;
      resTab = res.body_base64 && res.body_base64.length > 0 ? "body" : "headers";
    }
  });

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

  function formatBody(b64: string | null) {
    if (!b64) return null;
    try {
      return JSON.parse(atob(b64));
    } catch {
      return atob(b64);
    }
  }

  let reqBody = $derived(formatBody(req.body_base64));
  let resBody = $derived(res ? formatBody(res.body_base64) : null);
</script>

<div class="flex flex-col flex-1 min-h-0 bg-white dark:bg-[#0d1117]">
  <!-- Details header -->
  <div class="px-4 py-2 border-b border-slate-200 dark:border-[#30363d] bg-slate-50 dark:bg-[#161b22] shrink-0 flex items-baseline gap-2">
    <span class="font-mono font-semibold text-xs {methodColor(req.method)}">{req.method}</span>
    <span class="font-mono text-xs text-blue-600 dark:text-blue-400 break-all select-all">{req.uri}</span>
  </div>

  <div class="flex flex-1 min-h-0">
    <!-- Request Pane -->
    <div class="flex flex-col flex-1 min-w-0 border-r border-slate-200 dark:border-[#30363d]">
      <div class="bg-slate-100 dark:bg-[#21262d] border-b border-slate-200 dark:border-[#30363d] text-[11px] font-semibold uppercase text-slate-500 flex items-center shrink-0">
        <span class="px-3 py-1 border-r border-slate-200 dark:border-[#30363d]">Request</span>
        
        <button
          class="px-3 py-1 hover:bg-slate-200 dark:hover:bg-[#30363d] transition-colors border-r border-slate-200 dark:border-[#30363d] {reqTab === 'headers' ? 'text-blue-600 dark:text-blue-400 bg-white dark:bg-[#0d1117]' : ''}"
          onclick={() => (reqTab = "headers")}
        >
          Headers ({req.headers.length})
        </button>
        <button
          class="px-3 py-1 hover:bg-slate-200 dark:hover:bg-[#30363d] transition-colors {reqTab === 'body' ? 'text-blue-600 dark:text-blue-400 bg-white dark:bg-[#0d1117]' : ''}"
          onclick={() => (reqTab = "body")}
        >
          Body
        </button>
      </div>

      <div class="flex-1 overflow-y-auto bg-white dark:bg-[#0d1117]">
        {#if reqTab === "headers"}
          <div class="border border-slate-200 dark:border-[#30363d] rounded overflow-hidden">
            {#each req.headers as [k, v]}
              <div class="flex text-[11px] font-mono border-b border-slate-100 dark:border-[#21262d] last:border-b-0">
                <span class="w-36 shrink-0 px-2 py-1 bg-slate-50 dark:bg-[#161b22] text-blue-700 dark:text-blue-300 font-semibold border-r border-slate-200 dark:border-[#30363d] break-all">{k}</span>
                <span class="flex-1 px-2 py-1 text-slate-700 dark:text-slate-300 break-all">{v}</span>
              </div>
            {/each}
            {#if !req.headers.length}
              <div class="px-2 py-1 text-xs text-slate-400 italic">No headers found</div>
            {/if}
          </div>
        {:else}
          {#if reqBody === null}
            <div class="text-slate-400 italic text-xs">No request body captured.</div>
          {:else if typeof reqBody === "object"}
            <JsonViewer data={reqBody} />
          {:else}
            <pre class="text-[11px] font-mono bg-white dark:bg-[#0d1117] p-3 m-0 whitespace-pre-wrap break-all text-slate-800 dark:text-slate-200 min-h-full">{reqBody}</pre>
          {/if}
        {/if}
      </div>
    </div>

    <!-- Response Pane -->
    <div class="flex flex-col flex-1 min-w-0">
      <div class="bg-slate-100 dark:bg-[#21262d] border-b border-slate-200 dark:border-[#30363d] text-[11px] font-semibold uppercase text-slate-500 flex justify-between items-center shrink-0">
        <div class="flex items-center">
          <span class="px-3 py-1 border-r border-slate-200 dark:border-[#30363d]">Response</span>
          
          <button
            class="px-3 py-1 border-r border-slate-200 dark:border-[#30363d] hover:bg-slate-200 dark:hover:bg-[#30363d] transition-colors {resTab === 'headers' ? 'text-blue-600 dark:text-blue-400 bg-white dark:bg-[#0d1117]' : ''}"
            onclick={() => (resTab = "headers")}
          >
            Headers ({res?.headers.length ?? 0})
          </button>
          <button
            class="px-3 py-1 hover:bg-slate-200 dark:hover:bg-[#30363d] transition-colors border-r border-slate-200 dark:border-[#30363d] {resTab === 'body' ? 'text-blue-600 dark:text-blue-400 bg-white dark:bg-[#0d1117]' : ''}"
            onclick={() => (resTab = "body")}
          >
            Body
          </button>
        </div>
        {#if res}
          <span class="px-3 py-1 font-semibold {statusColor(res.status)}">{res.status}</span>
        {:else}
          <span class="px-3 py-1 text-slate-400 text-[10px] capitalize font-medium">Waiting...</span>
        {/if}
      </div>

      <div class="flex-1 overflow-y-auto bg-white dark:bg-[#0d1117]">
        {#if res}
          {#if resTab === "headers"}
            <div class="border border-slate-200 dark:border-[#30363d] rounded overflow-hidden">
              {#each res.headers as [k, v]}
                <div class="flex text-[11px] font-mono border-b border-slate-100 dark:border-[#21262d] last:border-b-0">
                  <span class="w-36 shrink-0 px-2 py-1 bg-slate-50 dark:bg-[#161b22] text-blue-700 dark:text-blue-300 font-semibold border-r border-slate-200 dark:border-[#30363d] break-all">{k}</span>
                  <span class="flex-1 px-2 py-1 text-slate-700 dark:text-slate-300 break-all">{v}</span>
                </div>
              {/each}
              {#if !res.headers.length}
                <div class="px-2 py-1 text-xs text-slate-400 italic">No headers found</div>
              {/if}
            </div>
          {:else}
            {#if resBody === null}
              <div class="text-slate-400 italic text-xs">No response body captured.</div>
            {:else if typeof resBody === "object"}
              <JsonViewer data={resBody} />
            {:else}
              <pre class="text-[11px] font-mono bg-white dark:bg-[#0d1117] p-3 m-0 whitespace-pre-wrap break-all text-slate-800 dark:text-slate-200 min-h-full">{resBody}</pre>
            {/if}
          {/if}
        {:else}
          <div class="text-slate-400 italic text-xs mt-10 text-center">Response is pending...</div>
        {/if}
      </div>
    </div>
  </div>
</div>
