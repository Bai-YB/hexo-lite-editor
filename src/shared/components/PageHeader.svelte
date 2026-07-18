<script lang="ts">
  export let title: string;
  export let description = "";
</script>

<header class="page-header">
  <div class="page-title-group">
    <h1>{title}</h1>
    {#if description}<p>{description}</p>{/if}
    <div class="page-status"><slot name="status"></slot></div>
  </div>
  <div class="page-actions"><slot></slot></div>
</header>
