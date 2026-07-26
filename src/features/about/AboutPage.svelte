<script lang="ts">
  import { onMount } from "svelte";
  import { ExternalLink, LoaderCircle, Scale } from "@lucide/svelte";
  import PageHeader from "$shared/components/PageHeader.svelte";
  import { normalizeError, platform } from "$platform/tauri";
  import type { UpdateCheckResult } from "$shared/types/app";

  export let onNotice: (message: string) => void = () => {};

  let version = "1.0.5";
  let update: UpdateCheckResult | null = null;
  let checking = false;

  onMount(async () => {
    try {
      version = (await platform.runtimeInfo()).version;
    } catch {
      version = "1.0.5";
    }
  });

  async function checkUpdate() {
    checking = true;
    try {
      update = await platform.checkUpdate();
      onNotice(update.hasUpdate ? `发现新版本 ${update.latestVersion}` : "当前已经是最新版本。");
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      checking = false;
    }
  }
</script>

<div class="workspace-page">
  <PageHeader title="关于" description="一个安静、可靠的 Hexo 桌面写作工具。" />

  <div class="about-simple">
    <section class="about-intro">
      <img src="/favicon.png" alt="" />
      <div><h2>Hexo Lite Editor</h2><p>专注于 Markdown 写作、图片管理与可靠发布，不内置博客环境，也不接管你的文章文件。</p><span>版本 {version}</span></div>
    </section>
    <section class="panel settings-group about-links">
      <button type="button" on:click={() => platform.openExternalTarget("projectHomepage")}><span><strong>项目主页</strong><small>源代码、问题反馈与版本记录</small></span><ExternalLink size={16} /></button>
      <button type="button" on:click={() => platform.openExternalTarget("license")}><span><strong>MIT License</strong><small>查看开源许可证</small></span><Scale size={16} /></button>
      <div class="about-update"><span><strong>更新</strong><small>{update ? (update.hasUpdate ? `发现 ${update.latestVersion}` : `当前 ${update.currentVersion} 已是最新`) : "尚未检查"}</small></span>{#if update?.hasUpdate}<button class="button quiet" type="button" on:click={() => platform.openExternalTarget("releasePage")}>查看版本</button>{:else}<button class="button quiet" type="button" disabled={checking} on:click={checkUpdate}>{#if checking}<LoaderCircle size={14} class="spin" />检查中{:else}检查更新{/if}</button>{/if}</div>
    </section>
  </div>
</div>
