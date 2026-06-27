<script lang="ts">
  import { RefreshCcw, ExternalLink } from "@lucide/svelte";
  import { renderMarkdown } from "$lib/markdown/renderMarkdown";

  export let content = "";
  export let onRefresh: () => void = () => {};
  export let onOpenPreview: () => void = () => {};

  let article: HTMLElement;
  let programmaticScroll = false;

  $: html = renderMarkdown(content);

  export function scrollToRatio(ratio: number) {
    if (!article) return;
    const max = Math.max(0, article.scrollHeight - article.clientHeight);
    programmaticScroll = true;
    article.scrollTop = max * Math.min(1, Math.max(0, ratio));
    window.setTimeout(() => {
      programmaticScroll = false;
    }, 80);
  }
</script>

<section class="preview-panel">
  <div class="panel-header">
    <div>
      <h2>预览</h2>
      <span>实时 Markdown</span>
    </div>
    <div class="compact-actions">
      <button title="刷新预览" on:click={onRefresh}><RefreshCcw size={16} /></button>
      <button title="打开 Hexo 预览" on:click={onOpenPreview}><ExternalLink size={16} /></button>
    </div>
  </div>
  <article class="markdown-preview" bind:this={article} class:programmatic-scroll={programmaticScroll}>
    {@html html}
  </article>
</section>
