<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import type { AppConfigV3, ImageBedProvider } from "$shared/types/app";
  import PluginManagerPage from "./PluginManagerPage.svelte";

  export let config: AppConfigV3;
  export let onConfigChange: (config: AppConfigV3) => void;
  export let onNotice: (message: string) => void;
</script>

<section class="plugins-workspace">
  <header><h1>{$ui("插件")}</h1><p>{$ui("安装、配置和管理插件，扩展你的写作工作区。")}</p></header>
  <PluginManagerPage {onNotice} selectedProvider={config.imageBed.defaultProvider}
    onProviderChange={(defaultProvider) => onConfigChange({ ...config, imageBed: { ...config.imageBed, defaultProvider: defaultProvider as ImageBedProvider } })} />
</section>

<style>
  .plugins-workspace { height: 100%; overflow: auto; padding: 28px; }
  header { margin-bottom: 24px; }
  h1 { margin: 0 0 8px; font-size: 24px; font-weight: 600; }
  p { margin: 0; color: var(--text-secondary); }
</style>
