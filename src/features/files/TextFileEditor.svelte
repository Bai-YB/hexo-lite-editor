<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorState, Compartment } from "@codemirror/state";
  import { EditorView, lineNumbers, keymap, drawSelection, highlightActiveLine } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { search, searchKeymap, highlightSelectionMatches } from "@codemirror/search";
  import { syntaxHighlighting, HighlightStyle } from "@codemirror/language";
  import { tags } from "@lezer/highlight";
  import { markdown } from "@codemirror/lang-markdown";
  import { yaml } from "@codemirror/lang-yaml";
  import { json } from "@codemirror/lang-json";
  import { javascript } from "@codemirror/lang-javascript";
  import { css } from "@codemirror/lang-css";
  import { html } from "@codemirror/lang-html";
  import type { AppConfigV3 } from "$shared/types/app";
  export let content: string;
  export let path: string;
  export let documentInstance: number;
  export let config: AppConfigV3;
  export let readonly = false;
  export let onChange: (content: string) => void;
  export let onSave: () => void;
  let host: HTMLDivElement;
  let view: EditorView | undefined;
  let instance = documentInstance;
  const options = new Compartment();
  const highlight = HighlightStyle.define([
    { tag: tags.keyword, color: "var(--accent)" },
    { tag: [tags.string, tags.regexp], color: "var(--success)" },
    { tag: [tags.number, tags.bool, tags.atom], color: "var(--warning)" },
    { tag: [tags.propertyName, tags.tagName, tags.attributeName], color: "var(--accent)" },
    { tag: [tags.comment, tags.meta], color: "var(--text-tertiary)" },
    { tag: tags.heading, color: "var(--accent)", fontWeight: "bold" },
    { tag: tags.link, color: "var(--accent)", textDecoration: "underline" },
    { tag: tags.invalid, color: "var(--danger)" }
  ]);
  function language() {
    const extension = path.split(".").at(-1)?.toLowerCase();
    if (["md", "markdown"].includes(extension ?? "")) return markdown();
    if (["yml", "yaml"].includes(extension ?? "")) return yaml();
    if (extension === "json") return json();
    if (["js", "mjs", "cjs", "ts"].includes(extension ?? "")) return javascript({ typescript: extension === "ts" });
    if (["css", "scss"].includes(extension ?? "")) return css();
    if (["html", "xml", "svg", "ejs", "njk"].includes(extension ?? "")) return html();
    return [];
  }
  function editorOptions() {
    return [
      EditorState.readOnly.of(readonly), EditorView.editable.of(!readonly),
      EditorState.tabSize.of(config.editor.tabSize),
      ...(config.editor.showLineNumbers ? [lineNumbers()] : []),
      ...(config.editor.highlightActiveLine ? [highlightActiveLine()] : []),
      ...(config.editor.lineWrapping ? [EditorView.lineWrapping] : [])
    ];
  }
  function state() {
    return EditorState.create({ doc: content, extensions: [
      history(), drawSelection(), search({ top: true }), highlightSelectionMatches(), language(), syntaxHighlighting(highlight), options.of(editorOptions()),
      EditorView.contentAttributes.of({ "aria-label": "文件内容" }),
      keymap.of([{ key: "Mod-s", preventDefault: true, run: () => { onSave(); return true; } }, indentWithTab, ...defaultKeymap, ...historyKeymap, ...searchKeymap]),
      EditorView.updateListener.of(update => { if (update.docChanged) onChange(update.state.doc.toString()); }),
      EditorView.theme({ "&": { height: "100%", color: "var(--text-primary)", backgroundColor: "var(--bg-panel)" }, ".cm-scroller": { overflow: "auto", fontFamily: "var(--font-mono, monospace)" }, ".cm-content": { padding: "16px 0" }, ".cm-gutters": { border: "none", color: "var(--text-tertiary)", backgroundColor: "transparent" }, ".cm-activeLine, .cm-activeLineGutter": { backgroundColor: "var(--accent-soft)" }, "&.cm-focused": { outline: "none" } })
    ] });
  }
  onMount(() => { view = new EditorView({ state: state(), parent: host }); });
  onDestroy(() => view?.destroy());
  $: if (view && instance !== documentInstance) { instance = documentInstance; view.setState(state()); }
  $: if (view && content !== view.state.doc.toString()) { view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: content } }); }
  $: if (view && config && typeof readonly === "boolean") view.dispatch({ effects: options.reconfigure(editorOptions()) });
</script>

<div class="text-file-editor" bind:this={host} style:font-size={`${config.editor.fontSize}px`} style:line-height={config.editor.lineHeight}></div>

<style>
  .text-file-editor { min-height: 0; height: 100%; flex: 1; overflow: hidden; }
</style>
