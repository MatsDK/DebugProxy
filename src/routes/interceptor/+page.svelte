<script lang="ts">
  import { onMount } from "svelte";
  import { ProxyState } from "$lib/proxy.svelte";
  import InterceptorPanel from "$lib/components/InterceptorPanel.svelte";
  import { windowState } from "$lib/window.svelte";

  const proxy = new ProxyState();

  onMount(() => {
    windowState.toggleInterceptorWindow(true);
  });
</script>

<div class="h-screen flex flex-col overflow-hidden bg-white dark:bg-[#0d1117]">
  <InterceptorPanel {proxy} />
</div>
