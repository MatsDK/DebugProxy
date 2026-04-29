import type { ProxyEvent as GeneratedProxyEvent, ScriptResult as GeneratedScriptResult } from "./bindings";

export type ProxyEvent = GeneratedProxyEvent & {
  // Script helpers added at runtime
  json?: any;
};

export type ScriptResult = GeneratedScriptResult & {
  // Script helpers added at runtime
  json?: any;
};

export type { FilterConfig, ScriptConfig } from "./bindings";

export type ScriptLog = {
  id: string; // Unique log ID
  requestId: string; // The request this log belongs to
  message: any;
  level: "info" | "error" | "warn";
  timestamp: number;
};

export type CtxMenu = { x: number; y: number; id: string } | null;
