import ts from "typescript";

// Ambient types describing the `req`/`res` context and `proxy` helper object built at
// runtime in ScriptsState.runScripts(). No import/export here on purpose — this makes it
// a global script (not a module), so every user script sees these names without importing.
const RUNTIME_LIB_NAME = "debugproxy-runtime.d.ts";
const RUNTIME_LIB_SOURCE = `
interface ScriptStore {
  get(key: string): any;
  set(key: string, value: any): void;
  delete(key: string): void;
  has(key: string): boolean;
  clear(): void;
}

interface ScriptProxy {
  /** Log a message, tagged to this request, shown in the Inspector. */
  log(message: any, level?: "info" | "warn" | "error"): void;
  /** Short-circuit the request with a mock response — it never reaches the server. */
  mock(status: number, body?: any, headers?: Record<string, string>): void;
  /** Hold the connection open for \`ms\` milliseconds (capped at 30s). */
  delay(ms: number): Promise<void>;
  /** Drop the request silently; the client receives a 403. */
  drop(): void;
  /** Key-value store persisted across requests and scripts. */
  store: ScriptStore;
  /** Pause and wait for manual edits in the Interceptor UI. */
  breakpoint(): Promise<void>;
}

interface ProxyEventContext {
  readonly id: string;
  readonly script_id: string;
  readonly timestamp: string;
  readonly method: string;
  readonly is_response: boolean;
  uri: string;
  status: number | null;
  readonly url: URL | null;
  readonly contentType: string | null;
  headers: Record<string, string>;
  rawHeaders: [string, string][];
  body: string | null;
  text: string | null;
  raw: Uint8Array | null;
  json: any;
  readonly formData: URLSearchParams | null;
}

type Req = ProxyEventContext;
type Res = ProxyEventContext;
`;

const VIRTUAL_FILE = "script.ts";

const compilerOptions: ts.CompilerOptions = {
  target: ts.ScriptTarget.ES2022,
  module: ts.ModuleKind.ESNext,
  moduleResolution: ts.ModuleResolutionKind.Bundler,
  strict: false,
  noEmit: true,
};

// Vite statically detects this exact `import.meta.glob(...)` call shape and rewrites it
// into real imports — casting/wrapping `import.meta` itself breaks that detection, so the
// call must stay untouched and only the result gets cast. `exhaustive` is required too:
// Vite silently excludes node_modules from glob matches unless you opt back in.
const libLoaders = import.meta.glob("/node_modules/typescript/lib/lib.*.d.ts", {
  query: "?raw",
  import: "default",
  exhaustive: true,
}) as Record<string, () => Promise<string>>;

const libFiles = new Map<string, string>();
let libsReady: Promise<void> | null = null;

// Loaded in small batches, not all ~90 files at once — WebKitGTK's dev-server networking
// is flaky under a big burst of simultaneous requests.
const LIB_LOAD_BATCH_SIZE = 8;

async function loadAllLibs(): Promise<void> {
  const entries = Object.entries(libLoaders);
  for (let i = 0; i < entries.length; i += LIB_LOAD_BATCH_SIZE) {
    await Promise.all(
      entries.slice(i, i + LIB_LOAD_BATCH_SIZE).map(async ([path, load]) => {
        libFiles.set(path.split("/").pop()!, await load());
      }),
    );
  }
}

function ensureLibsLoaded(): Promise<void> {
  if (!libsReady) {
    libsReady = loadAllLibs();
  }
  return libsReady;
}

let source = "";
let version = 0;

function setSource(code: string) {
  if (code !== source) {
    source = code;
    version++;
  }
}

const host: ts.LanguageServiceHost = {
  getScriptFileNames: () => [VIRTUAL_FILE, RUNTIME_LIB_NAME],
  getScriptVersion: (fileName) => (fileName === VIRTUAL_FILE ? String(version) : "1"),
  getScriptSnapshot: (fileName) => {
    const text = readFile(fileName);
    return text !== undefined ? ts.ScriptSnapshot.fromString(text) : undefined;
  },
  getCurrentDirectory: () => "/",
  getCompilationSettings: () => compilerOptions,
  getDefaultLibFileName: () => "lib.es2022.full.d.ts",
  fileExists: (fileName) => readFile(fileName) !== undefined,
  readFile,
  directoryExists: () => true,
  getDirectories: () => [],
};

function readFile(fileName: string): string | undefined {
  if (fileName === VIRTUAL_FILE) return source;
  if (fileName === RUNTIME_LIB_NAME) return RUNTIME_LIB_SOURCE;
  return libFiles.get(fileName.split("/").pop()!);
}

const languageService = ts.createLanguageService(host, ts.createDocumentRegistry());

export type ScriptDiagnostic = { message: string; line: number; start: number; length: number };

export async function getScriptDiagnostics(code: string): Promise<ScriptDiagnostic[]> {
  await ensureLibsLoaded();
  setSource(code);
  const diagnostics = [
    ...languageService.getSyntacticDiagnostics(VIRTUAL_FILE),
    ...languageService.getSemanticDiagnostics(VIRTUAL_FILE),
  ];
  return diagnostics.map((d) => ({
    message: ts.flattenDiagnosticMessageText(d.messageText, "\n"),
    line: d.file && d.start !== undefined ? d.file.getLineAndCharacterOfPosition(d.start).line + 1 : 0,
    start: d.start ?? 0,
    length: d.length ?? 1,
  }));
}

export async function getScriptCompletions(code: string, position: number): Promise<ts.CompletionEntry[]> {
  await ensureLibsLoaded();
  setSource(code);
  const completions = languageService.getCompletionsAtPosition(VIRTUAL_FILE, position, {});
  return completions?.entries ?? [];
}

export type ScriptQuickInfo = { text: string; start: number; length: number };

export async function getScriptQuickInfo(code: string, position: number): Promise<ScriptQuickInfo | null> {
  await ensureLibsLoaded();
  setSource(code);
  const info = languageService.getQuickInfoAtPosition(VIRTUAL_FILE, position);
  if (!info) return null;

  const signature = ts.displayPartsToString(info.displayParts);
  const docs = ts.displayPartsToString(info.documentation);
  return {
    text: docs ? `${signature}\n\n${docs}` : signature,
    start: info.textSpan.start,
    length: info.textSpan.length,
  };
}

export { ts };
