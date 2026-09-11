<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { onMount } from "svelte";
  import { ExternalLink, LoaderCircle, Scale } from "@lucide/svelte";
  import PageHeader from "$shared/components/PageHeader.svelte";
  import { normalizeError, platform } from "$platform/tauri";
  import type { UpdateSnapshot } from "$shared/types/app";
  import { renderSafeMarkdown } from "$shared/markdown/safeMarkdown";
  import { updateViewModel } from "$features/update/updateViewModel";
  import UpdateProgressBar from "$features/update/UpdateProgressBar.svelte";
  import { translate } from "$shared/i18n";
  import { appVersion } from "$shared/version";

  export let onNotice: (message: string) => void = () => {};
  export let onInstallUpdate: () => void = () => {};

  let version = appVersion;
  let update: UpdateSnapshot | null = null;
  let checking = false;
  let downloading = false;

  onMount(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    let receivedEvent = false;
    void Promise.all([platform.runtimeInfo(), platform.getUpdateSnapshot()])
      .then(([runtime, snapshot]) => { if (!disposed) { version = runtime.version; if (!receivedEvent) update = snapshot; } })
      .catch(() => { version = appVersion; });
    void platform.onUpdateSnapshot((snapshot) => { receivedEvent = true; if (!disposed) update = snapshot; }).then((value) => { if (disposed) value(); else unlisten = value; });
    return () => { disposed = true; unlisten?.(); };
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
    if (downloading) return;
    downloading = true;
    try {
      update = await platform.downloadUpdate();
      if (update.status === "upToDate") onNotice("该更新当前不可用，已重新检查版本。");
    } catch (error) { onNotice(normalizeError(error).message); }
    finally { downloading = false; }
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
      <div class="about-update"><span><strong>{$translate("pages.update")}</strong><small>{update ? `${update.currentVersion} → ${update.latestVersion ?? update.currentVersion} · ${update.status}` : $translate("pages.notChecked")}</small></span>{#if updateModel?.canDownload}<button class="button quiet" type="button" disabled={downloading} on:click={downloadUpdate}>{$translate("pages.downloadUpdate")}</button>{:else if updateModel?.canInstall}<button class="button quiet" type="button" on:click={onInstallUpdate}>{$translate("pages.restartInstall")}</button>{:else}<button class="button quiet" type="button" disabled={checking || (updateModel ? !updateModel.canCheck : false)} on:click={checkUpdate}>{#if checking}<LoaderCircle size={14} class="spin" />{$translate("pages.checking")}{:else}{$translate("pages.checkUpdate")}{/if}</button>{/if}</div>
      {#if updateModel && update?.status === "downloading"}<UpdateProgressBar percent={updateModel.percent} label={$ui("更新下载进度")} valueText={updateModel.valueText} indeterminate={updateModel.indeterminate} />{/if}
      {#if updateModel?.errorLabel}<p class="sync-error" role="alert">{update?.errorMessage || updateModel.errorLabel}</p>{/if}
      {#if releaseHtml}<article class="markdown-preview update-release-notes">{@html releaseHtml}</article>{/if}
    </section>
  </div>
</div>
