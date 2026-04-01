<script lang="ts">
  type Props = {
    data: any;
  };

  let { data }: Props = $props();

  let copied = $state(false);

  function syntaxHighlight(jsonObj: any) {
    let json = JSON.stringify(jsonObj, null, 2);
    if (!json) return "";
    json = json
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
    return json.replace(
      /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
      function (match) {
        let cls = "text-orange-600 dark:text-orange-400 font-semibold"; // number
        if (/^"/.test(match)) {
          if (/:$/.test(match)) {
            cls = "text-sky-600 dark:text-sky-400 font-bold"; // key
          } else {
            cls = "text-emerald-600 dark:text-emerald-400"; // string
          }
        } else if (/true|false/.test(match)) {
          cls = "text-purple-600 dark:text-purple-400 font-bold"; // boolean
        } else if (/null/.test(match)) {
          cls = "text-slate-400 dark:text-slate-500 italic font-bold"; // null
        }
        return '<span class="' + cls + '">' + match + "</span>";
      }
    );
  }

  let htmlData = $derived(syntaxHighlight(data));

  function copyToClipboard() {
    const raw = JSON.stringify(data, null, 2);
    navigator.clipboard.writeText(raw).catch(() => {});
    copied = true;
    setTimeout(() => {
      copied = false;
    }, 1500);
  }
</script>

<div class="relative group min-h-full">
  <div class="sticky top-0 z-20 flex justify-end h-0 w-[calc(100%-8px)] pointer-events-none pt-2">
    <button
      class="pointer-events-auto flex items-center justify-center gap-1.5 px-2 py-1 bg-white dark:bg-[#21262d] border border-slate-200 dark:border-[#30363d] rounded text-slate-500 hover:text-slate-900 dark:hover:text-white opacity-0 group-hover:opacity-100 transition-opacity select-none shadow-sm {copied ? 'opacity-100 text-emerald-600 dark:text-emerald-400 border-emerald-200 dark:border-emerald-800' : ''}"
      title="Copy JSON"
      onclick={copyToClipboard}
    >
    {#if copied}
      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
      <span class="text-[10px] font-bold uppercase tracking-wider">Copied</span>
    {:else}
      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
      <span class="text-[10px] font-bold uppercase tracking-wider">Copy</span>
    {/if}
    </button>
  </div>
  <pre
    class="m-0 p-3 font-mono text-[12px] leading-relaxed break-all whitespace-pre-wrap text-slate-800 dark:text-slate-200 min-h-full">{@html htmlData}</pre>
</div>
