<script lang="ts">
  import { CircleAlert } from "@lucide/svelte";

  export let title = "操作没有完成";
  export let message: string;
</script>

<div class="error-state" role="alert">
  <CircleAlert size={30} strokeWidth={1.5} aria-hidden="true" />
  <h2>{title}</h2>
  <p>{message}</p>
  <slot></slot>
</div>
