<script lang="ts">
  import {
    ExternalLink,
    FilePlus2,
    FolderOpen,
    GitBranch,
    Hammer,
    ImagePlus,
    ListTree,
    MoreHorizontal,
    Play,
    Rocket,
    Save,
    Settings,
    Square,
    SquareTerminal
  } from "@lucide/svelte";

  export let hasProject = false;
  export let hasPost = false;
  export let runningServer = false;
  export let showLogDrawer = false;
  export let showTerminalPanel = false;
  export let onOpenProject: () => void = () => {};
  export let onNewPost: () => void = () => {};
  export let onSave: () => void = () => {};
  export let onOpenImageBed: () => void = () => {};
  export let onStartServer: () => void = () => {};
  export let onStopServer: () => void = () => {};
  export let onOpenPreview: () => void = () => {};
  export let onGenerate: () => void = () => {};
  export let onDeploy: () => void = () => {};
  export let onGitStatus: () => void = () => {};
  export let onToggleSettings: () => void = () => {};
  export let onToggleLog: () => void = () => {};
  export let onToggleTerminal: () => void = () => {};

  let menuOpen = false;

  function toggleLog() {
    onToggleLog();
    menuOpen = false;
  }

  function toggleTerminal() {
    onToggleTerminal();
    menuOpen = false;
  }
</script>

<header class="toolbar">
  <div class="actions actions-left">
    <button title="打开 Hexo 项目" on:click={onOpenProject}><FolderOpen size={17} />打开项目</button>
    <button title="新建文章 Ctrl+N" disabled={!hasProject} on:click={onNewPost}><FilePlus2 size={17} />新建</button>
    <button title="保存文章 Ctrl+S" disabled={!hasPost} on:click={onSave}><Save size={17} />保存</button>
  </div>

  <div class="actions actions-right">
    <button title="打开图床" on:click={onOpenImageBed}><ImagePlus size={17} />图床</button>
    {#if runningServer}
      <button title="停止 Hexo Server" disabled={!hasProject} on:click={onStopServer}><Square size={17} />停止</button>
    {:else}
      <button title="启动 Hexo Server" disabled={!hasProject} on:click={onStartServer}><Play size={17} />预览</button>
    {/if}
    <button title="在浏览器打开当前文章" disabled={!hasProject} on:click={onOpenPreview}><ExternalLink size={17} />浏览器</button>
    <button title="生成静态文件" disabled={!hasProject} on:click={onGenerate}><Hammer size={17} />生成</button>
    <button title="Git 状态" disabled={!hasProject} on:click={onGitStatus}><GitBranch size={17} />Git</button>
    <button title="发布博客" disabled={!hasProject} on:click={onDeploy}><Rocket size={17} />发布</button>

    <div class="menu-wrap">
      <button class="icon-only" class:active={menuOpen} title="更多" on:click={() => (menuOpen = !menuOpen)}>
        <MoreHorizontal size={18} />
      </button>
      {#if menuOpen}
        <div class="toolbar-menu">
          <button class:active={showTerminalPanel} on:click={toggleTerminal}><SquareTerminal size={16} />终端</button>
          <button class:active={showLogDrawer} on:click={toggleLog}><ListTree size={16} />日志</button>
        </div>
      {/if}
    </div>

    <button class="icon-only" title="设置 Ctrl+," on:click={onToggleSettings}><Settings size={18} /></button>
  </div>
</header>
