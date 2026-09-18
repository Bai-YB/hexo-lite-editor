<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import PageHeader from "$shared/components/PageHeader.svelte";

  export let dirty = false;
  export let saving = false;
</script>

<div class="settings-sticky-header">
  <PageHeader title={$ui("设置")} description={$ui("所有设置改完即生效。")}>
    <span class:warning={dirty} class:success={!dirty} class="settings-save-state">{saving ? $ui("正在保存…") : dirty ? $ui("未保存") : $ui("已保存")}</span>
  </PageHeader>
</div>
