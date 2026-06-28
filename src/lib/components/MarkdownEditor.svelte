<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { yaml } from "@codemirror/lang-yaml";
  import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
  import { searchKeymap, openSearchPanel } from "@codemirror/search";
  import { EditorState, Prec, Transaction, type Range } from "@codemirror/state";
  import { tags } from "@lezer/highlight";
  import {
    Decoration,
    type DecorationSet,
    EditorView,
    type ViewUpdate,
    ViewPlugin,
    highlightActiveLine,
    keymap,
    lineNumbers
  } from "@codemirror/view";
  import type { ColorScheme } from "$lib/types/settings";

  export let content = "";
  export let fontSize = 15;
  export let lineHeight = 1.6;
  export let showLineNumbers = true;
  export let lineWrapping = true;
  export let highlightLine = true;
  export let tabSize = 2;
  export let colorScheme: ColorScheme = "classic";
  export let syntax: "markdown" | "yaml" = "markdown";
  export let onChange: (value: string) => void = () => {};
  export let onSave: () => void = () => {};
  export let onNewPost: () => void = () => {};
  export let onScrollRatio: (ratio: number) => void = () => {};
  export let onPasteImages: (files: File[]) => void = () => {};

  let host: HTMLDivElement;
  let view: EditorView | null = null;
  let externalContent = content;
  let scrollFrame = 0;
  let syncingExternalContent = false;

  type SyntaxPalette = {
    heading1: string;
    heading2: string;
    heading3: string;
    heading: string;
    emphasis: string;
    link: string;
    url: string;
    quote: string;
    monospace: string;
    keyword: string;
    atom: string;
    bool: string;
    string: string;
    number: string;
    comment: string;
    meta: string;
    processing: string;
    separator: string;
  };

  const syntaxPalettes: Record<ColorScheme, SyntaxPalette> = {
    classic: {
      heading1: "#0f6b8f",
      heading2: "#1f7a5c",
      heading3: "#7c5a13",
      heading: "#2f5f8f",
      emphasis: "#9f3a38",
      link: "#2563eb",
      url: "#0f766e",
      quote: "#7c3aed",
      monospace: "#b45309",
      keyword: "#7c3aed",
      atom: "#0f766e",
      bool: "#b45309",
      string: "#166534",
      number: "#b45309",
      comment: "#687385",
      meta: "#6d28d9",
      processing: "#9333ea",
      separator: "#64748b"
    },
    "vscode-light": {
      heading1: "#0451a5",
      heading2: "#267f99",
      heading3: "#795e26",
      heading: "#0451a5",
      emphasis: "#a31515",
      link: "#0000ff",
      url: "#098658",
      quote: "#800000",
      monospace: "#a31515",
      keyword: "#0000ff",
      atom: "#267f99",
      bool: "#0000ff",
      string: "#a31515",
      number: "#098658",
      comment: "#008000",
      meta: "#af00db",
      processing: "#af00db",
      separator: "#666666"
    },
    "vscode-dark": {
      heading1: "#4fc1ff",
      heading2: "#4ec9b0",
      heading3: "#dcdcaa",
      heading: "#569cd6",
      emphasis: "#ce9178",
      link: "#3794ff",
      url: "#4ec9b0",
      quote: "#c586c0",
      monospace: "#ce9178",
      keyword: "#569cd6",
      atom: "#4ec9b0",
      bool: "#569cd6",
      string: "#ce9178",
      number: "#b5cea8",
      comment: "#6a9955",
      meta: "#c586c0",
      processing: "#dcdcaa",
      separator: "#858585"
    },
    monokai: {
      heading1: "#66d9ef",
      heading2: "#a6e22e",
      heading3: "#e6db74",
      heading: "#66d9ef",
      emphasis: "#f92672",
      link: "#66d9ef",
      url: "#a6e22e",
      quote: "#ae81ff",
      monospace: "#fd971f",
      keyword: "#f92672",
      atom: "#66d9ef",
      bool: "#ae81ff",
      string: "#e6db74",
      number: "#ae81ff",
      comment: "#88846f",
      meta: "#f92672",
      processing: "#fd971f",
      separator: "#75715e"
    },
    "one-dark": {
      heading1: "#61afef",
      heading2: "#98c379",
      heading3: "#e5c07b",
      heading: "#61afef",
      emphasis: "#e06c75",
      link: "#61afef",
      url: "#56b6c2",
      quote: "#c678dd",
      monospace: "#d19a66",
      keyword: "#c678dd",
      atom: "#56b6c2",
      bool: "#d19a66",
      string: "#98c379",
      number: "#d19a66",
      comment: "#7f848e",
      meta: "#c678dd",
      processing: "#e5c07b",
      separator: "#5c6370"
    },
    "solarized-light": {
      heading1: "#268bd2",
      heading2: "#2aa198",
      heading3: "#b58900",
      heading: "#268bd2",
      emphasis: "#dc322f",
      link: "#268bd2",
      url: "#2aa198",
      quote: "#6c71c4",
      monospace: "#cb4b16",
      keyword: "#859900",
      atom: "#2aa198",
      bool: "#b58900",
      string: "#2aa198",
      number: "#d33682",
      comment: "#93a1a1",
      meta: "#6c71c4",
      processing: "#cb4b16",
      separator: "#839496"
    }
  };

  function buildHighlightTheme(scheme: ColorScheme) {
    const palette = syntaxPalettes[scheme] ?? syntaxPalettes.classic;
    return HighlightStyle.define([
      { tag: tags.heading1, color: palette.heading1, fontWeight: "800", fontSize: "1.38em" },
      { tag: tags.heading2, color: palette.heading2, fontWeight: "760", fontSize: "1.24em" },
      { tag: tags.heading3, color: palette.heading3, fontWeight: "720", fontSize: "1.14em" },
      { tag: tags.heading, color: palette.heading, fontWeight: "700" },
      { tag: [tags.strong, tags.emphasis], color: palette.emphasis, fontWeight: "700" },
      { tag: tags.link, color: palette.link, textDecoration: "underline" },
      { tag: tags.url, color: palette.url },
      { tag: tags.quote, color: palette.quote, fontStyle: "italic" },
      { tag: tags.monospace, color: palette.monospace, backgroundColor: "var(--code-bg)" },
      { tag: tags.keyword, color: palette.keyword, fontWeight: "650" },
      { tag: tags.atom, color: palette.atom },
      { tag: tags.bool, color: palette.bool },
      { tag: tags.string, color: palette.string },
      { tag: tags.number, color: palette.number },
      { tag: tags.comment, color: palette.comment, fontStyle: "italic" },
      { tag: tags.meta, color: palette.meta },
      { tag: tags.processingInstruction, color: palette.processing, fontWeight: "700" },
      { tag: tags.separator, color: palette.separator, fontWeight: "700" }
    ]);
  }

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: content,
        extensions: buildExtensions()
      })
    });
    view.scrollDOM.addEventListener("scroll", handleScroll, { passive: true });
  });

  $: if (view && content !== externalContent) {
    externalContent = content;
    syncingExternalContent = true;
    try {
      view.dispatch({
        changes: {
          from: 0,
          to: view.state.doc.length,
          insert: content
        },
        annotations: Transaction.addToHistory.of(false)
      });
    } finally {
      syncingExternalContent = false;
    }
  }

  export function insertText(text: string) {
    if (!view) return;
    const selection = view.state.selection.main;
    view.dispatch({
      changes: { from: selection.from, to: selection.to, insert: text },
      selection: { anchor: selection.from + text.length }
    });
    view.focus();
  }

  onDestroy(() => {
    if (scrollFrame) cancelAnimationFrame(scrollFrame);
    view?.scrollDOM.removeEventListener("scroll", handleScroll);
    view?.destroy();
  });

  function handleScroll() {
    if (!view || scrollFrame) return;
    scrollFrame = requestAnimationFrame(() => {
      scrollFrame = 0;
      if (!view) return;
      const scroller = view.scrollDOM;
      const frontMatterHeight = getFrontMatterLineCount(view.state.doc.toString()) * fontSize * lineHeight;
      const top = Math.max(0, scroller.scrollTop - frontMatterHeight);
      const max = Math.max(1, scroller.scrollHeight - scroller.clientHeight - frontMatterHeight);
      onScrollRatio(Math.min(1, Math.max(0, top / max)));
    });
  }

  function getFrontMatterLineCount(value: string): number {
    if (!value.startsWith("---")) return 0;
    const lines = value.split(/\r?\n/);
    for (let index = 1; index < lines.length; index += 1) {
      if (lines[index].trim() === "---") return index + 1;
    }
    return 0;
  }

  function buildExtensions() {
    return [
      showLineNumbers ? lineNumbers() : [],
      history(),
      syntax === "yaml" ? yaml() : markdown(),
      syntaxHighlighting(buildHighlightTheme(colorScheme)),
      syntax === "markdown" ? frontMatterHighlighter : [],
      lineWrapping ? EditorView.lineWrapping : [],
      highlightLine ? highlightActiveLine() : [],
      EditorState.tabSize.of(tabSize),
      EditorView.theme({
        "&": {
          height: "100%",
          fontSize: `${fontSize}px`,
          color: "var(--text-main)",
          backgroundColor: "var(--bg-panel)"
        },
        ".cm-scroller": {
          fontFamily: "'JetBrains Mono', Consolas, 'Courier New', monospace",
          lineHeight: String(lineHeight)
        },
        ".cm-content": {
          padding: "16px 18px",
          caretColor: "var(--editor-cursor)"
        },
        ".cm-cursor": {
          borderLeftColor: "var(--editor-cursor) !important",
          borderLeftWidth: "2px"
        },
        ".cm-dropCursor": {
          borderLeftColor: "var(--editor-cursor) !important",
          borderLeftWidth: "2px"
        },
        ".cm-selectionBackground, &.cm-focused .cm-selectionBackground": {
          backgroundColor: "var(--selection-bg) !important"
        },
        ".cm-line": {
          padding: "0 3px"
        },
        ".cm-activeLine": {
          backgroundColor: "var(--editor-active-line)"
        },
        ".cm-activeLineGutter": {
          backgroundColor: "var(--editor-active-gutter)"
        },
        ".cm-gutters": {
          borderRight: "1px solid var(--border-soft)",
          color: "var(--text-muted)",
          backgroundColor: "var(--bg-subtle)"
        },
        ".cm-content ::selection": {
          color: "var(--selection-text)",
          backgroundColor: "var(--selection-bg)"
        },
        ".cm-line:has(.tok-meta)": {
          backgroundColor: "var(--editor-meta-line)"
        },
        ".cm-frontmatter-line": {
          color: "var(--text-main)",
          backgroundColor: "var(--editor-meta-line)"
        },
        ".cm-frontmatter-key": {
          color: "var(--editor-frontmatter-key) !important",
          fontWeight: "700"
        },
        ".cm-frontmatter-marker": {
          color: "var(--editor-frontmatter-marker) !important",
          fontWeight: "700"
        }
      }),
      EditorView.domEventHandlers({
        paste: (event) => {
          const files = Array.from(event.clipboardData?.files ?? []).filter((file) => file.type.startsWith("image/"));
          if (!files.length) return false;
          event.preventDefault();
          onPasteImages(files);
          return true;
        }
      }),
      Prec.highest(
        keymap.of([
          {
            key: "Mod-s",
            run: () => {
              onSave();
              return true;
            }
          },
          {
            key: "Mod-f",
            run: (editor) => {
              openSearchPanel(editor);
              return true;
            }
          },
          {
            key: "Mod-n",
            run: () => {
              onNewPost();
              return true;
            }
          },
          indentWithTab,
          ...searchKeymap,
          ...historyKeymap,
          ...defaultKeymap
        ])
      ),
      EditorView.updateListener.of((update) => {
        if (!update.docChanged) return;
        const next = update.state.doc.toString();
        externalContent = next;
        if (syncingExternalContent) return;
        onChange(next);
      })
    ];
  }

  const frontMatterLine = Decoration.line({ class: "cm-frontmatter-line" });
  const frontMatterKey = Decoration.mark({ class: "cm-frontmatter-key" });
  const frontMatterMarker = Decoration.mark({ class: "cm-frontmatter-marker" });

  const frontMatterHighlighter = ViewPlugin.fromClass(
    class {
      decorations: DecorationSet;

      constructor(view: EditorView) {
        this.decorations = buildFrontMatterDecorations(view);
      }

      update(update: ViewUpdate) {
        if (update.docChanged || update.viewportChanged) {
          this.decorations = buildFrontMatterDecorations(update.view);
        }
      }
    },
    {
      decorations: (value) => value.decorations
    }
  );

  function buildFrontMatterDecorations(view: EditorView): DecorationSet {
    const ranges: Range<Decoration>[] = [];
    const doc = view.state.doc;
    const firstLine = doc.line(1);
    if (firstLine.text.trim() !== "---") return Decoration.none;

    let endLineNumber = 0;
    for (let lineNumber = 2; lineNumber <= doc.lines; lineNumber += 1) {
      const line = doc.line(lineNumber);
      if (line.text.trim() === "---") {
        endLineNumber = lineNumber;
        break;
      }
    }
    if (!endLineNumber) return Decoration.none;

    for (let lineNumber = 1; lineNumber <= endLineNumber; lineNumber += 1) {
      const line = doc.line(lineNumber);
      ranges.push(frontMatterLine.range(line.from));
      if (line.text.trim() === "---") {
        ranges.push(frontMatterMarker.range(line.from, line.to));
        continue;
      }
      const keyMatch = /^(\s*[A-Za-z_][\w-]*)(\s*:)/.exec(line.text);
      if (keyMatch) {
        const keyStart = line.from + keyMatch[1].search(/\S/);
        const keyEnd = line.from + keyMatch[1].length;
        ranges.push(frontMatterKey.range(keyStart, keyEnd));
      }
    }

    return Decoration.set(ranges, true);
  }
</script>

<div class="editor-host" bind:this={host}></div>

<style>
  .editor-host {
    height: 100%;
    min-height: 0;
    overflow: hidden;
    background: var(--bg-panel);
  }
</style>
