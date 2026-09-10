<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { ExternalLink, RefreshCw } from "@lucide/svelte";
  export let running = false; export let busy = false; export let onOpen: () => void; export let onReload: () => void;
</script>
<div class="empty-state"><strong>{$ui("真实 Hexo 主题预览")}</strong><p>{running ? $ui("保存当前文章后，使用博客主题在独立窗口中预览。") : $ui("点击后保存当前文章并启动 Hexo 预览服务。")}</p><div class="button-row"><button class="button primary" type="button" disabled={busy} on:click={onOpen}><ExternalLink size={14} />{running ? $ui("打开真实预览") : $ui("启动并预览")}</button><button class="button" type="button" disabled={busy} on:click={onReload}><RefreshCw size={14} />{$ui("刷新当前文章")}</button></div></div>
