<script lang="ts">
  import { FolderOpen } from "@lucide/svelte";
  import type { Component } from "svelte";

  export let icon: Component = FolderOpen;
  export let title: string;
  export let description: string;
</script>

<div class="empty-state">
  {#if icon}
    {@const Icon = icon}
    <Icon size={30} strokeWidth={1.5} aria-hidden="true" />
  {/if}
  <h2>{title}</h2>
  <p>{description}</p>
  <slot></slot>
</div>
