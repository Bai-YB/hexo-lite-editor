<script lang="ts">
  import { KeyRound, ShieldCheck, Trash2, Wifi } from "@lucide/svelte";
  import type { AppConfigV3, CredentialStatus } from "$shared/types/app";

  export let settings: AppConfigV3["imageBed"];
  export let credential: CredentialStatus;
  export let busy = false;
  export let statusMessage = "";
  export let onChange: (settings: AppConfigV3["imageBed"]) => void = () => {};
  export let onAcquireToken: () => void = () => {};
  export let onTestConnection: () => void = () => {};
  export let onDeleteToken: () => void = () => {};
</script>

<div class="provider-settings" data-provider="cloudflare-imgbed">
  <div class="setting-row">
    <div class="setting-copy">
      <strong>图床名称</strong>
      <span>仅用于在本机识别这套图床配置。</span>
    </div>
    <input
      class="input control-wide"
      value={settings.cloudflareName}
      placeholder="我的博客图床"
      on:change={(event) => onChange({ ...settings, cloudflareName: event.currentTarget.value })}
    />
  </div>
  <div class="setting-row">
    <div class="setting-copy">
      <strong>服务地址</strong>
      <span>使用不含账号信息的 HTTPS 地址。</span>
    </div>
    <input
      class="input control-wide"
      type="url"
      value={settings.cloudflareApiUrl}
      placeholder="https://img.example.com"
      on:change={(event) => onChange({ ...settings, cloudflareApiUrl: event.currentTarget.value })}
    />
  </div>
  <div class="setting-row credential-row">
    <div class="setting-copy">
      <strong>访问 Token</strong>
      <span>管理员账号和密码只在创建 Token 时临时使用，不会保存。</span>
    </div>
    <div class="credential-actions">
      <span class:success={credential.configured} class:warning={!credential.configured} class="credential-status">
        <ShieldCheck size={14} />{credential.configured ? "Token 已配置" : "Token 未配置"}
      </span>
      <div class="button-row">
        <button class="button secondary" type="button" disabled={busy || !settings.cloudflareApiUrl.trim()} on:click={onAcquireToken}>
          <KeyRound size={14} />{credential.configured ? "重新获取" : "一键获取 Token"}
        </button>
        <button class="button" type="button" disabled={busy || !credential.configured || !settings.cloudflareApiUrl.trim()} on:click={onTestConnection}>
          <Wifi size={14} />测试连接
        </button>
        {#if credential.configured}
          <button class="button danger" type="button" disabled={busy} on:click={onDeleteToken}>
            <Trash2 size={14} />删除本地 Token
          </button>
        {/if}
      </div>
      {#if statusMessage}<span class="credential-message" role="status">{statusMessage}</span>{/if}
    </div>
  </div>
</div>
