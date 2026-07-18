<script lang="ts">
  import { Images, Info, PenLine, Settings } from "@lucide/svelte";
  import type { AppPage } from "$shared/types/app";

  export let page: AppPage;
  export let onNavigate: (page: AppPage) => void;

  const primary = [
    { id: "editor" as const, label: "编辑器", icon: PenLine },
    { id: "imageBed" as const, label: "图床", icon: Images }
  ];
  const secondary = [
    { id: "settings" as const, label: "设置", icon: Settings },
    { id: "about" as const, label: "关于", icon: Info }
  ];
</script>

<nav class="nav-rail" aria-label="主导航">
  <div class="nav-mark" title="Hexo Lite Editor"><img src="/favicon.png" alt="" /></div>
  {#each primary as item}
    <button
      class:active={page === item.id}
      class="nav-item"
      type="button"
      aria-current={page === item.id ? "page" : undefined}
      title={item.label}
      on:click={() => onNavigate(item.id)}
    >
      <svelte:component this={item.icon} size={20} strokeWidth={1.7} />
      <span>{item.label}</span>
    </button>
  {/each}
  <div class="nav-spacer"></div>
  {#each secondary as item}
    <button
      class:active={page === item.id}
      class="nav-item"
      type="button"
      aria-current={page === item.id ? "page" : undefined}
      title={item.label}
      on:click={() => onNavigate(item.id)}
    >
      <svelte:component this={item.icon} size={20} strokeWidth={1.7} />
      <span>{item.label}</span>
    </button>
  {/each}
</nav>
