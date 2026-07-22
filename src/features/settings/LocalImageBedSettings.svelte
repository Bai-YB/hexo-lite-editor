<script lang="ts">
  import type { AppConfigV3 } from "$shared/types/app";

  export let settings: AppConfigV3["imageBed"];
  export let onChange: (settings: AppConfigV3["imageBed"]) => void = () => {};
</script>

<div class="provider-settings" data-provider="local">
  <div class="setting-row">
    <div class="setting-copy">
      <strong>图片保存目录</strong>
      <span>必须位于项目的 source/ 目录下，例如 source/images。</span>
    </div>
    <input
      class="input control-wide"
      value={settings.localImageDir}
      placeholder="source/images"
      on:change={(event) => onChange({ ...settings, localImageDir: event.currentTarget.value })}
    />
  </div>
  <div class="setting-row">
    <div class="setting-copy">
      <strong>Markdown 访问前缀</strong>
      <span>插入文章时使用的站点路径，例如 /images。</span>
    </div>
    <input
      class="input control-wide"
      value={settings.localMarkdownPrefix}
      placeholder="/images"
      on:change={(event) => onChange({ ...settings, localMarkdownPrefix: event.currentTarget.value })}
    />
  </div>
</div>
