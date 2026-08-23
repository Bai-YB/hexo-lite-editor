<script lang="ts">
  import { Images, Info, PenLine, Settings } from "@lucide/svelte";
  import type { AppPage } from "$shared/types/app";
  import { translate } from "$shared/i18n";

  export let page: AppPage;
  export let onNavigate: (page: AppPage) => void;

  const primary = [
    { id: "editor" as const, label: "navigation.editor" as const, icon: PenLine },
    { id: "imageBed" as const, label: "navigation.imageBed" as const, icon: Images }
  ];
  const secondary = [
    { id: "settings" as const, label: "navigation.settings" as const, icon: Settings },
    { id: "about" as const, label: "navigation.about" as const, icon: Info }
  ];
</script>

<nav class="nav-rail" aria-label={$translate("navigation.label")}>
  <div class="nav-mark" title="Hexo Lite Editor"><img src="/favicon.png" alt="" /></div>
  {#each primary as item, index}
    <button
      class:active={page === item.id}
      class="nav-item"
      type="button"
      aria-current={page === item.id ? "page" : undefined}
      title={`${$translate(item.label)} (Ctrl+${index + 1})`}
      aria-keyshortcuts={`Control+${index + 1}`}
      on:click={() => onNavigate(item.id)}
    >
      <svelte:component this={item.icon} size={20} strokeWidth={1.7} />
      <span>{$translate(item.label)}</span>
    </button>
  {/each}
  <div class="nav-spacer"></div>
  {#each secondary as item, index}
    <button
      class:active={page === item.id}
      class="nav-item"
      type="button"
      aria-current={page === item.id ? "page" : undefined}
      title={`${$translate(item.label)} (Ctrl+${index + 3})`}
      aria-keyshortcuts={`Control+${index + 3}`}
      on:click={() => onNavigate(item.id)}
    >
      <svelte:component this={item.icon} size={20} strokeWidth={1.7} />
      <span>{$translate(item.label)}</span>
    </button>
  {/each}
</nav>
