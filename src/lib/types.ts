export type ProxyEvent = {
  id: number;
  script_id: number;
  timestamp: number;
  method: string;
  uri: string;
  headers: [string, string][];
  is_response: boolean;
  status: number | null;
  body_base64: string | null;
};

export type ScriptResult = {
  headers?: [string, string][];
  body_base64?: string;
  status?: number;
  uri?: string;
  dropped: boolean;
};

export type ScriptConfig = {
  id: string;
  name: string;
  pattern: string; // Regex string
  code: string;
  enabled: boolean;
  compileError?: string;
};

export type ScriptLog = {
  id: string; // Unique log ID
  requestId: number; // The request this log belongs to
  message: any;
  level: "info" | "error" | "warn";
  timestamp: number;
};
