<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { syntaxHighlighting, HighlightStyle } from "@codemirror/language";
  import { searchKeymap } from "@codemirror/search";
  import { EditorState, StateEffect, type Extension } from "@codemirror/state";
  import { tags } from "@lezer/highlight";
  import {
    EditorView,
    highlightActiveLine,
    keymap,
    lineNumbers
  } from "@codemirror/view";

  export let content = "";
  export let fontSize = 15;
  export let lineHeight = 1.65;
  export let showLineNumbers = true;
  export let lineWrapping = true;
  export let highlightLine = true;
  export let tabSize = 2;
  export let selectionFrom = 0;
  export let selectionTo = 0;
  export let scrollTop = 0;
  export let onChange: (value: string) => void = () => {};
  export let onSelectionChange: (from: number, to: number) => void = () => {};
  export let onScroll: (scrollTop: number) => void = () => {};
  export let onImageFiles: (files: File[]) => void = () => {};
  export let onSave: () => void = () => {};
  export let onNewArticle: () => void = () => {};

  let host: HTMLDivElement;
  let view: EditorView | null = null;
  let externalContent = content;

  const quietHighlight = HighlightStyle.define([
    { tag: tags.heading1, color: "var(--accent)", fontWeight: "700" },
    { tag: tags.heading2, color: "var(--accent)", fontWeight: "650" },
    { tag: tags.heading, color: "var(--text-primary)", fontWeight: "650" },
    { tag: tags.emphasis, fontStyle: "italic" },
    { tag: tags.strong, fontWeight: "700" },
    { tag: tags.link, color: "var(--accent)" },
    { tag: tags.url, color: "var(--success)" },
    { tag: tags.quote, color: "var(--text-secondary)" },
    { tag: tags.monospace, color: "var(--warning)" },
    { tag: tags.comment, color: "var(--text-tertiary)" },
    { tag: tags.meta, color: "var(--accent)" }
  ]);

  function extensions(): Extension[] {
    return [
      history(),
      markdown(),
      syntaxHighlighting(quietHighlight),
      EditorState.tabSize.of(tabSize),
      keymap.of([
        { key: "Mod-s", preventDefault: true, run: () => (onSave(), true) },
        { key: "Mod-n", preventDefault: true, run: () => (onNewArticle(), true) },
        indentWithTab,
        ...defaultKeymap,
        ...historyKeymap,
        ...searchKeymap
      ]),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          externalContent = update.state.doc.toString();
          onChange(externalContent);
        }
        if (update.docChanged || update.selectionSet) {
          const range = update.state.selection.main;
          onSelectionChange(range.from, range.to);
        }
      }),
      EditorView.domEventHandlers({
        paste(event) {
          const files = Array.from(event.clipboardData?.files ?? []).filter((file) =>
            file.type.startsWith("image/")
          );
          if (!files.length) return false;
          event.preventDefault();
          onImageFiles(files);
          return true;
        },
        dragover(event) {
          if (Array.from(event.dataTransfer?.items ?? []).some((item) => item.kind === "file")) {
            event.preventDefault();
            return true;
          }
          return false;
        },
        drop(event) {
          const files = Array.from(event.dataTransfer?.files ?? []).filter((file) =>
            file.type.startsWith("image/")
          );
          if (!files.length) return false;
          event.preventDefault();
          onImageFiles(files);
          return true;
        }
      }),
      EditorView.theme({
        "&": {
          height: "100%",
          color: "var(--text-primary)",
          backgroundColor: "var(--bg-panel)",
          fontSize: `${fontSize}px`
        },
        ".cm-scroller": {
          fontFamily: "var(--font-mono)",
          lineHeight: String(lineHeight),
          overflow: "auto"
        },
        ".cm-content": { padding: "22px 8px 80px" },
        ".cm-line": { padding: "0 18px" },
        ".cm-gutters": {
          color: "var(--text-tertiary)",
          backgroundColor: "var(--bg-panel)",
          borderRight: "1px solid var(--border-subtle)"
        },
        ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "var(--bg-control)" },
        ".cm-cursor, .cm-dropCursor": {
          borderLeftColor: "var(--editor-caret)",
          borderLeftWidth: "2px"
        },
        ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
          backgroundColor: "var(--accent-selection)"
        },
        ".cm-panels": { backgroundColor: "var(--bg-elevated)", color: "var(--text-primary)" }
      }),
      ...(showLineNumbers ? [lineNumbers()] : []),
      ...(lineWrapping ? [EditorView.lineWrapping] : []),
      ...(highlightLine ? [highlightActiveLine()] : [])
    ];
  }

  onMount(() => {
    view = new EditorView({
      state: EditorState.create({ doc: content, extensions: extensions() }),
      parent: host
    });
    view.scrollDOM.scrollTop = scrollTop;
    view.scrollDOM.addEventListener("scroll", handleScroll, { passive: true });
  });

  onDestroy(() => {
    view?.scrollDOM.removeEventListener("scroll", handleScroll);
    view?.destroy();
  });

  function handleScroll() {
    if (view) onScroll(view.scrollDOM.scrollTop);
  }

  $: if (view && content !== externalContent) {
    externalContent = content;
    const cursor = Math.max(0, Math.min(selectionFrom, content.length));
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: content },
      selection: { anchor: cursor, head: Math.max(cursor, Math.min(selectionTo, content.length)) }
    });
    requestAnimationFrame(() => {
      if (view) view.scrollDOM.scrollTop = scrollTop;
    });
  }

  $: if (view && Math.abs(view.scrollDOM.scrollTop - scrollTop) > 1) {
    view.scrollDOM.scrollTop = scrollTop;
  }

  $: if (view && content === externalContent) {
    const current = view.state.selection.main;
    const from = Math.max(0, Math.min(selectionFrom, view.state.doc.length));
    const to = Math.max(0, Math.min(selectionTo, view.state.doc.length));
    if (current.from !== from || current.to !== to) {
      view.dispatch({ selection: { anchor: from, head: to } });
    }
  }

  $: if (
    view &&
    fontSize &&
    lineHeight &&
    tabSize &&
    typeof showLineNumbers === "boolean" &&
    typeof lineWrapping === "boolean" &&
    typeof highlightLine === "boolean"
  ) {
    view.dispatch({ effects: StateEffect.reconfigure.of(extensions()) });
  }
</script>

<div class="markdown-editor-host" bind:this={host}></div>
