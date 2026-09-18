<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { CheckCircle2, FolderOpen, PenLine, Settings2 } from "@lucide/svelte";
  import type { RecentProjectView, SettingsSectionId } from "$shared/types/app";

  export let recentProjects: RecentProjectView[] = [];
  export let onOpenProject: () => void = () => {};
  export let onOpenRecentProject: (recentId: string) => void = () => {};
  export let onOpenSettings: (section?: SettingsSectionId) => void = () => {};
</script>

<div class="editor-welcome">
  <section class="welcome-hero" aria-labelledby="welcome-title">
    <div class="welcome-brand"><img src="/favicon.png" alt="" /><span>HEXO LITE</span></div>
    <h1 id="welcome-title">{$ui("选择一个 Hexo 博客目录就能开始写。")}</h1>
    <p>{$ui("打开后会先检查项目结构和环境。没有装 Node.js 也能正常写作，但网站预览和发布用不了。")}</p>
    <div class="welcome-actions">
      <button class="button primary" type="button" on:click={onOpenProject}><FolderOpen size={16} />{$ui("选择项目文件夹")}</button>
      <button class="button" type="button" on:click={() => onOpenSettings("general")}><Settings2 size={15} />{$ui("先看写作设置")}</button>
    </div>
    <div class="welcome-checks">
      <div><CheckCircle2 size={17} /><span><strong>{$ui("项目结构")}</strong><small>{$ui("检查 _config.yml、package.json 与 source/_posts")}</small></span></div>
      <div><CheckCircle2 size={17} /><span><strong>{$ui("运行环境")}</strong><small>{$ui("检查 Node.js 和 Hexo，缺了会说明还能用什么")}</small></span></div>
      <div><PenLine size={17} /><span><strong>{$ui("写作范围")}</strong><small>{$ui("图片目录、预览端口、发布流程都能在设置里改")}</small></span></div>
    </div>
  </section>
  {#if recentProjects.length}
    <section class="welcome-recents" aria-label={$ui("最近项目")}><h2>{$ui("最近项目")}</h2>{#each recentProjects.slice(0, 5) as recent (recent.recentId)}<button type="button" disabled={!recent.available} on:click={() => onOpenRecentProject(recent.recentId)}><span><strong>{recent.name}</strong><small>{recent.displayPath}</small></span><small>{recent.available ? $ui("打开") : $ui("位置不可用")}</small></button>{/each}</section>
  {/if}
</div>
