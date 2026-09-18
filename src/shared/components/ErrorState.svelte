<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { CircleAlert } from "@lucide/svelte";

  export let title: string | undefined = undefined;
  export let message: string;
</script>

<div class="error-state" role="alert">
  <CircleAlert size={30} strokeWidth={1.5} aria-hidden="true" />
  <h2>{title ?? $ui("操作没有完成")}</h2>
  <p>{message}</p>
  <slot></slot>
</div>
