<script lang="ts">
  import { ExternalLink, RefreshCw } from "@lucide/svelte";
  export let running = false; export let busy = false; export let onOpen: () => void; export let onReload: () => void;
</script>
<div class="empty-state"><strong>真实 Hexo 主题预览</strong><p>{running ? "使用当前项目的主题、插件和文章实际路由，在受限独立窗口中显示。" : "请先启动 Hexo Server。"}</p><div class="button-row"><button class="button primary" type="button" disabled={!running || busy} on:click={onOpen}><ExternalLink size={14} />打开真实预览</button><button class="button" type="button" disabled={!running} on:click={onReload}><RefreshCw size={14} />刷新</button></div></div>
