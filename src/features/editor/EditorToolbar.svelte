<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { onDestroy, onMount } from "svelte";
  import {
    ChevronDown,
    FilePlus2,
    FolderOpen,
    ImagePlus,
    MoreHorizontal,
    PanelRightClose,
    PanelRightOpen,
    Rocket,
    Save,
    Server
  } from "@lucide/svelte";
  import { shortcutLabel } from "$platform/os";
  import type {
    PreviewServerView,
    ProjectSessionView,
    RecentProjectView,
    SettingsSectionId,
    TaskType
  } from "$shared/types/app";

  export let session: ProjectSessionView;
  export let recentProjects: RecentProjectView[] = [];
  export let previewVisible = true;
  export let previewServer: PreviewServerView | null = null;
  export let taskBusy = false;
  export let previewBusy = false;
  export let saving = false;
  export let saveDisabled = true;
  export let imageDisabled = true;
  export let onOpenProject: () => void = () => {};
  export let onOpenRecentProject: (recentId: string) => void = () => {};
  export let onPreview: () => void = () => {};
  export let onCreate: () => void = () => {};
  export let onSelectImages: () => void = () => {};
  export let onSave: () => void = () => {};
  export let onTogglePreview: () => void = () => {};
  export let onRunAdvanced: (task: TaskType) => void = () => {};
  export let onTogglePreviewServer: () => void = () => {};
  export let onOpenPreviewHome: () => void = () => {};
  export let onOpenSettings: (section?: SettingsSectionId) => void = () => {};
  export let onPublish: () => void = () => {};

  let projectMenuOpen = false;
  let advancedMenuOpen = false;

  onMount(() => {
    window.addEventListener("pointerdown", closeMenus);
    window.addEventListener("keydown", closeMenus);
  });

  onDestroy(() => {
    window.removeEventListener("pointerdown", closeMenus);
    window.removeEventListener("keydown", closeMenus);
  });

  function closeMenus(event: Event) {
    if (event instanceof KeyboardEvent && event.key !== "Escape") return;
    const target = event.target as HTMLElement;
    if (!(event instanceof KeyboardEvent) && target.closest?.(".project-switcher-wrap, .advanced-menu-wrap")) return;
    projectMenuOpen = false;
    advancedMenuOpen = false;
  }
</script>

<header class="editor-toolbar">
  <div class="project-switcher-wrap">
    <button class="project-switcher" type="button" aria-expanded={projectMenuOpen} on:click={() => { projectMenuOpen = !projectMenuOpen; advancedMenuOpen = false; }}>
      <FolderOpen size={17} /><span>{session.name}</span><ChevronDown size={14} />
    </button>
    {#if projectMenuOpen}
      <div class="project-menu quiet-menu">
        <div class="project-menu-current"><strong>{session.name}</strong><span>{session.displayPath}</span></div>
        {#each recentProjects.slice(0, 10) as recent (recent.recentId)}
          <button type="button" disabled={!recent.available} on:click={() => { projectMenuOpen = false; onOpenRecentProject(recent.recentId); }}><span>{recent.name}</span><small>{recent.available ? recent.displayPath : $ui("位置不可用")}</small></button>
        {/each}
        <button class="project-menu-open" type="button" on:click={() => { projectMenuOpen = false; onOpenProject(); }}><FolderOpen size={15} /><span>{$ui("打开其他博客")}</span></button>
      </div>
    {/if}
  </div>
  <div class="toolbar-spacer"></div>
  <button class="button quiet" type="button" disabled={previewBusy} on:click={onPreview}><Server size={16} />{previewBusy ? $ui("正在处理预览") : $ui("浏览器预览")}</button>
  {#if previewServer?.state === "running"}
    <button class="button quiet" type="button" disabled={previewBusy} on:click={onTogglePreviewServer}>{$ui("关闭后台预览")}</button>
  {/if}
  <button class="button quiet" type="button" title={$ui("新建（{p0}）", { p0: shortcutLabel("N") })} on:click={onCreate}><FilePlus2 size={16} />{$ui("新建")}</button>
  <button class="icon-button" type="button" disabled={imageDisabled} title={$ui("选择图片并插入")} aria-label={$ui("选择图片并插入")} on:click={onSelectImages}><ImagePlus size={17} /></button>
  <button class="button quiet" type="button" disabled={saveDisabled} title={$ui("保存（{p0}）", { p0: shortcutLabel("S") })} on:click={onSave}><Save size={16} />{saving ? $ui("保存中") : $ui("保存")}</button>
  <button
    class:active={previewVisible}
    class="icon-button preview-toggle"
    type="button"
    aria-pressed={previewVisible}
    title={`${previewVisible ? $ui("隐藏即时预览") : $ui("显示即时预览")} (${shortcutLabel("\\")})`}
    aria-label={previewVisible ? $ui("收起右侧即时预览") : $ui("展开右侧即时预览")}
    on:click={onTogglePreview}
  >
    {#if previewVisible}<PanelRightClose size={17} />{:else}<PanelRightOpen size={17} />{/if}
  </button>
  <div class="advanced-menu-wrap">
    <button class="icon-button" type="button" title={$ui("高级操作")} aria-label={$ui("高级操作")} aria-expanded={advancedMenuOpen} on:click={() => { advancedMenuOpen = !advancedMenuOpen; projectMenuOpen = false; }}><MoreHorizontal size={18} /></button>
    {#if advancedMenuOpen}
      <div class="advanced-menu quiet-menu">
        <button type="button" on:click={() => { advancedMenuOpen = false; onRunAdvanced("clean"); }}>{$ui("清理缓存")}</button>
        <button type="button" on:click={() => { advancedMenuOpen = false; onRunAdvanced("generate"); }}>{$ui("生成站点")}</button>
        <button type="button" on:click={() => { advancedMenuOpen = false; onRunAdvanced("deploy"); }}>{$ui("单独部署")}</button>
        <button type="button" on:click={() => { advancedMenuOpen = false; onRunAdvanced("gitStatus"); }}>{$ui("检查 Git 状态")}</button>
        <div class="menu-separator"></div>
        <button type="button" on:click={() => { advancedMenuOpen = false; onTogglePreview(); }}>{previewVisible ? $ui("隐藏即时预览") : $ui("显示即时预览")}</button>
        <button type="button" on:click={() => { advancedMenuOpen = false; onTogglePreviewServer(); }}>{previewServer?.state === "running" ? $ui("停止本地预览") : $ui("启动本地预览")}</button>
        <button type="button" on:click={() => { advancedMenuOpen = false; onOpenPreviewHome(); }}>{$ui("打开博客首页")}</button>
        <button type="button" on:click={() => { advancedMenuOpen = false; onOpenSettings("maintenance"); }}>{$ui("维护设置")}</button>
      </div>
    {/if}
  </div>
  <button class="button primary" type="button" disabled={taskBusy} title={$ui("发布（{p0}）", { p0: shortcutLabel("⇧P") })} on:click={onPublish}><Rocket size={16} />{taskBusy ? $ui("处理中") : $ui("发布")}</button>
</header>
