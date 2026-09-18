<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { LoaderCircle } from "@lucide/svelte";
  export let label: string | undefined = undefined;
</script>

<div class="loading-state" aria-live="polite" aria-busy="true">
  <LoaderCircle class="spin" size={16} aria-hidden="true" />
  <span>{label ?? $ui("正在加载")}…</span>
</div>
