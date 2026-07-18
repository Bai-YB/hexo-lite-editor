<script lang="ts">
  import { LoaderCircle } from "@lucide/svelte";
  export let label = "正在加载";
</script>

<div class="loading-state" aria-live="polite" aria-busy="true">
  <LoaderCircle class="spin" size={16} aria-hidden="true" />
  <span>{label}…</span>
</div>
