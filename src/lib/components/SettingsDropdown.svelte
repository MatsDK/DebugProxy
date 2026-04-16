<script lang="ts">
  import { taurpc } from "$lib/rpc";
  import type { ProxyState } from "$lib/proxy.svelte";
  import { toast } from "$lib/toast.svelte";
  import {
    Settings,
    Download,
    Sun,
    Moon,
    FileUp,
    FileDown,
    Trash2,
    RotateCcw,
    ChevronDown,
    ExternalLink,
    Shield,
  } from "lucide-svelte";
  import SslSettingsModal from "./SslSettingsModal.svelte";

  let { proxy }: { proxy: ProxyState } = $props();

  let isOpen = $state(false);
  let isSslSettingsOpen = $state(false);
  let el = $state<HTMLElement | null>(null);

  function toggle() {
    isOpen = !isOpen;
  }

  function close() {
    isOpen = false;
  }

  // Handle click outside to close
  $effect(() => {
    if (!isOpen) return;
    const handleOutside = (e: MouseEvent) => {
      if (el && !el.contains(e.target as Node)) close();
    };
    window.addEventListener("mousedown", handleOutside);
    return () => window.removeEventListener("mousedown", handleOutside);
  });

  async function toggleDark() {
    await proxy.setTheme(!proxy.isDark);
  }

  async function handleExport() {
    try {
      const settings = {
        port: proxy.port,
        interceptSsl: proxy.interceptSsl,
        isBlocked: proxy.isBlocked,
        sslExceptionPatterns: $state.snapshot(proxy.sslExceptionPatterns),
        theme: proxy.isDark ? "dark" : "light",
        scripts: $state.snapshot(proxy.scripts.list),
        scriptsEnabled: proxy.scripts.enabled,
      };
      await taurpc.export_settings(settings);
      toast.success("Settings exported");
      close();
    } catch (e: any) {
      toast.error("Export failed: " + e);
    }
  }

  async function handleImport() {
    try {
      const settings = await taurpc.import_settings();
      if (settings) {
        // App settings update
        proxy.port = settings.port;
        proxy.interceptSsl = settings.interceptSsl;
        proxy.isBlocked = settings.isBlocked;
        proxy.sslExceptionPatterns = settings.sslExceptionPatterns;
        proxy.scripts.hydrate(settings.scripts, settings.scriptsEnabled);

        await proxy.setTheme(settings.theme === "dark");

        toast.success("Settings imported");
        close();
      }
    } catch (e: any) {
      toast.error("Import failed: " + e);
    }
  }

  async function handleReset() {
    if (
      !confirm(
        "Are you sure you want to reset all settings to defaults? This will erase all scripts.",
      )
    )
      return;
    try {
      const settings = await taurpc.reset_settings();
      proxy.port = settings.port;
      proxy.interceptSsl = settings.interceptSsl;
      proxy.isBlocked = settings.isBlocked;
      proxy.sslExceptionPatterns = settings.sslExceptionPatterns;
      proxy.scripts.hydrate(settings.scripts, settings.scriptsEnabled);

      await proxy.setTheme(settings.theme === "dark");

      toast.success("Settings reset to defaults");
      close();
    } catch (e: any) {
      toast.error("Reset failed: " + e);
    }
  }

  async function exportCert() {
    try {
      await taurpc.export_ca_cert();
      toast.success("Certificate exported");
      close();
    } catch (e: any) {
      toast.error("Export failed: " + e);
    }
  }

  function clearHistory() {
    if (!confirm("Clear all captured traffic?")) return;
    proxy.clearTraffic();
    toast.success("Traffic cleared");
    close();
  }
</script>

<div class="relative inline-block font-sans" bind:this={el}>
  <button
    onclick={toggle}
    class="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-bold text-slate-600 dark:text-slate-300 hover:bg-slate-200/50 dark:hover:bg-white/5 rounded-md transition-all active:scale-95 border border-transparent hover:border-slate-200 dark:hover:border-white/10"
  >
    <Settings
      size={14}
      class="shrink-0 {isOpen
        ? 'rotate-90'
        : ''} transition-transform duration-300"
    />
    <span>Settings</span>
    <ChevronDown
      size={12}
      class="opacity-50 shrink-0 {isOpen
        ? 'rotate-180'
        : ''} transition-transform duration-200"
    />
  </button>

  {#if isOpen}
    <div
      class="fixed sm:absolute right-4 top-12 sm:right-0 sm:top-full mt-1.5 w-56 bg-white dark:bg-[#161b22] border border-slate-200 dark:border-[#30363d] rounded-md shadow-xl z-[100] overflow-hidden animate-in fade-in zoom-in-95 duration-100 origin-top-right"
    >
      <div class="p-1 space-y-0.5">
        <!-- Appearance Toggle (Compact) -->
        <button
          onclick={toggleDark}
          class="w-full px-2 py-1.5 flex items-center justify-between hover:bg-slate-50 dark:hover:bg-white/5 rounded transition-colors group"
        >
          <div class="flex items-center gap-2">
            {#if proxy.isDark}
              <Moon
                size={14}
                class="text-indigo-400 group-hover:scale-110 transition-transform"
              />
            {:else}
              <Sun
                size={14}
                class="text-amber-500 group-hover:scale-110 transition-transform"
              />
            {/if}
            <span
              class="text-[11px] font-semibold text-slate-700 dark:text-slate-300"
              >Appearance</span
            >
          </div>
          <span
            class="text-[9px] font-bold uppercase tracking-wider text-slate-400 bg-slate-100 dark:bg-white/5 px-1.5 py-0.5 rounded"
          >
            {proxy.isDark ? "Dark" : "Light"}
          </span>
        </button>

        <div class="h-px bg-slate-100 dark:bg-[#30363d] my-1 mx-1"></div>

        <!-- Utility Group -->
        <button
          onclick={exportCert}
          class="w-full flex items-center gap-2 px-2 py-1.5 text-left hover:bg-slate-50 dark:hover:bg-white/5 rounded transition-colors group"
        >
          <Download
            size={14}
            class="text-slate-400 group-hover:text-indigo-500"
          />
          <span
            class="text-[11px] font-medium text-slate-600 dark:text-slate-300"
            >Download Root CA</span
          >
        </button>

        <button
          onclick={clearHistory}
          class="w-full flex items-center gap-2 px-2 py-1.5 text-left hover:bg-slate-50 dark:hover:bg-white/5 rounded transition-colors group"
        >
          <Trash2 size={14} class="text-slate-400 group-hover:text-red-500" />
          <span
            class="text-[11px] font-medium text-slate-600 dark:text-slate-300"
            >Clear Traffic</span
          >
        </button>

        <button
          onclick={() => {
            isSslSettingsOpen = true;
            close();
          }}
          class="w-full flex items-center gap-2 px-2 py-1.5 text-left hover:bg-slate-50 dark:hover:bg-white/5 rounded transition-colors group"
        >
          <Shield size={14} class="text-slate-400 group-hover:text-amber-500" />
          <span
            class="text-[11px] font-medium text-slate-600 dark:text-slate-300"
            >SSL Settings</span
          >
        </button>

        <div class="h-px bg-slate-100 dark:bg-[#30363d] my-1 mx-1"></div>

        <!-- Backup & Restore -->
        <div
          class="px-2 py-1 text-[9px] font-bold text-slate-400 uppercase tracking-tighter"
        >
          Configuration
        </div>
        <button
          onclick={handleImport}
          class="w-full flex items-center gap-2 px-2 py-1.5 text-left hover:bg-slate-50 dark:hover:bg-white/5 rounded transition-colors group"
        >
          <FileUp
            size={14}
            class="text-slate-400 group-hover:text-emerald-500"
          />
          <span
            class="text-[11px] font-medium text-slate-600 dark:text-slate-300"
            >Import settings.json...</span
          >
        </button>

        <button
          onclick={handleExport}
          class="w-full flex items-center gap-2 px-2 py-1.5 text-left hover:bg-slate-50 dark:hover:bg-white/5 rounded transition-colors group"
        >
          <FileDown
            size={14}
            class="text-slate-400 group-hover:text-indigo-500"
          />
          <span
            class="text-[11px] font-medium text-slate-600 dark:text-slate-300"
            >Export settings.json...</span
          >
        </button>

        <div class="h-px bg-slate-100 dark:bg-[#30363d] my-1 mx-1"></div>

        <!-- Danger Zone -->
        <button
          onclick={handleReset}
          class="w-full flex items-center gap-2 px-2 py-1.5 text-left hover:bg-red-500 hover:text-white dark:hover:bg-red-500/20 dark:hover:text-red-400 rounded transition-colors group"
        >
          <RotateCcw
            size={14}
            class="text-slate-400 group-hover:text-inherit"
          />
          <span
            class="text-[11px] font-medium text-slate-600 dark:text-slate-300 group-hover:text-inherit underline-offset-2"
            >Factory Reset</span
          >
        </button>
      </div>

      <div
        class="bg-slate-50 dark:bg-[#0d1117] px-2.5 py-1.5 border-t border-slate-200 dark:border-[#30363d] flex items-center justify-between"
      >
        <span
          class="text-[8px] font-bold text-slate-400 uppercase tracking-widest leading-none"
          >Debugger Proxy v0.1</span
        >
        <a
          href="https://github.com/MatsDK/DebugProxy"
          target="_blank"
          class="text-slate-400 hover:text-indigo-500 transition-colors"
        >
          <ExternalLink size={9} />
        </a>
      </div>
    </div>
  {/if}
</div>

<SslSettingsModal
  {proxy}
  isOpen={isSslSettingsOpen}
  onClose={() => (isSslSettingsOpen = false)}
/>
