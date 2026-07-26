<script lang="ts">
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
    <h1 id="welcome-title">从博客目录，直接开始写作。</h1>
    <p>选择 Hexo 根目录后会自动检查项目结构、Node.js 与 Hexo 环境。没有 Node.js 也能编辑和管理文章，只会暂停预览与发布。</p>
    <div class="welcome-actions">
      <button class="button primary" type="button" on:click={onOpenProject}><FolderOpen size={16} />选择项目文件夹</button>
      <button class="button" type="button" on:click={() => onOpenSettings("general")}><Settings2 size={15} />先看写作设置</button>
    </div>
    <div class="welcome-checks">
      <div><CheckCircle2 size={17} /><span><strong>项目结构</strong><small>检查 _config.yml、package.json 与 source/_posts</small></span></div>
      <div><CheckCircle2 size={17} /><span><strong>运行环境</strong><small>检查 Node.js 和 Hexo；缺失时给出可继续使用的范围</small></span></div>
      <div><PenLine size={17} /><span><strong>写作边界</strong><small>图片目录、预览端口与发布流程可稍后在设置中调整</small></span></div>
    </div>
  </section>
  {#if recentProjects.length}
    <section class="welcome-recents" aria-label="最近项目"><h2>最近项目</h2>{#each recentProjects.slice(0, 5) as recent (recent.recentId)}<button type="button" disabled={!recent.available} on:click={() => onOpenRecentProject(recent.recentId)}><span><strong>{recent.name}</strong><small>{recent.displayPath}</small></span><small>{recent.available ? "打开" : "位置不可用"}</small></button>{/each}</section>
  {/if}
</div>
