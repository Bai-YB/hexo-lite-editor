<script lang="ts">
  import { onMount } from "svelte";
  import { platform, normalizeError } from "$platform/tauri";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import type { PluginView } from "$shared/plugins/types";
  import { contributedImageBedProviders, imageBedProviderId, testPluginConnection, validatePluginConfig } from "$shared/plugins/PluginProviderRuntime";

  export let onNotice: (message: string) => void = () => {};
  export let selectedProvider = "local";
  export let onProviderChange: (provider: string) => void = () => {};

  let plugins: PluginView[] = [];
  let loading = true;
  let pendingEnable: PluginView | null = null;
  let editing: PluginView | null = null;
  let settingsSchema: Record<string, unknown> | null = null;
  let settings: Record<string, unknown> = {};
  let busy = false;

  onMount(() => void refresh());

  async function refresh() {
    try { plugins = await platform.listPlugins(); }
    catch (error) { onNotice(normalizeError(error).message); }
    finally { loading = false; }
  }

  async function confirmEnable() {
    if (!pendingEnable) return;
    busy = true;
    try { plugins = await platform.enablePlugin(pendingEnable.manifest.id); pendingEnable = null; }
    catch (error) { onNotice(normalizeError(error).message); }
    finally { busy = false; }
  }

  async function disable(plugin: PluginView) {
    plugins = await platform.disablePlugin(plugin.manifest.id);
    if (selectedProvider === imageBedProviderId(plugin.manifest.id)) onProviderChange("local");
  }

  async function uninstall(plugin: PluginView) {
    plugins = await platform.uninstallPlugin(plugin.manifest.id);
    if (selectedProvider === imageBedProviderId(plugin.manifest.id)) onProviderChange("local");
  }

  async function install() {
    try { plugins = await platform.chooseAndInstallPlugin(); }
    catch (error) { onNotice(normalizeError(error).message); }
  }

  async function openSettings(plugin: PluginView) {
    busy = true;
    try {
      [settingsSchema, settings] = await Promise.all([
        platform.getPluginSettingsSchema(plugin.manifest.id),
        platform.getPluginSettings(plugin.manifest.id)
      ]);
      editing = plugin;
    } catch (error) { onNotice(normalizeError(error).message); }
    finally { busy = false; }
  }

  async function saveSettings() {
    if (!editing) return;
    busy = true;
    try { await platform.savePluginSettings(editing.manifest.id, settings); editing = null; onNotice("插件设置已保存。"); }
    catch (error) { onNotice(normalizeError(error).message); }
    finally { busy = false; }
  }

  async function testSettings() {
    if (!editing) return;
    busy = true;
    try {
      const validation = await validatePluginConfig(editing, settings);
      if (!validation.ok) throw new Error(validation.message || "插件配置无效。");
      const result = await testPluginConnection(editing, settings);
      onNotice(result.ok ? (result.message || "插件连接正常。") : (result.message || "插件连接测试失败。"));
    } catch (error) { onNotice(normalizeError(error).message); }
    finally { busy = false; }
  }

  function schemaProperties(): Array<[string, Record<string, unknown>]> {
    const properties = settingsSchema?.properties;
    return properties && typeof properties === "object"
      ? Object.entries(properties as Record<string, Record<string, unknown>>)
      : [];
  }
</script>

<div class="settings-block">
  <div class="settings-block-heading"><h3>插件</h3><p>插件在隔离 Worker 中运行，所有 Host API 请求都需要声明权限。</p><button class="button" type="button" on:click={install}>安装插件</button></div>
  {#if loading}<p class="muted-line">正在读取插件…</p>{:else if !plugins.length}<p class="muted-line">尚未安装插件。将插件目录安装到应用配置目录后会显示在这里。</p>{:else}
    <div class="recent-project-list">{#each plugins as plugin (plugin.manifest.id)}
      <div class="recent-project-row"><div><strong>{plugin.manifest.name}</strong><span>{plugin.manifest.id} · {plugin.manifest.version}</span></div>
        {#if contributedImageBedProviders(plugin).length}<button class="button" type="button" on:click={() => onProviderChange(imageBedProviderId(plugin.manifest.id))}>{selectedProvider === imageBedProviderId(plugin.manifest.id) ? "当前图床" : "设为图床"}</button>{/if}
        {#if plugin.manifest.contributes.settings}<button class="button" type="button" on:click={() => openSettings(plugin)}>设置</button>{/if}
        <button class="button" type="button" on:click={() => plugin.enabled ? disable(plugin) : (pendingEnable = plugin)}>{plugin.enabled ? "禁用" : "启用"}</button>
        <button class="button danger" type="button" on:click={() => uninstall(plugin)}>卸载</button>
      </div>
    {/each}</div>
  {/if}
</div>

{#if pendingEnable}
  <ModalDialog title="确认启用插件" description={pendingEnable.manifest.name} onClose={() => (pendingEnable = null)}>
    <p>此插件将获得以下权限。只启用你信任的插件：</p>
    <ul>{#each pendingEnable.manifest.permissions as permission}<li><code>{permission}</code></li>{/each}</ul>
    <svelte:fragment slot="actions"><button class="button" type="button" on:click={() => (pendingEnable = null)}>取消</button><button class="button primary" type="button" disabled={busy} on:click={confirmEnable}>确认启用</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if editing}
  <ModalDialog title={`${editing.manifest.name} 设置`} onClose={() => (editing = null)}>
    {#if schemaProperties().length}
      <div class="modal-form">{#each schemaProperties() as [key, schema]}
        <label><span>{String(schema.title ?? key)}</span><input class="input" type={schema.type === "number" ? "number" : "text"} value={String(settings[key] ?? schema.default ?? "")} on:input={(event) => (settings = { ...settings, [key]: schema.type === "number" ? Number(event.currentTarget.value) : event.currentTarget.value })} /></label>
      {/each}</div>
    {:else}<p class="muted-line">此插件没有可编辑的设置。</p>{/if}
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={busy} on:click={testSettings}>测试连接</button><button class="button" type="button" on:click={() => (editing = null)}>取消</button><button class="button primary" type="button" disabled={busy} on:click={saveSettings}>保存</button></svelte:fragment>
  </ModalDialog>
{/if}
