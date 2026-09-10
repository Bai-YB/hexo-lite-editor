<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { KeyRound, ShieldCheck, Trash2, Wifi } from "@lucide/svelte";
  import type { AppConfigV3, CredentialStatus } from "$shared/types/app";

  export let settings: AppConfigV3["imageBed"];
  export let credential: CredentialStatus;
  export let legacyCredentialAvailable = false;
  export let busy = false;
  export let statusMessage = "";
  export let onChange: (settings: AppConfigV3["imageBed"]) => void = () => {};
  export let onAcquireToken: () => void = () => {};
  export let onMigrateLegacyToken: () => void = () => {};
  export let onTestConnection: () => void = () => {};
  export let onDeleteToken: () => void = () => {};
</script>

<div class="provider-settings" data-provider="cloudflare-imgbed">
  <div class="setting-row">
    <div class="setting-copy">
      <label for="cloudflare-service-url">{$ui("服务地址")}</label>
      <span id="cloudflare-service-hint">{$ui("使用不含账号信息的 HTTPS 地址。")}</span>
    </div>
    <input
      id="cloudflare-service-url"
      aria-describedby="cloudflare-service-hint"
      disabled={busy}
      class="input control-wide"
      type="url"
      value={settings.cloudflareApiUrl}
      placeholder="https://img.example.com"
      on:change={(event) => onChange({ ...settings, cloudflareApiUrl: event.currentTarget.value })}
    />
  </div>
  <div class="setting-row credential-row">
    <div class="setting-copy">
      <strong>{$ui("访问 Token")}</strong>
      <span>{$ui("管理员账号和密码只在创建 Token 时临时使用，不会保存。")}</span>
    </div>
    <div class="credential-actions">
      <span class:success={credential.configured} class:warning={!credential.configured} class="credential-status">
        <ShieldCheck size={14} />{credential.configured ? $ui("Token 已配置") : $ui("Token 未配置")}
      </span>
      <div class="button-row">
        {#if legacyCredentialAvailable && !credential.configured}
          <button class="button secondary" type="button" disabled={busy || !settings.cloudflareApiUrl.trim()} on:click={onMigrateLegacyToken}>
            <KeyRound size={14} />{$ui("绑定旧版 Token")}
          </button>
        {/if}
        <button class="button secondary" type="button" disabled={busy || !settings.cloudflareApiUrl.trim()} on:click={onAcquireToken}>
          <KeyRound size={14} />{credential.configured ? $ui("重新获取") : $ui("一键获取 Token")}
        </button>
        <button class="button" type="button" disabled={busy || !credential.configured || !settings.cloudflareApiUrl.trim()} on:click={onTestConnection}>
          <Wifi size={14} />{$ui("测试连接")}
        </button>
        {#if credential.configured}
          <button class="button danger" type="button" disabled={busy} on:click={onDeleteToken}>
            <Trash2 size={14} />{$ui("删除本地 Token")}
          </button>
        {/if}
      </div>
      {#if statusMessage}<span class="credential-message" role="status">{statusMessage}</span>{/if}
    </div>
  </div>
</div>
