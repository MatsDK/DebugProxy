// src/lib/utils.ts

export function statusColor(status: number | null): string {
  if (!status) return "text-slate-400";
  if (status < 300) return "text-emerald-500";
  if (status < 400) return "text-amber-500";
  return "text-red-500";
}

export function methodColor(m: string): string {
  return (
    ({
      GET: "text-blue-500",
      POST: "text-emerald-500",
      PUT: "text-amber-500",
      DELETE: "text-red-500",
    } as Record<string, string>)[m.toUpperCase()] ?? "text-slate-400"
  );
}

export function pathOnly(uri: string): string {
  if (!uri.includes("://") && uri.includes(":")) return "";
  try {
    const u = new URL(uri);
    return u.pathname + u.search;
  } catch {
    return uri;
  }
}

export function domainOnly(uri: string): string {
  if (!uri.includes("://") && uri.includes(":")) return uri.split(":")[0];
  try {
    return new URL(uri).hostname;
  } catch {
    return uri;
  }
}

export function formatSize(bytes: Uint8Array | null): string {
  if (!bytes) return "–";
  const n = bytes.length;
  if (n < 1024) return n + " B";
  if (n < 1048576) return (n / 1024).toFixed(1) + " KB";
  return (n / 1048576).toFixed(1) + " MB";
}

export function formatTime(ms: number | undefined): string {
  if (!ms) return "–";
  const d = new Date(ms);
  return (
    d.toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    }) +
    "." +
    String(d.getMilliseconds()).padStart(3, "0")
  );
}

export function formatDuration(t0: number | undefined, t1: number | undefined): string {
  if (!t0 || !t1) return "–";
  const ms = t1 - t0;
  if (ms < 1000) return ms + "ms";
  return (ms / 1000).toFixed(2) + "s";
}

export function normalizeUri(uri: string): string {
  try {
    const url = new URL(uri);
    return url.href;
  } catch {
    return uri;
  }
}

export function buildCurl(req: any): string {
  const cleanUrl = normalizeUri(req.uri);
  const headers = req.headers
    .map(([k, v]: [string, string]) => `-H "${k}: ${v.replace(/"/g, '\\"')}"`)
    .join(" \\\n  ");
  const bodyText = req.body_base64 ? atob(req.body_base64) : "";
  const body = bodyText ? `-d '${bodyText.replace(/'/g, "'\\''")}'` : "";
  return `curl -X ${req.method} "${cleanUrl}" \\\n  ${headers}${body ? " \\\n  " + body : ""}`;
}
