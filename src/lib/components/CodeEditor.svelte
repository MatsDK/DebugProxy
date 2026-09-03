<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, keymap, lineNumbers, highlightActiveLineGutter, highlightSpecialChars, drawSelection, dropCursor, highlightActiveLine, hoverTooltip } from "@codemirror/view";
  import { EditorState, Compartment } from "@codemirror/state";
  import { javascript } from "@codemirror/lang-javascript";
  import { oneDark } from "@codemirror/theme-one-dark";
  import {
    defaultKeymap, history, historyKeymap,
    indentWithTab
  } from "@codemirror/commands";
  import {
    indentOnInput, syntaxHighlighting, defaultHighlightStyle,
    bracketMatching, foldGutter, indentUnit
  } from "@codemirror/language";
  import { closeBrackets, closeBracketsKeymap, autocompletion, completionKeymap, startCompletion, type CompletionContext, type CompletionResult } from "@codemirror/autocomplete";
  import { linter, lintGutter, type Diagnostic } from "@codemirror/lint";
  import { getScriptCompletions, getScriptDiagnostics, getScriptQuickInfo, ts } from "$lib/scriptLanguageService";

  type Props = {
    value: string;
    onchange: (val: string) => void;
    darkMode?: boolean;
  };

  let { value, onchange, darkMode = false }: Props = $props();

  let container: HTMLDivElement | null = null;
  let view: EditorView | null = null;
  let themeCompartment = new Compartment();
  let updating = false;

  function buildTheme(dark: boolean) {
    return dark ? oneDark : EditorView.theme({
      "&": { background: "transparent", color: "#0f172a" },
      ".cm-content": { caretColor: "#6366f1" },
      ".cm-gutters": { background: "#f8fafc", borderRight: "1px solid #e2e8f0", color: "#94a3b8" },
      ".cm-activeLineGutter": { background: "#eff6ff" },
      ".cm-activeLine": { background: "#eff6ff50" },
      ".cm-selectionBackground, ::selection": { background: "#c7d2fe !important" },
      ".cm-cursor": { borderLeftColor: "#6366f1" },
    });
  }

  function completionKindToType(kind: string): string {
    switch (kind) {
      case ts.ScriptElementKind.functionElement:
        return "function";
      case ts.ScriptElementKind.memberFunctionElement:
        return "method";
      case ts.ScriptElementKind.classElement:
        return "class";
      case ts.ScriptElementKind.interfaceElement:
      case ts.ScriptElementKind.typeElement:
      case ts.ScriptElementKind.alias:
        return "type";
      case ts.ScriptElementKind.enumElement:
        return "enum";
      case ts.ScriptElementKind.variableElement:
      case ts.ScriptElementKind.letElement:
      case ts.ScriptElementKind.constElement:
        return "variable";
      case ts.ScriptElementKind.memberVariableElement:
      case ts.ScriptElementKind.memberGetAccessorElement:
      case ts.ScriptElementKind.memberSetAccessorElement:
        return "property";
      case ts.ScriptElementKind.keyword:
        return "keyword";
      default:
        return "variable";
    }
  }

  async function tsCompletionSource(context: CompletionContext): Promise<CompletionResult | null> {
    const charBefore = context.state.sliceDoc(Math.max(0, context.pos - 1), context.pos);
    if (!context.explicit && !/[\w$.]/.test(charBefore)) return null;

    const word = context.matchBefore(/[\w$]*/);
    let entries;
    try {
      entries = await getScriptCompletions(context.state.doc.toString(), context.pos);
    } catch {
      return null;
    }
    if (!entries.length) return null;

    return {
      from: word ? word.from : context.pos,
      options: entries.slice(0, 100).map((e) => ({ label: e.name, type: completionKindToType(e.kind) })),
      validFor: /^[\w$]*$/,
    };
  }

  async function tsLintSource(editorView: EditorView): Promise<Diagnostic[]> {
    const docLength = editorView.state.doc.length;
    let diagnostics;
    try {
      diagnostics = await getScriptDiagnostics(editorView.state.doc.toString());
    } catch {
      return [];
    }
    return diagnostics.map((d) => {
      const from = Math.max(0, Math.min(d.start, docLength));
      const to = Math.max(from, Math.min(d.start + d.length, docLength));
      return { from, to, severity: "error" as const, message: d.message };
    });
  }

  const tsHoverTooltip = hoverTooltip(async (editorView, pos) => {
    let info;
    try {
      info = await getScriptQuickInfo(editorView.state.doc.toString(), pos);
    } catch {
      return null;
    }
    if (!info) return null;

    return {
      pos: info.start,
      end: info.start + info.length,
      create: () => {
        const dom = document.createElement("div");
        dom.textContent = info.text;
        dom.style.cssText = "max-width: 420px; padding: 6px 8px; font: 11px/1.5 'JetBrains Mono', monospace; white-space: pre-wrap;";
        return { dom };
      },
    };
  });

  onMount(() => {
    const state = EditorState.create({
      doc: value,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightSpecialChars(),
        history(),
        foldGutter(),
        drawSelection(),
        dropCursor(),
        EditorState.allowMultipleSelections.of(true),
        indentOnInput(),
        syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
        bracketMatching(),
        closeBrackets(),
        autocompletion({ override: [tsCompletionSource], activateOnTyping: true }),
        linter(tsLintSource, { delay: 500 }),
        lintGutter(),
        tsHoverTooltip,
        highlightActiveLine(),
        keymap.of([
          // Ctrl-Space is often grabbed by the OS/IME, and Ctrl-letter combos risk colliding
          // with GTK's built-in Emacs-style text bindings (Ctrl-K = kill line, etc. — this
          // app runs in a WebKitGTK webview on Linux). A bare function key sidesteps both.
          { key: "F2", run: startCompletion, preventDefault: true, stopPropagation: true },
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...historyKeymap,
          ...completionKeymap,
          indentWithTab,
        ]),
        indentUnit.of("  "),
        javascript({ typescript: true }),
        themeCompartment.of(buildTheme(darkMode)),
        EditorView.updateListener.of((update) => {
          if (update.docChanged && !updating) {
            onchange(update.state.doc.toString());
          }
        }),
        EditorView.baseTheme({
          "&": { height: "100%", fontSize: "13px", fontFamily: "'JetBrains Mono', 'Fira Code', 'Menlo', monospace" },
          ".cm-scroller": { overflow: "auto", lineHeight: "1.6" },
        }),
      ],
    });

    view = new EditorView({ state, parent: container! });
  });

  onDestroy(() => {
    view?.destroy();
  });

  // Sync external value changes (e.g. switching scripts)
  $effect(() => {
    if (!view) return;
    const current = view.state.doc.toString();
    if (current !== value) {
      updating = true;
      view.dispatch({
        changes: { from: 0, to: current.length, insert: value },
      });
      updating = false;
    }
  });

  // Sync dark mode changes
  $effect(() => {
    if (!view) return;
    view.dispatch({
      effects: themeCompartment.reconfigure(buildTheme(darkMode)),
    });
  });
</script>

<div bind:this={container} class="h-full w-full overflow-hidden [&_.cm-editor]:h-full [&_.cm-editor.cm-focused]:outline-none"></div>
