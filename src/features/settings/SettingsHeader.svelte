<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { Save } from "@lucide/svelte";
  import PageHeader from "$shared/components/PageHeader.svelte";

  export let dirty = false;
  export let saving = false;
  export let onDiscard: () => void = () => {};
  export let onSave: () => void = () => {};
</script>

<div class="settings-sticky-header">
  <PageHeader title={$ui("设置")} description={$ui("按工作流程整理；更改只在保存后写入应用配置。")}>
    <span class:warning={dirty} class:success={!dirty} class="settings-save-state">{saving ? $ui("正在保存") : dirty ? $ui("有未保存更改") : $ui("已保存")}</span>
    {#if dirty || saving}
      <button class="button" type="button" disabled={saving} on:click={onDiscard}>{$ui("取消")}</button>
      <button class="button primary" type="button" disabled={saving} on:click={onSave}><Save size={15} />{$ui("保存")}</button>
    {/if}
  </PageHeader>
</div>
