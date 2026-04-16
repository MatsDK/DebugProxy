export type ProxyEvent = {
  id: string;
  script_id: string;
  timestamp: string;
  method: string;
  uri: string;
  headers: [string, string][];
  is_response: boolean;
  status: number | null;
  body: Uint8Array | null;
  
  // Script helpers
  json?: any;
};

export type ScriptResult = {
  headers?: [string, string][];
  body?: number[] | null;
  status?: number | null;
  uri?: string | null;
  dropped: boolean;
  
  // Script helpers
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

