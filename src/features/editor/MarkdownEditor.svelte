<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { markdown } from "@codemirror/lang-markdown";
  import { syntaxHighlighting, HighlightStyle } from "@codemirror/language";
  import { highlightSelectionMatches, search, searchKeymap } from "@codemirror/search";
  import { EditorState, StateEffect, type Extension } from "@codemirror/state";
  import { tags } from "@lezer/highlight";
  import {
    EditorView,
    crosshairCursor,
    drawSelection,
    dropCursor,
    highlightActiveLine,
    highlightActiveLineGutter,
    highlightSpecialChars,
    keymap,
    lineNumbers,
    rectangularSelection
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
  export let onScroll: (scrollTop: number, scrollHeight: number, clientHeight: number) => void = () => {};
  export let onImageFiles: (files: File[]) => void = () => {};
  export let onSave: () => void = () => {};
  export let onNewArticle: () => void = () => {};

  let host: HTMLDivElement;
  let view: EditorView | null = null;
  let externalContent = content;

  // 组件实例级常量：样式值全部走 CSS var，主题切换无需重建；
  // reconfigure 时引用同一实例，生成的高亮 class 不抖动。
  const quietHighlight = HighlightStyle.define([
    { tag: tags.heading1, color: "var(--accent)", fontWeight: "700" },
    { tag: tags.heading2, color: "var(--accent)", fontWeight: "650" },
    { tag: tags.heading3, color: "var(--text-primary)", fontWeight: "650" },
    { tag: tags.heading, color: "var(--text-primary)", fontWeight: "600" },
    { tag: tags.emphasis, fontStyle: "italic" },
    { tag: tags.strong, fontWeight: "700" },
    { tag: tags.strikethrough, color: "var(--text-tertiary)", textDecoration: "line-through" },
    { tag: tags.link, color: "var(--accent)" },
    { tag: tags.url, color: "var(--success)" },
    { tag: tags.string, color: "var(--success)" },
    { tag: tags.labelName, color: "var(--text-tertiary)" },
    { tag: tags.monospace, color: "var(--warning)" },
    { tag: tags.quote, color: "var(--text-secondary)", fontStyle: "italic" },
    { tag: tags.contentSeparator, color: "var(--text-tertiary)", fontWeight: "600" },
    { tag: tags.comment, color: "var(--text-tertiary)", fontStyle: "italic" },
    { tag: [tags.meta, tags.documentMeta], color: "var(--text-tertiary)" },
    { tag: [tags.escape, tags.character], color: "var(--warning)" },
    { tag: tags.atom, color: "var(--accent)", fontWeight: "600" },
    { tag: tags.invalid, color: "var(--danger)" },
    { tag: tags.tagName, color: "var(--accent)" },
    { tag: tags.attributeName, color: "var(--text-secondary)" },
    // 必须放最后：HeaderMark 等节点同时带 heading 与 processingInstruction，
    // CSS 按定义顺序取胜，本条在后 ⇒ "#"、">"、"-" 等标记符统一淡化。
    { tag: tags.processingInstruction, color: "var(--text-tertiary)" }
  ]);

  const zhPhrases = EditorState.phrases.of({
    "Find": "查找",
    "Replace": "替换",
    "next": "下一个",
    "previous": "上一个",
    "all": "全部",
    "match case": "区分大小写",
    "by word": "全词匹配",
    "regexp": "正则表达式",
    "replace": "替换",
    "replace all": "全部替换",
    "close": "关闭",
    "current match": "当前匹配",
    "replaced match on line $": "已在第 $ 行替换匹配",
    "replaced $ matches": "已替换 $ 处匹配",
    "on line": "所在行",
    "Go to line": "跳转到行",
    "go": "跳转",
    "Control character": "控制字符"
  });

  function extensions(): Extension[] {
    return [
      history(),
      markdown(),
      syntaxHighlighting(quietHighlight),
      EditorState.tabSize.of(tabSize),
      EditorState.allowMultipleSelections.of(true),
      zhPhrases,
      drawSelection(),
      dropCursor(),
      rectangularSelection(),
      crosshairCursor(),
      highlightSpecialChars(),
      // 显式挂载搜索（而非靠 openSearchPanel 动态注入），
      // 字号/行高触发 reconfigure 时面板与查询状态得以保留。
      search({ top: true }),
      highlightSelectionMatches({ minSelectionLength: 2 }),
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
          const position = view?.posAtCoords({ x: event.clientX, y: event.clientY });
          if (position != null) view?.dispatch({ selection: { anchor: position } });
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
        // 半透明 token：drawSelection 的选区层画在文本层之下，
        // 不透明背景会盖住当前行内的选区。
        ".cm-activeLine, .cm-activeLineGutter": {
          backgroundColor: "var(--editor-active-line)"
        },
        ".cm-cursor, .cm-dropCursor": {
          borderLeftColor: "var(--editor-caret)",
          borderLeftWidth: "2px",
          marginLeft: "-1px"
        },
        // 失焦选区退为中性灰；聚焦选择器对齐 baseTheme 的完整链，
        // 否则会被默认的浅蓝选区盖掉。
        ".cm-selectionBackground": { backgroundColor: "var(--bg-control-active)" },
        "&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground": {
          backgroundColor: "var(--accent-selection)"
        },
        ".cm-selectionMatch": {
          backgroundColor: "var(--editor-selection-match)",
          borderRadius: "2px"
        },
        ".cm-searchMatch": {
          backgroundColor: "var(--editor-selection-match)",
          borderRadius: "2px"
        },
        ".cm-searchMatch.cm-searchMatch-selected": {
          backgroundColor: "var(--accent-selection)",
          outline: "1px solid var(--accent)"
        },
        ".cm-panels": {
          backgroundColor: "var(--bg-elevated)",
          color: "var(--text-primary)"
        },
        ".cm-panels.cm-panels-top": {
          borderBottom: "1px solid var(--border-subtle)"
        },
        ".cm-panel.cm-search": {
          fontFamily: "var(--font-ui)",
          fontSize: "12px",
          padding: "8px 10px 10px"
        },
        ".cm-panel.cm-search .cm-textfield": {
          backgroundColor: "var(--bg-control)",
          border: "1px solid var(--border-subtle)",
          borderRadius: "6px",
          color: "var(--text-primary)"
        },
        ".cm-panel.cm-search .cm-button": {
          backgroundImage: "none",
          backgroundColor: "var(--bg-control)",
          border: "1px solid var(--border-subtle)",
          borderRadius: "6px",
          color: "var(--text-primary)"
        },
        ".cm-panel.cm-search .cm-button:active": {
          backgroundColor: "var(--bg-control-active)"
        },
        ".cm-panel.cm-search .cm-button:hover": {
          backgroundColor: "var(--bg-control-hover)"
        },
        ".cm-panel.cm-search label": { color: "var(--text-secondary)" },
        ".cm-panel.cm-search button[name=close]": { color: "var(--text-tertiary)" }
      }),
      ...(showLineNumbers ? [lineNumbers()] : []),
      ...(lineWrapping ? [EditorView.lineWrapping] : []),
      // highlightLine 开关同时控制正文行高亮与行号槽高亮
      ...(highlightLine ? [highlightActiveLine(), highlightActiveLineGutter()] : [])
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
    if (view) {
      onScroll(view.scrollDOM.scrollTop, view.scrollDOM.scrollHeight, view.scrollDOM.clientHeight);
    }
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
