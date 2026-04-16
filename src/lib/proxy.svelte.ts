import { SvelteMap } from "svelte/reactivity";
import { listen } from "@tauri-apps/api/event";
import { taurpc } from "./rpc";
import type { ProxyEvent, ScriptLog } from "$lib/types";
import { ScriptsState } from "./scripts.svelte";

export class ProxyState {
  ip = $state("Detecting...");
  port = $state(8080);
  isRunning = $state(false);
  errorMsg = $state("");
  interceptSsl = $state(true);
  isBlocked = $state(false);

  reqMap = new SvelteMap<string, ProxyEvent>();
  resMap = new SvelteMap<string, ProxyEvent>();
  orderedIds = $state<string[]>([]);
  private idSet = new Set<string>();

  reqTime = new SvelteMap<string, number>();
  resTime = new SvelteMap<string, number>();

  scripts: ScriptsState;
  scriptLogs = $state<ScriptLog[]>([]);

  constructor() {
    this.scripts = new ScriptsState(this);

    this.scripts = new ScriptsState(this);

    // Cross-window synchronization (local browser broadcast)
    if (typeof window !== "undefined") {
      window.addEventListener("storage", (e) => {
        if (e.key === "proxy_settings_sync" && e.newValue) {
          const payload = JSON.parse(e.newValue);
          this.port = payload.port;
          this.interceptSsl = payload.interceptSsl;
          this.isBlocked = payload.isBlocked;
          
          // Hydrate scripts ONLY if deeply changed, to avoid reactivity loops
          if (JSON.stringify(this.scripts.list) !== JSON.stringify(payload.scripts)) {
            this.scripts.hydrate(payload.scripts, true);
          }
        }
      });
    }
  }

  log(msg: any, requestId: string, level: "info" | "warn" | "error" = "info") {
    this.scriptLogs.push({
      id: Math.random().toString(36).slice(2),
      requestId,
      level,
      message: msg,
      timestamp: Date.now(),
    });
    if (this.scriptLogs.length > 2000) this.scriptLogs.shift();
  }

  async init() {
    try {
      this.ip = await taurpc.get_local_ip();
    } catch {
      this.ip = "127.0.0.1";
    }

    try {
      const settings = await taurpc.get_settings();
      this.port = settings.port;
      this.interceptSsl = settings.interceptSsl;
      this.isBlocked = settings.isBlocked;
      this.scripts.hydrate(settings.scripts, true);
    } catch (e) {
      console.error("[Proxy] Failed to load settings:", e);
    }

    try {
      await taurpc.start_proxy(this.port);
      this.isRunning = true;
    } catch (e: any) {
      if (e === "Proxy is already running") {
        this.isRunning = true;
      } else {
        const msg = String(e);
        if (msg.includes("AddrInUse") || msg.toLowerCase().includes("address already in use")) {
          this.errorMsg = `Auto-start failed: Port ${this.port} is already in use.`;
        } else if (msg.includes("PermissionDenied") || msg.toLowerCase().includes("permission denied")) {
          this.errorMsg = `Auto-start failed: Insufficient permissions for port ${this.port}.`;
        } else {
          this.errorMsg = "Auto-start failed: " + msg;
        }
        console.error(`[Proxy] Auto-start error:`, e);
      }
    }

    await this.setupListeners();
  }

  async startProxy() {
    this.port = Number(this.port);
    this.errorMsg = "";
    try {
      await taurpc.start_proxy(this.port);
      this.isRunning = true;
      this.saveSettings();
    } catch (e: any) {
      const msg = String(e);
      if (msg.includes("AddrInUse") || msg.toLowerCase().includes("address already in use")) {
        this.errorMsg = `Port ${this.port} is already in use. Please close the other application or choose a different port.`;
      } else if (msg.includes("PermissionDenied") || msg.toLowerCase().includes("permission denied")) {
        this.errorMsg = `Insufficient permissions to bind to port ${this.port}. Please check your AppImage/Sandbox settings or try a port > 1024.`;
      } else {
        this.errorMsg = msg;
      }
    }
  }

  async stopProxy() {
    this.errorMsg = "";
    try {
      await taurpc.stop_proxy();
      this.isRunning = false;
    } catch (e: any) {
      this.errorMsg = e.toString();
    }
  }

  async toggleProxy() {
    if (this.isRunning) await this.stopProxy();
    else await this.startProxy();
  }

  async toggleSsl(enabled: boolean) {
    try {
      await taurpc.toggle_ssl_intercept(enabled);
      if (this.isRunning) {
        await taurpc.stop_proxy();
        await new Promise((r) => setTimeout(r, 100));
        await taurpc.start_proxy(this.port);
      }
    } catch (e: any) {
      this.errorMsg = "Failed to toggle SSL intercept: " + e;
    }
  }

  async toggleBlocked(enabled: boolean) {
    try {
      await taurpc.toggle_blocked(enabled);
      this.isBlocked = enabled;
    } catch (e: any) {
      this.errorMsg = "Failed to toggle block mode: " + e;
    }
  }

  async exportCert(): Promise<string | null> {
    try {
      return await taurpc.get_ca_cert();
    } catch (e: any) {
      this.errorMsg = e.toString();
      return null;
    }
  }

  async saveSettings() {
    this.port = Number(this.port);
    try {
      const payload = {
        port: this.port,
        interceptSsl: this.interceptSsl,
        isBlocked: this.isBlocked,
        theme: document.documentElement.classList.contains("dark") ? "dark" : "light",
        scripts: $state.snapshot(this.scripts.list)
      };
      
      // Native fast cross-window synchronization broadcast (ignores source window)
      if (typeof window !== "undefined") {
        localStorage.setItem("proxy_settings_sync", JSON.stringify(payload));
      }

      await taurpc.save_settings(payload);
    } catch (e) {
      console.error("[Proxy] Failed to sync settings to runtime:", e);
    }
  }

  clearTraffic() {
    this.orderedIds = [];
    this.idSet.clear();
    this.reqMap.clear();
    this.resMap.clear();
    this.reqTime.clear();
    this.resTime.clear();
  }

  private async setupListeners() {
    await listen<ProxyEvent>("proxy-event", async (event) => {
      const e = event.payload;
      if (e.body) e.body = new Uint8Array(e.body);
      const id = String(e.id);

      // Update maps and times
      if (!e.is_response) {
        this.reqMap.set(id, e);
        this.reqTime.set(id, Number(e.timestamp));
        if (!this.idSet.has(id)) {
          this.idSet.add(id);
          this.orderedIds.push(id);
        }
      } else {
        this.resMap.set(id, e);
        this.resTime.set(id, Number(e.timestamp));
      }

      if (e.script_id !== "0") {
        try {
          const result = await this.scripts.runScripts(e, e.is_response);
          
          // Update the local event with script modifications so the UI reflects changes
          if (result.body !== null) e.body = new Uint8Array(result.body);
          if (result.headers) e.headers = result.headers;
          if (result.uri) e.uri = result.uri;
          if (result.status !== undefined && result.status !== null) e.status = result.status;

          // Re-set in map to trigger reactivity if properties were modified
          if (e.is_response) {
            this.resMap.set(id, { ...e });
          } else {
            this.reqMap.set(id, { ...e });
          }

          await taurpc.scripts.submit_script_result(e.script_id, result);
        } catch (err) {
          console.error(`[Proxy] Script runtime error for event ${id}:`, err);
          await taurpc.scripts.submit_script_result(e.script_id, {
            dropped: false,
            headers: null,
            uri: null,
            status: null,
            body: null
          }).catch(() => { });
        }
      }
    });
  }
}
