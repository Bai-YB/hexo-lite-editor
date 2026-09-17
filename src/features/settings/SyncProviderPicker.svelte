<script lang="ts">
  import { GitBranch, Server } from "@lucide/svelte";
  import { createEventDispatcher } from "svelte";
  import { ui } from "$shared/i18n/ui";
  import type { ContentSyncProvider } from "$shared/types/app";

  export let value: ContentSyncProvider;
  export let disabled = false;

  const dispatch = createEventDispatcher<{ change: ContentSyncProvider }>();

  function choose(provider: ContentSyncProvider) {
    if (!disabled && provider !== value) dispatch("change", provider);
  }
</script>

<fieldset class="provider-picker" {disabled}>
  <legend>{$ui("选择同步方式")}</legend>
  <p>{$ui("选择后只显示该方式需要的设置；启用前可以随时切换。")}</p>
  <div class="provider-options" role="radiogroup" aria-label={$ui("同步方式")}>
    <button
      type="button"
      role="radio"
      aria-checked={value === "github"}
      class:active={value === "github"}
      on:click={() => choose("github")}
    >
      <span class="provider-icon"><GitBranch size={19} /></span>
      <span class="provider-copy">
        <strong>GitHub</strong>
        <span>{$ui("使用项目已有的 GitHub 仓库，在独立分支保存源码。")}</span>
        <small>{$ui("适合已使用 GitHub Pages 的项目")}</small>
      </span>
      <span class="provider-check" aria-hidden="true"></span>
    </button>
    <button
      type="button"
      role="radio"
      aria-checked={value === "webdav"}
      class:active={value === "webdav"}
      on:click={() => choose("webdav")}
    >
      <span class="provider-icon"><Server size={19} /></span>
      <span class="provider-copy">
        <strong>WebDAV</strong>
        <span>{$ui("连接你的 WebDAV 服务器，在指定目录保存源码。")}</span>
        <small>{$ui("适合自建存储、坚果云或 NAS")}</small>
      </span>
      <span class="provider-check" aria-hidden="true"></span>
    </button>
  </div>
</fieldset>

<style>
  .provider-picker { min-width: 0; margin: 0; padding: 0; border: 0; }
  .provider-picker legend { padding: 0; font-size: 15px; font-weight: 650; color: var(--text-primary); }
  .provider-picker > p { margin: 5px 0 16px; color: var(--text-secondary); font-size: 12px; line-height: 1.6; }
  .provider-options { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
  .provider-options button {
    position: relative;
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    align-items: start;
    gap: 11px;
    min-width: 0;
    min-height: 116px;
    padding: 16px;
    text-align: left;
    color: var(--text-primary);
    background: var(--bg-control);
    border: 1px solid var(--border-subtle);
    border-radius: 12px;
    cursor: pointer;
    transition: border-color 160ms ease, background 160ms ease, transform 160ms ease;
  }
  .provider-options button:hover:not(:disabled) { border-color: color-mix(in srgb, var(--accent) 55%, var(--border-subtle)); }
  .provider-options button:active:not(:disabled) { transform: translateY(1px); }
  .provider-options button:focus-visible { outline: 2px solid var(--accent); outline-offset: 3px; }
  .provider-options button.active { border-color: var(--accent); background: var(--accent-soft); }
  .provider-icon { display: grid; place-items: center; width: 34px; height: 34px; border-radius: 9px; color: var(--text-secondary); background: var(--bg-panel); border: 1px solid var(--border-subtle); }
  button.active .provider-icon { color: var(--accent); border-color: color-mix(in srgb, var(--accent) 40%, var(--border-subtle)); }
  .provider-copy { display: grid; gap: 5px; min-width: 0; }
  .provider-copy strong { font-size: 14px; }
  .provider-copy span { color: var(--text-secondary); font-size: 12px; line-height: 1.55; }
  .provider-copy small { color: var(--text-tertiary); font-size: 11px; line-height: 1.4; }
  .provider-check { width: 14px; height: 14px; margin-top: 3px; border: 1px solid var(--border-strong); border-radius: 50%; box-shadow: inset 0 0 0 3px var(--bg-control); }
  button.active .provider-check { background: var(--accent); border-color: var(--accent); }
  fieldset:disabled button { cursor: default; opacity: 0.7; }
  @media (max-width: 660px) { .provider-options { grid-template-columns: 1fr; } .provider-options button { min-height: 0; } }
</style>
