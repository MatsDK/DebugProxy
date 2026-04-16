<script lang="ts">
  import { normalizeUri, buildCurl } from "$lib/utils";
  
  let { req, res, x, y, onClose } = $props<{
    req: any;
    res: any;
    x: number;
    y: number;
    onClose: () => void;
  }>();

  function copy(text: string) {
    navigator.clipboard.writeText(text).catch(() => {});
    onClose();
  }
</script>

<div
  class="fixed z-50 bg-white dark:bg-[#161b22] border border-slate-200 dark:border-[#30363d] rounded shadow-lg py-1 min-w-[160px] text-xs"
  style="left: {x}px; top: {y}px;"
  onmousedown={(e) => e.stopPropagation()}
  role="presentation"
>
  {#if req}
    <button
      onclick={() => copy(normalizeUri(req.uri))}
      class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d]"
      >Copy URL</button
    >
    <button
      onclick={() => copy(JSON.stringify(req.headers, null, 2))}
      class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d]"
      >Copy Request Headers</button
    >
    <button
      onclick={() => copy(buildCurl(req))}
      class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d]"
      >Copy as cURL</button
    >
  {/if}
  {#if res}
    <div class="h-px bg-slate-200 dark:bg-[#30363d] my-1"></div>
    <button
      onclick={() => copy(JSON.stringify(res.headers, null, 2))}
      class="w-full text-left px-3 py-1.5 hover:bg-slate-100 dark:hover:bg-[#21262d]"
      >Copy Response Headers</button
    >
  {/if}
</div>
