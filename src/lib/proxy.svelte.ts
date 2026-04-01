import { SvelteMap } from "svelte/reactivity";
import { listen } from "@tauri-apps/api/event";
import { createTauRPCProxy } from "$lib/bindings";
import type { ScriptResult } from "$lib/bindings";
import type { ProxyEvent, ScriptLog, ScriptConfig } from "$lib/types";

export const taurpc = createTauRPCProxy();

export class ProxyState {
  // Connection state
  ip = $state("Detecting...");
  port = $state(8080);
  isRunning = $state(false);
  errorMsg = $state("");
  interceptSsl = $state(true);

  // Request/response traffic
  reqMap = new SvelteMap<number, ProxyEvent>();
  resMap = new SvelteMap<number, ProxyEvent>();
  orderedIds = $state<number[]>([]);
  reqTime = new SvelteMap<number, number>();
  resTime = new SvelteMap<number, number>();

  // Script state
  scriptLogs = $state<ScriptLog[]>([]);

  log(msg: any, requestId: number, level: "info" | "warn" | "error" = "info") {
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
      this.interceptSsl = await taurpc.is_ssl_intercept_enabled();
    } catch {}

    const savedPort = localStorage.getItem("proxy_port");
    if (savedPort) this.port = parseInt(savedPort);

    try {
      await taurpc.start_proxy(this.port);
      this.isRunning = true;
    } catch (e: any) {
      if (e === "Proxy is already running") {
        this.isRunning = true;
      } else {
        this.errorMsg = "Auto-start failed: " + e;
      }
    }

    await this.setupListeners();
  }

  async startProxy() {
    this.errorMsg = "";
    try {
      await taurpc.start_proxy(this.port);
      this.isRunning = true;
      localStorage.setItem("proxy_port", String(this.port));
    } catch (e: any) {
      this.errorMsg = e.toString();
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

  async exportCert(): Promise<string | null> {
    try {
      return await taurpc.get_ca_cert();
    } catch (e: any) {
      this.errorMsg = e.toString();
      return null;
    }
  }

  async setScriptPatterns(scripts: ScriptConfig[]) {
    const patterns = scripts.filter((s) => s.enabled && s.pattern).map((s) => s.pattern);
    await taurpc.scripts.set_script_patterns(patterns).catch(console.error);
  }

  async setScriptingEnabled(enabled: boolean) {
    await taurpc.scripts.toggle_scripting(enabled).catch(console.error);
  }

  async submitScriptResult(scriptId: number, result: ScriptResult) {
    await taurpc.scripts.submit_script_result(String(scriptId), result).catch(() => {});
  }

  clearTraffic() {
    this.orderedIds = [];
    this.reqMap.clear();
    this.resMap.clear();
  }

  private async setupListeners() {
    await listen<ProxyEvent>("proxy-event", (event) => {
      const e = event.payload;
      if (!e.is_response) {
        this.reqMap.set(e.id, e);
        this.reqTime.set(e.id, Date.now());
        this.orderedIds.push(e.id);
      } else {
        this.resMap.set(e.id, e);
        this.resTime.set(e.id, Date.now());
      }
    });
  }
}
