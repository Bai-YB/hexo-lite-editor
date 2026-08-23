<script lang="ts">
  import type { PreviewMode } from "./previewSession";
  export let mode: PreviewMode; export let onChange: (mode: PreviewMode) => void;
</script>
<div class="filter-row" role="group" aria-label="预览模式">
  <button class:active={mode === "quick"} class="filter-chip" type="button" on:click={() => onChange("quick")}>快速预览</button>
  <button class:active={mode === "theme"} class="filter-chip" type="button" on:click={() => onChange("theme")}>真实主题</button>
</div>
