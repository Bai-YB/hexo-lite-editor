<script lang="ts">
  import { onMount } from "svelte";
  import { ExternalLink, LoaderCircle, Scale } from "@lucide/svelte";
  import PageHeader from "$shared/components/PageHeader.svelte";
  import { normalizeError, platform } from "$platform/tauri";
  import type { UpdateSnapshot } from "$shared/types/app";
  import { renderSafeMarkdown } from "$shared/markdown/safeMarkdown";
  import { updateViewModel } from "$features/update/updateViewModel";
  import UpdateProgressBar from "$features/update/UpdateProgressBar.svelte";
  import { translate } from "$shared/i18n";

  export let onNotice: (message: string) => void = () => {};
  export let onInstallUpdate: () => void = () => {};

  let version = "1.0.6";
  let update: UpdateSnapshot | null = null;
  let checking = false;

  onMount(() => {
    let unlisten: (() => void) | undefined;
    void Promise.all([platform.runtimeInfo(), platform.getUpdateSnapshot()])
      .then(([runtime, snapshot]) => { version = runtime.version; update = snapshot; })
      .catch(() => { version = "1.0.6"; });
    void platform.onUpdateSnapshot((snapshot) => (update = snapshot)).then((value) => (unlisten = value));
    return () => unlisten?.();
  });

  async function checkUpdate() {
    checking = true;
    try {
      update = await platform.checkUpdate();
      onNotice(update.status === "available" ? `发现新版本 ${update.latestVersion}` : "当前已经是最新版本。");
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      checking = false;
    }
  }
  async function downloadUpdate() {
    try { update = await platform.downloadUpdate(); }
    catch (error) { onNotice(normalizeError(error).message); }
  }
  $: updateModel = update ? updateViewModel(update) : null;
  $: releaseHtml = update?.releaseNotes ? renderSafeMarkdown(update.releaseNotes) : "";
</script>

<div class="workspace-page">
  <PageHeader title={$translate("pages.aboutTitle")} description={$translate("pages.aboutDescription")} />

  <div class="about-simple">
    <section class="about-intro">
      <img src="/favicon.png" alt="" />
      <div><h2>Hexo Lite Editor</h2><p>{$translate("pages.aboutProductDescription")}</p><span>{$translate("pages.version")} {version}</span></div>
    </section>
    <section class="panel settings-group about-links">
      <button type="button" on:click={() => platform.openExternalTarget("projectHomepage")}><span><strong>{$translate("pages.projectHomepage")}</strong><small>{$translate("pages.projectHomepageDescription")}</small></span><ExternalLink size={16} /></button>
      <button type="button" on:click={() => platform.openExternalTarget("license")}><span><strong>MIT License</strong><small>{$translate("pages.licenseDescription")}</small></span><Scale size={16} /></button>
      <div class="about-update"><span><strong>{$translate("pages.update")}</strong><small>{update ? `${update.currentVersion} → ${update.latestVersion ?? update.currentVersion} · ${update.status}` : $translate("pages.notChecked")}</small></span>{#if update?.status === "available"}<button class="button quiet" type="button" on:click={downloadUpdate}>{$translate("pages.downloadUpdate")}</button>{:else if update?.status === "downloaded"}<button class="button quiet" type="button" on:click={onInstallUpdate}>{$translate("pages.restartInstall")}</button>{:else}<button class="button quiet" type="button" disabled={checking || update?.status === "downloading" || update?.status === "installing"} on:click={checkUpdate}>{#if checking}<LoaderCircle size={14} class="spin" />{$translate("pages.checking")}{:else}{$translate("pages.checkUpdate")}{/if}</button>{/if}</div>
      {#if updateModel && update?.status === "downloading"}<UpdateProgressBar percent={updateModel.percent} label="更新下载进度" valueText={updateModel.valueText} indeterminate={updateModel.indeterminate} />{/if}
      {#if releaseHtml}<article class="markdown-preview update-release-notes">{@html releaseHtml}</article>{/if}
    </section>
  </div>
</div>
