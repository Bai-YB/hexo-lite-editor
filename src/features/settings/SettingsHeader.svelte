<script lang="ts">
  import { Save } from "@lucide/svelte";
  import PageHeader from "$shared/components/PageHeader.svelte";

  export let dirty = false;
  export let saving = false;
  export let onDiscard: () => void = () => {};
  export let onSave: () => void = () => {};
</script>

<div class="settings-sticky-header">
  <PageHeader title="设置" description="按工作流程整理；更改只在保存后写入应用配置。">
    <span class:warning={dirty} class:success={!dirty} class="settings-save-state">{saving ? "正在保存" : dirty ? "有未保存更改" : "已保存"}</span>
    {#if dirty || saving}
      <button class="button" type="button" disabled={saving} on:click={onDiscard}>取消</button>
      <button class="button primary" type="button" disabled={saving} on:click={onSave}><Save size={15} />保存</button>
    {/if}
  </PageHeader>
</div>
