<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, keymap, lineNumbers, highlightActiveLineGutter, highlightSpecialChars, drawSelection, dropCursor, highlightActiveLine } from "@codemirror/view";
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
  import { closeBrackets, closeBracketsKeymap, autocompletion, completionKeymap } from "@codemirror/autocomplete";

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
        autocompletion(),
        highlightActiveLine(),
        keymap.of([
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...historyKeymap,
          ...completionKeymap,
          indentWithTab,
        ]),
        indentUnit.of("  "),
        javascript(),
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
