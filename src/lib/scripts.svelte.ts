import { SvelteMap } from "svelte/reactivity";
import { taurpc } from "./rpc";
import { toast } from "./toast.svelte";
import type { ScriptConfig, ProxyEvent } from "$lib/types";
import type { ScriptResult } from "$lib/bindings";
import type { ProxyState } from "./proxy.svelte";

export class ScriptsState {
  enabled = $state(true);
  store = new Map<string, any>();
  list = $state<ScriptConfig[]>([]);
  syncStatus = $state<"idle" | "saving" | "saved" | "error">("idle");

  private modules = new SvelteMap<string, any>();
  private urls = new Map<string, string>();
  private patterns = new Map<string, RegExp>();
  private proxy: ProxyState;
  private compilationTimeout: any;
  private toastDebounce: any;
  private lastHydrateTime = Date.now();

  constructor(proxy: ProxyState) {
    this.proxy = proxy;

    // Compilation and Backend Sync Logic
    $effect(() => {
      // Track all dependencies deeply (JSON stringify ensures deep read)
      const _tracker = JSON.stringify(this.list);
      const isScriptingEnabled = this.enabled;

      clearTimeout(this.compilationTimeout);
      this.compilationTimeout = setTimeout(async () => {
        this.syncStatus = "saving";
        try {
          await this.compileAll();

          const deps = this.list.map(s => ({ id: s.id, code: s.code, enabled: s.enabled, pattern: s.pattern }));
          await this.syncBackend(isScriptingEnabled, deps);
          await this.proxy.saveSettings();

          this.syncStatus = "saved";

          // Debounce the success toast so it doesn't spam during active typing.
          clearTimeout(this.toastDebounce);

          const isInitial = Date.now() - this.lastHydrateTime < 2000;
          if (isInitial) {
            if (this.syncStatus === "saved") this.syncStatus = "idle";
          } else {
            this.toastDebounce = setTimeout(() => {
              toast.success("Script Saved");
              // Reset status after toast shows (or just keep it saved until next edit)
              if (this.syncStatus === "saved") this.syncStatus = "idle";
            }, 300);
          }
        } catch (e) {
          console.error("[Scripts] Sync effect failed:", e);
          this.syncStatus = "error";
        }
      }, 800);
    });
  }

  hydrate(scripts: ScriptConfig[], enabled: boolean) {
    this.lastHydrateTime = Date.now();
    this.list = scripts;
    this.enabled = enabled;
  }

  compileToRegex(filters: any[]): string {
    if (!filters || filters.length === 0) return ".*";

    const patterns = filters.map(f => {
      let proto = f.filterProtocol === "all" ? "[a-zA-Z]+" : f.filterProtocol;
      let host = f.filterHost ? this.toRegexString(f.filterHost) : ".*";

      let path = f.filterPath ? this.toRegexString(f.filterPath) : "(?:/.*)?";
      if (path === ".*") path = "(?:/.*)?";

      // Ensure path starts with / if specified and not already handled
      if (f.filterPath && !f.filterPath.startsWith("/") && !f.filterPath.startsWith("*")) {
        path = "/" + path;
      }
      // If path is just /, make it optional to match https://host
      if (path === "/") path = "(?:/)?";

      const query = f.filterQuery
        ? "\\?" + this.toRegexString(f.filterQuery)
        : "(\\?.*)?";

      // Port is always optional in our matching model
      const portMarker = "(?::\\d+)?";

      return `^${proto}://${host}${portMarker}${path}${query}$`;
    });

    return patterns.join("|");
  }

  private toRegexString(pattern: string): string {
    if (!pattern) return "";
    // If it's already a raw regex /.../, return the inside
    if (pattern.startsWith("/") && pattern.endsWith("/") && pattern.length > 2) {
      return pattern.slice(1, -1);
    }
    // Escape all regex special characters except *
    let escaped = pattern.replace(/[.+^${}()|[\]\\]/g, "\\$&");
    // Convert * to .*
    return escaped.replace(/\*/g, ".*");
  }

  private async syncBackend(enabled: boolean, scripts: any[]) {
    try {
      await taurpc.scripts.toggle_scripting(enabled);
      const activePatterns = enabled
        ? scripts.filter(s => s.enabled && s.pattern).map(s => s.pattern)
        : [];
      await taurpc.scripts.set_script_patterns(activePatterns);
    } catch (e: any) {
      console.error("[Scripts] Failed to sync with backend:", e);
      toast.error("Proxy sync failed");
      throw e;
    }
  }

  compilePattern(pattern: string): RegExp {
    try {
      return new RegExp(pattern, "i");
    } catch (e) {
      console.error("[Scripts] Pattern invalid:", pattern, e);
      return /$./i;
    }
  }

  private async compileAll() {
    for (const s of this.list) {
      // Re-compile pattern from multiple filters
      s.pattern = this.compileToRegex(s.filters);

      // Pre-compile pattern regex
      this.patterns.set(s.id, this.compilePattern(s.pattern));

      if (!s.enabled || !s.code) {
        this.modules.delete(s.id);
        const oldUrl = this.urls.get(s.id);
        if (oldUrl) { URL.revokeObjectURL(oldUrl); this.urls.delete(s.id); }
        s.compileError = undefined;
        continue;
      }

      const blob = new Blob([s.code], { type: "application/javascript" });
      const url = URL.createObjectURL(blob);
      try {
        const mod = await import(/* @vite-ignore */ url);
        const oldUrl = this.urls.get(s.id);
        if (oldUrl) URL.revokeObjectURL(oldUrl);
        this.urls.set(s.id, url);
        this.modules.set(s.id, mod);
        s.compileError = undefined;
      } catch (e: any) {
        URL.revokeObjectURL(url);
        s.compileError = e.message;
        this.proxy.log(`[System] ${s.name} compilation error: ${e.message}`, "0", "error");
        toast.error(`Compile Error (${s.name}): ${e.message}`, 5000);
      }
    }
  }

  async runScripts(event: ProxyEvent, isResponse: boolean): Promise<ScriptResult> {
    const result: ScriptResult = { dropped: false, headers: null, uri: null, status: null, body: null };
    if (!this.enabled) return result;

    // ── Body Channel Lock ──
    // All body accessors share a single backing store (currentBytes).
    // The channel lock prevents stale deep-proxy references from silently overwriting
    // body changes made through a different accessor.
    const originalBytes = event.body;
    let currentBytes = event.body;
    let bodyChannel: 'none' | 'json' | 'body' | 'raw' | 'formData' = 'none';
    let jsonProxyAlive = true; // invalidated when channel switches away from json
    let _mocked = false;
    let _dropped = false;

    const setBytes = (bytes: Uint8Array | null, channel: typeof bodyChannel) => {
      if (bodyChannel !== 'none' && bodyChannel !== channel) {
        // Channel switch — invalidate old proxies
        if (bodyChannel === 'json') jsonProxyAlive = false;
        this.proxy.log(`[System] Body channel switched: ${bodyChannel} → ${channel}`, event.id, "warn");
      }
      bodyChannel = channel;
      currentBytes = bytes;
    };

    // ── Headers Proxy (case-insensitive, syncs back to raw array) ──
    const headersRaw = event.headers;
    const headersTarget = Object.fromEntries(headersRaw);
    const headersProxy = new Proxy(headersTarget, {
      get: (target, prop: string) => {
        if (typeof prop !== "string") return undefined;
        const foundKey = Object.keys(target).find(k => k.toLowerCase() === prop.toLowerCase());
        return foundKey ? (target as any)[foundKey] : undefined;
      },
      set: (target, prop: string, value: string) => {
        if (typeof prop !== "string") return false;
        const foundKey = Object.keys(target).find(k => k.toLowerCase() === prop.toLowerCase());
        const keyToUse = foundKey || prop;
        (target as any)[keyToUse] = value;
        const idx = headersRaw.findIndex(([k]) => k.toLowerCase() === prop.toLowerCase());
        if (idx !== -1) {
          headersRaw[idx][1] = value;
        } else {
          headersRaw.push([prop, value]);
        }
        return true;
      },
      deleteProperty: (target, prop: string) => {
        if (typeof prop !== "string") return false;
        const foundKey = Object.keys(target).find(k => k.toLowerCase() === prop.toLowerCase());
        if (foundKey) delete (target as any)[foundKey];
        for (let i = headersRaw.length - 1; i >= 0; i--) {
          if (headersRaw[i][0].toLowerCase() === prop.toLowerCase()) {
            headersRaw.splice(i, 1);
          }
        }
        return true;
      },
      ownKeys: (target) => Object.keys(target),
      getOwnPropertyDescriptor: (target, prop) => Object.getOwnPropertyDescriptor(target, prop),
    });

    // ── URL Proxy (syncs changes back to ctx.uri) ──
    let currentUri = event.uri;
    const createUrlProxy = () => {
      try {
        const urlObj = new URL(currentUri);
        return new Proxy(urlObj, {
          get: (target, prop) => {
            const val = (target as any)[prop];
            if (typeof val === 'function') return val.bind(target);
            // Return searchParams directly (it's already mutable)
            return val;
          },
          set: (target, prop: string, value) => {
            (target as any)[prop] = value;
            currentUri = target.toString();
            return true;
          }
        });
      } catch {
        return null;
      }
    };

    // ── Build the Context Object (req/res) ──
    const ctx: any = {
      ...event,
      headers: headersProxy as any,
      rawHeaders: headersRaw,

      // ── uri (synced with url proxy) ──
      get uri() { return currentUri; },
      set uri(v: string) { currentUri = v; },

      // ── url — Parsed URL object ──
      get url() { return createUrlProxy(); },

      // ── contentType — Quick header check ──
      get contentType() {
        const ct = headersProxy['content-type'];
        return ct || null;
      },

      // ── body — String body (channel: 'body') ──
      get body() {
        if (!currentBytes) return null;
        try { return new TextDecoder().decode(currentBytes); }
        catch { return "[Binary Data]"; }
      },
      set body(v: string | null) {
        setBytes(v === null ? null : new TextEncoder().encode(v), 'body');
      },

      // ── text — Alias for body ──
      get text() { return this.body; },
      set text(v: string | null) { this.body = v; },

      // ── raw — Uint8Array body (channel: 'raw') ──
      get raw() { return currentBytes; },
      set raw(v: Uint8Array | null) { setBytes(v, 'raw'); },

      // ── json — Deep proxy with channel lock (channel: 'json') ──
      get json() {
        const b = this.body;
        if (!b || b === "[Binary Data]") return null;
        try {
          const obj = JSON.parse(b);
          const createDeepProxy = (target: any): any => {
            if (target !== null && typeof target === 'object') {
              return new Proxy(target, {
                get: (t, p) => {
                  if (p === Symbol.toPrimitive || p === 'toJSON') return undefined;
                  const val = t[p];
                  return (val !== null && typeof val === 'object') ? createDeepProxy(val) : val;
                },
                set: (t, p, v) => {
                  if (!jsonProxyAlive) return true; // dead proxy — no-op
                  t[p] = v;
                  setBytes(new TextEncoder().encode(JSON.stringify(obj)), 'json');
                  return true;
                }
              });
            }
            return target;
          };
          return createDeepProxy(obj);
        } catch { return null; }
      },
      set json(v: any) {
        setBytes(new TextEncoder().encode(JSON.stringify(v)), 'json');
      },

      // ── formData — URLSearchParams for form-encoded bodies (channel: 'formData') ──
      get formData() {
        const ct = this.contentType;
        if (!ct || !ct.includes("x-www-form-urlencoded")) return null;
        const b = this.body;
        if (!b) return null;
        try {
          const params = new URLSearchParams(b);
          // Wrap to auto-serialize on mutation
          return new Proxy(params, {
            get: (target, prop) => {
              const val = (target as any)[prop];
              if (typeof val !== 'function') return val;
              return (...args: any[]) => {
                const ret = val.apply(target, args);
                // After any mutating call, sync back
                if (['set', 'append', 'delete', 'sort'].includes(prop as string)) {
                  setBytes(new TextEncoder().encode(target.toString()), 'formData');
                }
                return ret;
              };
            }
          });
        } catch { return null; }
      },

      // ── status (writable on responses) ──
      get status() { return event.status; },
      set status(v: number | null) { event.status = v; },

      // ── method (read-only) ──
      get method() { return event.method; },

      // ── is_response (read-only) ──
      get is_response() { return event.is_response; },

      // ── id (read-only) ──
      get id() { return event.id; },
    };

    // ── Build the Proxy Helper Object ──
    const sessionProxy = {
      log: (msg: any, level?: any) => this.proxy.log(msg, event.id, level || "info"),

      /** Return a mock response — request never reaches the server */
      mock: (status: number, body?: any, headers?: Record<string, string>) => {
        _mocked = true;
        event.status = status;
        if (body !== undefined) {
          if (typeof body === 'object') {
            setBytes(new TextEncoder().encode(JSON.stringify(body)), 'body');
            headersProxy['content-type'] = 'application/json';
          } else {
            setBytes(new TextEncoder().encode(String(body)), 'body');
          }
        }
        if (headers) {
          for (const [k, v] of Object.entries(headers)) {
            headersProxy[k] = v;
          }
        }
      },

      /** Simulate network latency (real delay — holds the connection open).
       *  NOTE: Uses browser setTimeout for now. If script execution moves to a
       *  backend runtime (e.g. QuickJS), swap this for a native host function
       *  that calls tokio::time::sleep. The user-facing API stays the same. */
      delay: (ms: number) => new Promise<void>(r => setTimeout(r, Math.min(ms, 30000))),

      /** Drop the request silently (returns 403 to client) */
      drop: () => { _dropped = true; },

      /** Persistent key-value store shared across all scripts and requests */
      store: {
        get: (key: string) => this.store.get(key),
        set: (key: string, value: any) => {
          this.store.set(key, value);
          try { localStorage.setItem("proxy_script_store", JSON.stringify([...this.store])); } catch { }
        },
        delete: (key: string) => {
          this.store.delete(key);
          try { localStorage.setItem("proxy_script_store", JSON.stringify([...this.store])); } catch { }
        },
        has: (key: string) => this.store.has(key),
        clear: () => {
          this.store.clear();
          try { localStorage.setItem("proxy_script_store", "[]"); } catch { }
        },
      },
    };

    // ── Execute Scripts ──
    for (const s of this.list) {
      if (!s.enabled) continue;
      if (_mocked || _dropped) break; // stop executing further scripts

      const mod = this.modules.get(s.id);
      if (!mod) continue;

      const regex = this.patterns.get(s.id);
      if (regex && !regex.test(event.uri)) continue;

      try {
        const handler = isResponse ? mod.onResponse : mod.onRequest;
        const afterHandler = isResponse ? mod.afterResponse : null;

        if (handler) await handler(ctx, sessionProxy);

        if (afterHandler) {
          afterHandler(ctx, sessionProxy).catch((e: any) => {
            this.proxy.log(`[${s.name}] afterResponse error: ${e.message}`, event.id, "error");
          });
        }
      } catch (e: any) {
        this.proxy.log(`[${s.name}] script error: ${e.message}`, event.id, "error");
      }
    }

    // ── Sync Results ──
    if (_dropped) {
      result.dropped = true;
      return result;
    }

    if (currentBytes !== originalBytes) {
      result.body = currentBytes ? Array.from(currentBytes) : null;
    }

    result.headers = [...headersRaw];
    result.uri = currentUri !== event.uri ? currentUri : null;
    result.status = (event.status !== undefined && event.status !== null) ? event.status : null;

    // For mock responses from onRequest, status must be set
    if (_mocked && !isResponse) {
      result.status = event.status !== null ? event.status : 200;
    }

    return result;
  }

  addScript() {
    const id = Math.random().toString(36).slice(2);
    this.list.push({
      id,
      name: "New Script " + (this.list.length + 1),
      filters: [{
        id: Math.random().toString(36).slice(2),
        filterProtocol: "all",
        filterHost: "",
        filterPort: "",
        filterPath: "",
        filterQuery: ""
      }],
      code: DEFAULT_SCRIPT,
      enabled: false,
      pattern: ".*",
      description: "",
      compileError: undefined,
    });
  }

  removeScript(id: string) {
    this.list = this.list.filter(s => s.id !== id);
    this.modules.delete(id);
    this.patterns.delete(id);
    const url = this.urls.get(id);
    if (url) { URL.revokeObjectURL(url); this.urls.delete(id); }
  }
}

const DEFAULT_SCRIPT = `export async function onRequest(req, proxy) {
  // ── URL & Headers ──
  // proxy.log(req.url.hostname);
  // req.url.searchParams.set("debug", "true");
  // req.headers['X-Custom'] = 'value';

  // ── Body (JSON, form data, or raw text) ──
  // if (req.json) req.json.modified = true;
  // if (req.formData) req.formData.set("key", "val");
  // req.body = "raw text override";

  // ── Mock / Delay / Drop ──
  // proxy.mock(200, { mocked: true });
  // await proxy.delay(1000);
  // proxy.drop();

  // ── Cross-request storage ──
  // proxy.store.set("token", "abc");
}

export async function onResponse(res, proxy) {
  // res.status = 200;
  // if (res.json) res.json.injected = true;
  // proxy.log(res.contentType);
}`