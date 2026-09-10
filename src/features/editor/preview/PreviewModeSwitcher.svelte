<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import type { PreviewMode } from "./previewSession";
  export let mode: PreviewMode; export let onChange: (mode: PreviewMode) => void;
</script>
<div class="filter-row" role="group" aria-label={$ui("预览模式")}>
  <button class:active={mode === "quick"} class="filter-chip" type="button" on:click={() => onChange("quick")}>{$ui("快速预览")}</button>
  <button class:active={mode === "theme"} class="filter-chip" type="button" on:click={() => onChange("theme")}>{$ui("真实主题")}</button>
</div>
