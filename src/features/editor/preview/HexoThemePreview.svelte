<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { ExternalLink, RefreshCw } from "@lucide/svelte";
  export let running = false; export let busy = false; export let onOpen: () => void; export let onReload: () => void;
</script>
<div class="empty-state"><strong>{$ui("网站预览")}</strong><p>{running ? $ui("保存文章后，用博客主题在独立窗口里预览。") : $ui("会先保存文章，再启动预览服务。")}</p><div class="button-row"><button class="button primary" type="button" disabled={busy} on:click={onOpen}><ExternalLink size={14} />{running ? $ui("打开预览窗口") : $ui("启动并预览")}</button><button class="button" type="button" disabled={busy} on:click={onReload}><RefreshCw size={14} />{$ui("刷新预览")}</button></div></div>
