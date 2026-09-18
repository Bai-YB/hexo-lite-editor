<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { onMount } from "svelte";
  import { ExternalLink, LoaderCircle, Scale } from "@lucide/svelte";
  import PageHeader from "$shared/components/PageHeader.svelte";
  import { normalizeError, platform } from "$platform/tauri";
  import { renderSafeMarkdown } from "$shared/markdown/safeMarkdown";
  import { releaseSummary, sampleTransfer, transferViewModel, updateViewModel, type TransferSample } from "$features/update/updateViewModel";
  import { updateStore } from "$features/update/updateStore";
  import UpdateProgressBar from "$features/update/UpdateProgressBar.svelte";
  import { translate } from "$shared/i18n";
  import { appVersion } from "$shared/version";

  export let onNotice: (message: string, severity?: "info" | "error") => void = () => {};
  export let onInstallUpdate: () => void = () => {};

  let version = appVersion;
  let checking = false;
  let downloading = false;
  let samples: TransferSample[] = [];
  let now = Date.now();

  onMount(() => {
    let disposed = false;
    void platform.runtimeInfo().then((runtime) => { if (!disposed) version = runtime.version; }).catch(() => {});
    const unsubscribe = updateStore.subscribe((snapshot) => {
      now = Date.now();
      samples = sampleTransfer(samples, snapshot, now);
    });
    const timer = setInterval(() => { now = Date.now(); }, 1000);
    return () => { disposed = true; unsubscribe(); clearInterval(timer); };
  });

  async function checkUpdate() {
    if (checking) return;
    checking = true;
    try { updateStore.set(await platform.checkUpdate()); }
    catch (error) { onNotice(normalizeError(error).message, "error"); }
    finally { checking = false; }
  }
  async function downloadUpdate() {
    if (downloading) return;
    downloading = true;
    try { updateStore.set(await platform.downloadUpdate()); }
    catch (error) { onNotice(normalizeError(error).message, "error"); }
    finally { downloading = false; }
  }
  $: updateModel = updateViewModel($updateStore, $ui);
  $: transfer = transferViewModel(samples, $updateStore, now, $ui);
  $: summary = releaseSummary($updateStore.releaseNotes);
  $: releaseHtml = $updateStore.releaseNotes ? renderSafeMarkdown($updateStore.releaseNotes) : "";
</script>

<div class="workspace-page">
  <PageHeader title={$translate("pages.aboutTitle")} description={$translate("pages.aboutDescription")} />

  <div class="about-simple">
    <section class="about-intro">
      <img src="/favicon.png" alt="" />
      <div><h2>Hexo Lite Editor</h2><p>{$translate("pages.aboutProductDescription")}</p><span>{$translate("pages.version")} {version}</span></div>
    </section>

    <section class="panel update-panel" aria-label={$ui("应用更新")}>
      <div class="update-heading">
        <div><h2>{$translate("pages.update")}</h2><p>{version}{#if $updateStore.latestVersion} → {$updateStore.latestVersion}{/if}</p></div>
        {#if updateModel.canInstall}
          <button class="button primary" type="button" on:click={onInstallUpdate}>{$ui("安装更新")}</button>
        {:else if updateModel.canDownload}
          <button class="button primary" type="button" disabled={downloading} on:click={downloadUpdate}>{$translate("pages.downloadUpdate")}</button>
        {:else}
          <button class="button" type="button" disabled={checking || !updateModel.canCheck} on:click={checkUpdate}>
            {#if checking || $updateStore.status === "checking"}<LoaderCircle size={14} class="spin" />{$translate("pages.checking")}{:else}{$translate("pages.checkUpdate")}{/if}
          </button>
        {/if}
      </div>
      <p class="update-stage" role="status">{$ui(updateModel.stageLabel)}</p>
      {#if $updateStore.status === "downloading" || $updateStore.status === "verifying"}
        <UpdateProgressBar percent={updateModel.percent} label={$ui(updateModel.stageLabel)} valueText={updateModel.valueText}
          indeterminate={updateModel.indeterminate} speed={transfer.speed} remaining={transfer.remaining} />
      {:else if updateModel.canInstall}
        <p class="update-help">{$ui("更新包已验证。点击安装后，应用将自动重启。")}</p>
      {:else if $updateStore.status === "available"}
        <p class="update-help">{$ui("下载期间可以继续编辑，完成后由你选择安装。")}</p>
      {/if}
      {#if updateModel.errorLabel}<p class="sync-error" role="alert">{$updateStore.errorMessage || updateModel.errorLabel}</p>{/if}
      {#if releaseHtml}
        <div class="release-summary">
          <h3>{$ui("本次更新")}</h3>
          <ul>{#each summary as item}<li>{item}</li>{/each}</ul>
          <details>
            <summary>{$ui("完整更新日志")}</summary>
            <article class="markdown-preview update-release-notes">{@html releaseHtml}</article>
          </details>
        </div>
      {/if}
    </section>

    <section class="panel settings-group about-links">
      <button type="button" on:click={() => platform.openExternalTarget("projectHomepage")}><span><strong>{$translate("pages.projectHomepage")}</strong><small>{$translate("pages.projectHomepageDescription")}</small></span><ExternalLink size={16} /></button>
      <button type="button" on:click={() => platform.openExternalTarget("license")}><span><strong>MIT License</strong><small>{$translate("pages.licenseDescription")}</small></span><Scale size={16} /></button>
    </section>
  </div>
</div>

<style>
  .update-panel { padding: 20px; }
  .update-heading { display: flex; align-items: center; justify-content: space-between; gap: 16px; }
  .update-heading h2 { margin: 0; font-size: 16px; }
  .update-heading p { margin: 5px 0 0; color: var(--text-secondary); font-size: 13px; font-variant-numeric: tabular-nums; }
  .update-stage { margin: 18px 0 0; font-weight: 600; }
  .update-help { margin: 6px 0 0; color: var(--text-secondary); font-size: 13px; }
  .release-summary { margin-top: 20px; padding-top: 16px; border-top: 1px solid var(--border-subtle); }
  .release-summary h3 { margin: 0 0 8px; font-size: 14px; }
  .release-summary ul { margin: 0; padding-left: 20px; color: var(--text-secondary); font-size: 13px; line-height: 1.75; }
  .release-summary li + li { margin-top: 4px; }
  details { margin-top: 14px; }
  summary { width: fit-content; color: var(--accent); cursor: pointer; font-size: 13px; padding-block: 4px; }
  summary:hover { color: var(--accent-hover); }
  summary:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: 3px; border-radius: 3px; }
  .update-release-notes { margin-top: 12px; max-height: 320px; overflow-y: auto; }
  @media (max-width: 660px) {
    .update-panel { padding: 16px; }
    .update-heading { align-items: stretch; flex-direction: column; }
    .update-heading button { width: 100%; }
  }
</style>
