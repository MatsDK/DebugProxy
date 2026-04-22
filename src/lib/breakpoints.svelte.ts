import { SvelteMap } from "svelte/reactivity";
import type { ProxyEvent } from "$lib/types";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { taurpc } from "./rpc";

export interface PendingBreakpoint {
  id: string;
  type: 'request' | 'response';
  event: ProxyEvent;
  resolve: (modifiedEvent: ProxyEvent | null) => void;
  timestamp: number;
}

export class BreakpointState {
  // requestId -> PendingBreakpoint
  active = new SvelteMap<string, PendingBreakpoint>();
  
  // Total count of active breakpoints (useful for badges)
  count = $derived(this.active.size);

  private isMain = false;
  private label = "";

  constructor() {
    if (typeof window !== 'undefined') {
      const win = getCurrentWindow();
      this.label = win.label;
      this.isMain = this.label === "main";
      console.log(`[BreakpointState] Initializing (wid: "${this.label}", isMain: ${this.isMain})`);
      this.setupListeners().catch(e => console.error("[BreakpointState] Listener setup failed:", e));
    }
  }

  private async setupListeners() {
    console.log(`[BreakpointState] Setting up listeners (wid: ${this.label})...`);
    
    // Register all listeners concurrently to avoid sequential blocking
    const registrations = [
       (taurpc.events as any).breakpoint_hit.on((id: string, type: string, event: any) => {
         console.log(`[BreakpointState] EVENT: breakpoint_hit (wid: ${this.label})`, id);
         if (this.isMain) return;
         if (this.active.has(id)) return;

         this.active.set(id, {
           id,
           type: type as any,
           event: {
             ...event,
             headers: event.headers as [string, string][],
             body: event.body ? new Uint8Array(event.body) : null
           } as any as ProxyEvent,
           resolve: (modified) => {
             console.log("[BreakpointState] Resolution triggered on client", id);
             taurpc.submit_breakpoint_resolution(id, modified as any);
           },
           timestamp: Date.now()
         });
       }),

       (taurpc.events as any).breakpoint_resolved_signal.on((id: string, modifiedEvent: any) => {
         console.log(`[BreakpointState] EVENT: breakpoint_resolved_signal (wid: ${this.label})`, id);
         if (this.isMain) {
           this.resolve(id, modifiedEvent);
         } else {
           this.active.delete(id);
         }
       }),

       (taurpc.events as any).sync_requested.on(() => {
         console.log(`[BreakpointState] EVENT: sync_requested (received by: ${this.label}, isMain: ${this.isMain})`);
         if (!this.isMain) return;
         
         const activeBps = Array.from(this.active.values());
         console.log(`[BreakpointState] Master responding to sync. ${activeBps.length} active BPs.`);
         for (const bp of activeBps) {
           try {
             const payload = this.toSerializable(bp);
             taurpc.emit_breakpoint_hit(payload.id, payload.type, payload.event as any).catch(e => {
               console.error("[BreakpointState] Failed to re-emit breakpoint during sync:", e);
             });
           } catch (err) {
             console.error("[BreakpointState] Serialization error during sync:", err);
           }
         }
       }),

       (taurpc.events as any).window_closed.on((label: string) => {
         if (label === "interceptor") {
           console.log("[BreakpointState] Interceptor window closed");
         }
       })
    ];

    await Promise.all(registrations);
    console.log(`[BreakpointState] All listeners registered (wid: ${this.label})`);

    if (!this.isMain) {
       // Request sync immediately and with retries
       this.triggerSync();
       setTimeout(() => this.triggerSync(), 500);
       setTimeout(() => this.triggerSync(), 2000);
       setTimeout(() => this.triggerSync(), 5000);
    }
  }

  private triggerSync() {
    if (this.isMain) return;
    console.log("[BreakpointState] Client window requesting sync via taurpc...");
    taurpc.trigger_breakpoint_sync().catch(e => {
       console.error("[BreakpointState] Sync trigger failed:", e);
    });
  }

  public toSerializable(bp: PendingBreakpoint) {
    const ctx = bp.event as any;
    // CRITICAL: We must explicitly pull properties because 'ctx' is a Proxy (or might be)
    let bodyData: number[] | null = null;

    // Handle both Proxy context (from script) and plain serialized object
    const rawBody = ctx.raw !== undefined ? ctx.raw : ctx.body;
    
    if (rawBody) {
      if (rawBody instanceof Uint8Array || (typeof rawBody === 'object' && rawBody.buffer)) {
        bodyData = Array.from(rawBody);
      } else if (Array.isArray(rawBody)) {
        bodyData = rawBody;
      } else if (typeof rawBody === "string") {
        bodyData = Array.from(new TextEncoder().encode(rawBody));
      }
    }

    const serializableEvent = {
      id: String(ctx.id),
      method: ctx.method,
      uri: ctx.uri,
      headers: ctx.rawHeaders
        ? [...ctx.rawHeaders]
        : Array.isArray(ctx.headers)
          ? ctx.headers
          : Object.entries(ctx.headers || {}),
      status: ctx.status,
      timestamp: String(ctx.timestamp),
      script_id: String(ctx.script_id || "0"),
      is_response: !!ctx.is_response,
      body: bodyData,
    };

    return {
      id: String(bp.id),
      type: bp.type,
      timestamp: bp.timestamp,
      event: serializableEvent as any as ProxyEvent,
    };
  }

  add(breakpoint: PendingBreakpoint) {
    console.log(`[BreakpointState] Adding breakpoint locally (wid: ${this.label}, isMain: ${this.isMain}):`, breakpoint.id);
    this.active.set(breakpoint.id, breakpoint);

    if (this.isMain) {
      const payload = this.toSerializable(breakpoint);
      console.log("[BreakpointState] Master window broadcasting hit via taurpc...");
      taurpc.emit_breakpoint_hit(
        payload.id,
        payload.type,
        payload.event as any,
      );
    }
  }

  resolve(id: string, modifiedEvent: any | null) {
    const bp = this.active.get(id);
    if (!bp) return;

    console.log(`[BreakpointState] resolve() called (wid: ${this.label}, isMain: ${this.isMain})`, id);
    
    if (this.isMain) {
      // If the user modified the event via the UI, it's often the SAME proxy object.
      // We only need to apply fields if it's a DIFFERENT object (e.g. from a detached window).
      if (modifiedEvent && modifiedEvent !== bp.event) {
        const ctx = bp.event as any;
        if (modifiedEvent.uri) ctx.uri = modifiedEvent.uri;
        if (modifiedEvent.status !== undefined) ctx.status = modifiedEvent.status;
        
        if (modifiedEvent.headers) {
           ctx.headers = modifiedEvent.headers;
        }

        if (modifiedEvent.body) {
          if (modifiedEvent.body instanceof Uint8Array) {
            ctx.raw = modifiedEvent.body;
          } else if (Array.isArray(modifiedEvent.body)) {
            ctx.raw = new Uint8Array(modifiedEvent.body);
          } else if (typeof modifiedEvent.body === "string") {
            ctx.body = modifiedEvent.body;
          }
        }
      }
      
      bp.resolve(modifiedEvent || null); 
      this.active.delete(id);
    } else {
      // Client window: signal back to master
      console.log("[BreakpointState] Client window signaling resolution back to master", id);
      const payload = this.toSerializable(bp);
      taurpc.submit_breakpoint_resolution(id, payload.event as any).catch(e => {
        console.error("[BreakpointState] Failed to submit resolution:", e);
      });
      this.active.delete(id);
    }
  }

  drop(id: string) {
    this.resolve(id, null);
  }
}

// Global singleton
export const breakpointState = new BreakpointState();
