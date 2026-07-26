<script lang="ts">
  import { Image, PenLine, RefreshCw, Rocket, SlidersHorizontal, Wrench } from "@lucide/svelte";
  import type { SettingsSectionId } from "$shared/types/app";

  export let sections: Array<{ id: SettingsSectionId; title: string; description: string }> = [];
  export let activeSection: SettingsSectionId;
  export let dirtySections: Record<SettingsSectionId, boolean>;
  export let onSelect: (section: SettingsSectionId) => void = () => {};
  export let onKeydown: (event: KeyboardEvent, index: number) => void = () => {};

  const icons: Record<SettingsSectionId, typeof SlidersHorizontal> = {
    general: SlidersHorizontal,
    editing: PenLine,
    images: Image,
    hexoPublish: Rocket,
    sync: RefreshCw,
    maintenance: Wrench
  };
</script>

<nav class="settings-nav" aria-label="设置分类">
  {#each sections as section, index (section.id)}
    {@const SectionIcon = icons[section.id]}
    <button
      type="button"
      class:active={activeSection === section.id}
      data-settings-section={section.id}
      aria-current={activeSection === section.id ? "page" : undefined}
      on:click={() => onSelect(section.id)}
      on:keydown={(event) => onKeydown(event, index)}
    >
      <span class="settings-nav-icon" aria-hidden="true"><SectionIcon size={16} /></span>
      <span class="settings-nav-copy"><strong>{section.title}</strong><small>{section.description}</small></span>
      {#if dirtySections[section.id]}<i class="settings-dirty-dot" aria-label="此分类有未保存更改"></i>{/if}
    </button>
  {/each}
</nav>
