<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { onMount } from "svelte";
  import { platform, normalizeError } from "$platform/tauri";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import type { PluginView } from "$shared/plugins/types";
  import { contributedImageBedProviders, disposePluginWorker, reconcilePluginWorkers, imageBedProviderId, testPluginConnection, validatePluginConfig } from "$shared/plugins/PluginProviderRuntime";
  import { initializeSettings, settingFields, validateSettings } from "$shared/plugins/settings";

  export let onNotice: (message: string) => void = () => {};
  export let selectedProvider = "local";
  export let onProviderChange: (provider: string) => void = () => {};

  let plugins: PluginView[] = [];
  let loading = true;
  let pendingEnable: PluginView | null = null;
  let pendingUninstall: PluginView | null = null;
  let preserveSettings = true;
  let editing: PluginView | null = null;
  let settingsSchema: Record<string, unknown> | null = null;
  let settings: Record<string, unknown> = {};
  let busy = false;
  let fieldErrors: Record<string, string> = {};
  let settingsError = "";
  $: if (!loading) reconcilePluginWorkers(plugins);

  onMount(() => void refresh());

  async function refresh() {
    try { plugins = await platform.listPlugins(); }
    catch (error) { onNotice(normalizeError(error).message); }
    finally { loading = false; }
  }

  async function confirmEnable() {
    if (!pendingEnable || busy) return;
    busy = true;
    try { plugins = await platform.enablePlugin(pendingEnable.manifest.id); pendingEnable = null; }
    catch (error) { onNotice(normalizeError(error).message); }
    finally { busy = false; }
  }

  async function disable(plugin: PluginView) {
    if (busy) return;
    busy = true;
    try {
      plugins = await platform.disablePlugin(plugin.manifest.id);
      disposePluginWorker(plugin.manifest.id);
      if (selectedProvider === imageBedProviderId(plugin.manifest.id)) onProviderChange("local");
      onNotice("插件已禁用。");
    } catch (error) { onNotice(normalizeError(error).message); }
    finally { busy = false; }
  }

  async function uninstall() {
    if (!pendingUninstall || busy) return;
    const plugin = pendingUninstall;
    busy = true;
    try {
      plugins = await platform.uninstallPlugin(plugin.manifest.id, preserveSettings);
      disposePluginWorker(plugin.manifest.id);
      if (selectedProvider === imageBedProviderId(plugin.manifest.id)) onProviderChange("local");
      pendingUninstall = null;
      onNotice(preserveSettings ? "插件已卸载，设置已保留，重新安装后恢复。" : "插件及其设置已卸载。");
    } catch (error) { onNotice(normalizeError(error).message); }
    finally { busy = false; }
  }

  async function install() {
    if (busy) return;
    busy = true;
    try { plugins = await platform.chooseAndInstallPlugin(); }
    catch (error) { onNotice(normalizeError(error).message); }
    finally { busy = false; }
  }

  async function openSettings(plugin: PluginView) {
    if (busy) return;
    busy = true;
    try {
      [settingsSchema, settings] = await Promise.all([
        platform.getPluginSettingsSchema(plugin.manifest.id),
        platform.getPluginSettings(plugin.manifest.id)
      ]);
      settings = initializeSettings(settingsSchema, settings);
      fieldErrors = {};
      settingsError = "";
      editing = plugin;
    } catch (error) { onNotice(normalizeError(error).message); }
    finally { busy = false; }
  }

  async function saveSettings() {
    if (!editing || busy) return;
    fieldErrors = validateSettings(settingsSchema, settings);
    if (Object.keys(fieldErrors).length) return;
    const plugin = editing;
    busy = true;
    try {
      if (plugin.enabled) {
        const validation = await validatePluginConfig(plugin, settings);
        if (!validation?.ok) throw new Error(validation?.message || "插件配置无效。");
      }
      await platform.savePluginSettings(plugin.manifest.id, settings);
      editing = null;
      onNotice("插件设置已保存。");
    }
    catch (error) { settingsError = normalizeError(error).message; }
    finally { busy = false; }
  }

  async function testSettings() {
    if (!editing || busy || !editing.enabled) return;
    const plugin = editing;
    fieldErrors = validateSettings(settingsSchema, settings);
    if (Object.keys(fieldErrors).length) return;
    busy = true;
    try {
      const validation = await validatePluginConfig(plugin, settings);
      if (!validation.ok) throw new Error(validation.message || "插件配置无效。");
      const result = await testPluginConnection(plugin, settings);
      onNotice(result.ok ? (result.message || "插件连接正常。") : (result.message || "插件连接测试失败。"));
    } catch (error) { settingsError = normalizeError(error).message; }
    finally { busy = false; }
  }

  function schemaProperties(): Array<[string, Record<string, unknown>]> {
    return settingFields(settingsSchema);
  }
</script>

<div class="settings-block">
  <div class="settings-block-heading"><h3>{$ui("插件")}</h3><p>{$ui("插件在隔离 Worker 中运行，所有 Host API 请求都需要声明权限。")}</p><button class="button" type="button" disabled={busy} on:click={install}>{$ui("安装插件")}</button></div>
  {#if loading}<p class="muted-line">{$ui("正在读取插件…")}</p>{:else if !plugins.length}<p class="muted-line">{$ui("尚未安装插件。将插件目录安装到应用配置目录后会显示在这里。")}</p>{:else}
    <div class="recent-project-list">{#each plugins as plugin (plugin.manifest.id)}
      <div class="recent-project-row"><div><strong>{plugin.manifest.name}</strong><span>{plugin.manifest.id} · {plugin.manifest.version}</span></div>
        {#if contributedImageBedProviders(plugin).length}<button class="button" type="button" disabled={busy} on:click={() => onProviderChange(imageBedProviderId(plugin.manifest.id))}>{selectedProvider === imageBedProviderId(plugin.manifest.id) ? $ui("当前图床") : $ui("设为图床")}</button>{/if}
        {#if plugin.manifest.contributes.settings}<button class="button" type="button" disabled={busy} on:click={() => openSettings(plugin)}>{$ui("设置")}</button>{/if}
        <button class="button" type="button" disabled={busy} on:click={() => plugin.enabled ? disable(plugin) : (pendingEnable = plugin)}>{plugin.enabled ? $ui("禁用") : $ui("启用")}</button>
        <button class="button danger" type="button" disabled={busy} on:click={() => { pendingUninstall = plugin; preserveSettings = true; }}>{$ui("卸载")}</button>
      </div>
    {/each}</div>
  {/if}
</div>

{#if pendingEnable}
  <ModalDialog title={$ui("确认启用插件")} description={pendingEnable.manifest.name} onClose={() => { if (!busy) pendingEnable = null; }}>
    <p>{$ui("此插件将获得以下权限。只启用你信任的插件：")}</p>
    <ul>{#each pendingEnable.manifest.permissions as permission}<li><code>{permission}</code></li>{/each}</ul>
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={busy} on:click={() => (pendingEnable = null)}>{$ui("取消")}</button><button class="button primary" type="button" disabled={busy} on:click={confirmEnable}>{$ui("确认启用")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if editing}
  <ModalDialog title={$ui("{p0} 设置", { p0: editing.manifest.name })} onClose={() => { if (!busy) editing = null; }}>
    {#if settingsError}<p class="form-error" role="alert">{settingsError}</p>{/if}
    {#if schemaProperties().length}
      <div class="modal-form">{#each schemaProperties() as [key, schema]}
        <label><span>{String(schema.title ?? key)}</span>
          {#if schema.type === "boolean"}
            <input type="checkbox" disabled={busy} checked={settings[key] === true} on:change={(event) => (settings = { ...settings, [key]: event.currentTarget.checked })} />
          {:else if Array.isArray(schema.enum)}
            <select class="input" disabled={busy} value={String(schema.enum.indexOf(settings[key]))} on:change={(event) => (settings = { ...settings, [key]: (schema.enum as unknown[])[Number(event.currentTarget.value)] })}>
              <option value="-1" disabled>{$ui("请选择")}</option>{#each schema.enum as option, index}<option value={String(index)}>{String(option)}</option>{/each}
            </select>
          {:else if !schema.type || ["string", "number", "integer"].includes(String(schema.type))}
            <input class="input" disabled={busy} aria-invalid={!!fieldErrors[key]} type={schema.type === "number" || schema.type === "integer" ? "number" : "text"} value={String(settings[key] ?? "")} on:input={(event) => (settings = { ...settings, [key]: schema.type === "number" || schema.type === "integer" ? (event.currentTarget.value === "" ? undefined : Number(event.currentTarget.value)) : event.currentTarget.value })} />
          {:else}<span class="muted-line">{$ui("此设置类型暂不支持编辑，已保存的值将保留。")}</span>{/if}
          {#if fieldErrors[key]}<span class="form-error" role="alert">{fieldErrors[key]}</span>{/if}
        </label>
      {/each}</div>
    {:else}<p class="muted-line">{$ui("此插件没有可编辑的设置。")}</p>{/if}
    {#if !editing.enabled}<p class="muted-line">{$ui("可先保存配置，启用插件后再测试连接。")}</p>{/if}
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={busy || !editing.enabled} on:click={testSettings}>{$ui("测试连接")}</button><button class="button" type="button" disabled={busy} on:click={() => (editing = null)}>{$ui("取消")}</button><button class="button primary" type="button" disabled={busy} on:click={saveSettings}>{$ui("保存")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if pendingUninstall}
  <ModalDialog title={$ui("卸载插件？")} description={pendingUninstall.manifest.name} onClose={() => { if (!busy) pendingUninstall = null; }}>
    <p>{$ui("卸载后此插件将停止运行。若它是当前图床，上传来源将切换为本地。")}</p>
    <label><input type="checkbox" disabled={busy} bind:checked={preserveSettings} /> {$ui("保留插件设置，重新安装时恢复")}</label>
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={busy} data-autofocus on:click={() => (pendingUninstall = null)}>{$ui("取消")}</button><button class="button danger" type="button" disabled={busy} on:click={uninstall}>{busy ? $ui("卸载中…") : $ui("确认卸载")}</button></svelte:fragment>
  </ModalDialog>
{/if}
