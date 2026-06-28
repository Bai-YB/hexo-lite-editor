<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import dayjs from "dayjs";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Toolbar from "$lib/components/Toolbar.svelte";
  import PostList from "$lib/components/PostList.svelte";
  import MarkdownEditor from "$lib/components/MarkdownEditor.svelte";
  import MarkdownPreview from "$lib/components/MarkdownPreview.svelte";
  import LogDrawer from "$lib/components/LogDrawer.svelte";
  import TerminalPanel from "$lib/components/TerminalPanel.svelte";
  import SettingsPanel from "$lib/components/SettingsPanel.svelte";
  import ImageBedPanel from "$lib/components/ImageBedPanel.svelte";
  import StatusBar from "$lib/components/StatusBar.svelte";
  import { loadSettings, saveSettings } from "$lib/api/config";
  import {
    generateAndDeploy,
    generateSite,
    getGitStatus,
    startHexoServer,
    stopHexoServer
  } from "$lib/api/hexo";
  import { openCurrentPostPreview } from "$lib/api/preview";
  import { backupPostContent, createPost, readPostContent, savePostContent, scanPosts, slugifyTitle } from "$lib/api/posts";
  import { validateProject } from "$lib/api/project";
  import { runTerminalCommand } from "$lib/api/terminal";
  import {
    saveClipboardImageToProject,
    uploadClipboardImageToCloudflareImgBed
  } from "$lib/api/uploader";
  import { defaultSettings, type AppSettings, type ColorScheme } from "$lib/types/settings";
  import type { CommandResult } from "$lib/types/command";
  import type { HexoProject } from "$lib/types/project";
  import type { Post } from "$lib/types/post";

  const SIDEBAR_MIN = 230;
  const SIDEBAR_MAX = 520;
  const EDITOR_MIN = 320;
  const PREVIEW_MIN = 320;
  const RESIZE_HANDLE_WIDTH = 14;

  let project: HexoProject | null = null;
  let posts: Post[] = [];
  let activePost: Post | null = null;
  let content = "";
  let settings: AppSettings = structuredClone(defaultSettings);
  let saveStatus = "已保存";
  let lastSavedAt = "";
  let logs: string[] = [];
  let runningServer = false;
  let showSettings = false;
  let showImageBed = false;
  let showLogDrawer = false;
  let showTerminalPanel = false;
  let terminalRunning = false;
  let terminalOutput: string[] = [];
  let previewRefreshToken = String(Date.now());
  let loading = false;
  let activeResizeKind: "sidebar" | "preview" | "terminal" | null = null;
  let sidebarWidth = defaultSettings.layout.sidebarWidth;
  let previewWidthState = defaultSettings.layout.previewWidth;
  let editorRef: MarkdownEditor;
  let previewRef: MarkdownPreview;
  let workspaceElement: HTMLElement;
  let workspaceWidth = 0;
  let previousWorkspaceWidth = 0;
  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null;
  let mediaQuery: MediaQueryList | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let layoutDirty = false;
  let closeRequestedUnlisten: (() => void) | null = null;
  let closingAfterLayoutSave = false;

  $: words = content.replace(/\s+/g, "").length;
  $: activeColorScheme = effectiveColorSchemeFor(settings.appearance.themeMode);
  $: previewWidth = currentPreviewWidth(workspaceWidth, sidebarWidth, previewWidthState);
  $: workspaceColumns = `${sidebarWidth}px minmax(${EDITOR_MIN}px, 1fr) ${previewWidth}px`;
  $: workspaceRows = showTerminalPanel
    ? `minmax(0, 1fr) ${settings.layout.logPanelHeight}px`
    : "minmax(0, 1fr)";
  $: workspaceBottomOffset = showTerminalPanel ? settings.layout.logPanelHeight : 0;
  $: sidebarHandleLeft = Math.max(0, sidebarWidth - RESIZE_HANDLE_WIDTH / 2);
  $: previewHandleLeft = Math.max(0, workspaceWidth - previewWidth - RESIZE_HANDLE_WIDTH / 2);
  $: settings && applyTheme();

  onMount(async () => {
    const loaded = await loadSettings();
    settings = {
      ...loaded,
      appearance: {
        ...loaded.appearance,
        colorScheme: effectiveColorSchemeFor(loaded.appearance.themeMode)
      },
      layout: {
        ...loaded.layout,
        showLogPanel: false
      }
    };
    sidebarWidth = settings.layout.sidebarWidth;
    previewWidthState = settings.layout.previewWidth;
    void saveSettings(settings).catch((error) => appendLog(`Settings migration save failed: ${error}`));
    applyTheme();
    mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    mediaQuery.addEventListener("change", applyTheme);
    window.addEventListener("keydown", handleGlobalKeydown);
    window.addEventListener("beforeunload", persistCurrentLayoutBeforeUnload);
    resizeObserver = new ResizeObserver(([entry]) => {
      handleWorkspaceResize(entry.contentRect.width);
    });
    if (workspaceElement) {
      handleWorkspaceResize(workspaceElement.getBoundingClientRect().width, true);
      resizeObserver.observe(workspaceElement);
    }
    try {
      const appWindow = getCurrentWindow();
      closeRequestedUnlisten = await appWindow.onCloseRequested(async (event) => {
        if (closingAfterLayoutSave || !layoutDirty) return;
        event.preventDefault();
        closingAfterLayoutSave = true;
        try {
          await persistCurrentLayoutSafely();
        } finally {
          closeRequestedUnlisten?.();
          closeRequestedUnlisten = null;
          const destroy = (appWindow as unknown as { destroy?: () => Promise<void> }).destroy;
          if (destroy) await destroy.call(appWindow);
          else await appWindow.close();
        }
      });
    } catch {
      closeRequestedUnlisten = null;
    }
    if (settings.general.openRecentProjectOnStart && settings.recentProjects[0]) {
      await openProject(settings.recentProjects[0], false);
    }
  });

  onDestroy(() => {
    if (autoSaveTimer) clearTimeout(autoSaveTimer);
    mediaQuery?.removeEventListener("change", applyTheme);
    window.removeEventListener("keydown", handleGlobalKeydown);
    window.removeEventListener("beforeunload", persistCurrentLayoutBeforeUnload);
    resizeObserver?.disconnect();
    closeRequestedUnlisten?.();
  });

  function handleGlobalKeydown(event: KeyboardEvent) {
    if ((event.ctrlKey || event.metaKey) && event.key === ",") {
      event.preventDefault();
      showSettings = true;
    }
    if (event.key === "Escape") {
      if (showSettings) showSettings = false;
      else if (showImageBed) showImageBed = false;
      else if (showLogDrawer) showLogDrawer = false;
      else if (showTerminalPanel) showTerminalPanel = false;
    }
  }

  function applyTheme() {
    if (typeof document === "undefined") return;
    const dark = isDarkTheme(settings.appearance.themeMode);
    const scheme = effectiveColorSchemeFor(settings.appearance.themeMode);
    document.documentElement.classList.toggle("dark", dark);
    document.documentElement.classList.toggle("compact", settings.appearance.compactMode);
    document.documentElement.classList.remove(
      "scheme-classic",
      "scheme-vscode-light",
      "scheme-vscode-dark",
      "scheme-monokai",
      "scheme-one-dark",
      "scheme-solarized-light"
    );
    document.documentElement.classList.add(`scheme-${scheme}`);
    document.documentElement.style.setProperty("--font-scale", String(settings.appearance.fontScale));
  }

  function isDarkTheme(themeMode = settings.appearance.themeMode) {
    const systemDark = window.matchMedia?.("(prefers-color-scheme: dark)").matches ?? false;
    return themeMode === "dark" || (themeMode === "system" && systemDark);
  }

  function effectiveColorSchemeFor(themeMode = settings.appearance.themeMode): ColorScheme {
    return isDarkTheme(themeMode) ? "vscode-dark" : "vscode-light";
  }

  function handleWorkspaceResize(nextWidth: number, initial = false) {
    const roundedWidth = Math.round(nextWidth);
    if (!roundedWidth) return;
    if (initial || !previousWorkspaceWidth) {
      workspaceWidth = roundedWidth;
      previousWorkspaceWidth = roundedWidth;
      return;
    }

    const oldWidth = previousWorkspaceWidth;
    workspaceWidth = roundedWidth;
    previousWorkspaceWidth = roundedWidth;

    if (activeResizeKind || Math.abs(roundedWidth - oldWidth) < 2) return;

    const ratio = roundedWidth / oldWidth;
    const maxSidebar = Math.max(SIDEBAR_MIN, Math.min(SIDEBAR_MAX, roundedWidth - EDITOR_MIN - PREVIEW_MIN));
    sidebarWidth = clamp(Math.round(sidebarWidth * ratio), SIDEBAR_MIN, maxSidebar);
    if (previewWidthState > 0) {
      previewWidthState = clamp(Math.round(previewWidthState * ratio), PREVIEW_MIN, maxPreviewWidth(roundedWidth, sidebarWidth));
    }
    layoutDirty = true;
  }

  function resolvePreviewWidth(nextWorkspaceWidth: number, nextSidebarWidth: number, nextPreviewWidth: number) {
    if (nextPreviewWidth > 0) return nextPreviewWidth;
    const available = Math.max(0, nextWorkspaceWidth - nextSidebarWidth);
    return Math.max(PREVIEW_MIN, Math.floor(available / 2));
  }

  function currentPreviewWidth(nextWorkspaceWidth: number, nextSidebarWidth: number, nextPreviewWidth: number) {
    return clamp(
      resolvePreviewWidth(nextWorkspaceWidth, nextSidebarWidth, nextPreviewWidth),
      PREVIEW_MIN,
      maxPreviewWidth(nextWorkspaceWidth, nextSidebarWidth)
    );
  }

  function maxPreviewWidth(nextWorkspaceWidth: number, nextSidebarWidth: number) {
    if (!nextWorkspaceWidth) return Number.POSITIVE_INFINITY;
    return Math.max(PREVIEW_MIN, nextWorkspaceWidth - nextSidebarWidth - EDITOR_MIN);
  }

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, value));
  }

  function updateLayout(partial: Partial<AppSettings["layout"]>, persist = false) {
    settings = {
      ...settings,
      layout: {
        ...settings.layout,
        ...partial
      }
    };
    if (persist) void saveSettings(settings).catch((error) => appendLog(`Settings save failed: ${error}`));
  }

  function measuredPreviewWidth() {
    return Math.round(workspaceElement?.querySelector(".right-rail")?.getBoundingClientRect().width || previewWidth);
  }

  function layoutSnapshot() {
    return {
      ...settings.layout,
      sidebarWidth,
      previewWidth: previewWidthState > 0 ? previewWidthState : 0
    };
  }

  async function persistCurrentLayout() {
    if (!layoutDirty) return;
    settings = {
      ...settings,
      layout: layoutSnapshot()
    };
    layoutDirty = false;
    await saveSettings(settings);
  }

  async function persistCurrentLayoutSafely() {
    try {
      await persistCurrentLayout();
    } catch (error) {
      appendLog(`Layout save failed before close: ${error}`);
    }
  }

  function persistCurrentLayoutBeforeUnload() {
    void persistCurrentLayoutSafely();
  }

  function startResize(kind: "sidebar" | "preview" | "terminal", event: MouseEvent) {
    event.preventDefault();
    const rect = workspaceElement.getBoundingClientRect();
    const startX = event.clientX;
    const startY = event.clientY;
    const startSidebarWidth = sidebarWidth;
    const startPreviewWidth = measuredPreviewWidth();
    const startLogHeight = settings.layout.logPanelHeight;
    activeResizeKind = kind;
    document.body.classList.add("resizing-layout");
    document.body.classList.toggle("resizing-horizontal", kind === "terminal");
    document.body.classList.toggle("resizing-vertical", kind !== "terminal");
    let nextLayout = {
      ...settings.layout,
      sidebarWidth: startSidebarWidth,
      previewWidth: startPreviewWidth
    };

    const handleMove = (moveEvent: MouseEvent) => {
      if (kind === "sidebar") {
        const delta = moveEvent.clientX - startX;
        const max = Math.max(SIDEBAR_MIN, rect.width - startPreviewWidth - EDITOR_MIN);
        sidebarWidth = clamp(startSidebarWidth + delta, SIDEBAR_MIN, Math.min(SIDEBAR_MAX, max));
        previewWidthState = startPreviewWidth;
        nextLayout = {
          ...nextLayout,
          sidebarWidth,
          previewWidth: previewWidthState
        };
        layoutDirty = true;
        return;
      }

      if (kind === "preview") {
        const delta = moveEvent.clientX - startX;
        const max = Math.max(PREVIEW_MIN, rect.width - startSidebarWidth - EDITOR_MIN);
        previewWidthState = clamp(startPreviewWidth - delta, PREVIEW_MIN, max);
        nextLayout = {
          ...nextLayout,
          previewWidth: previewWidthState
        };
        layoutDirty = true;
        return;
      }

      const delta = startY - moveEvent.clientY;
      nextLayout = {
        ...nextLayout,
        logPanelHeight: clamp(startLogHeight + delta, 160, Math.min(520, window.innerHeight - 160))
      };
      updateLayout({ logPanelHeight: nextLayout.logPanelHeight });
    };

    const handleUp = () => {
      activeResizeKind = null;
      document.body.classList.remove("resizing-layout", "resizing-horizontal", "resizing-vertical");
      document.removeEventListener("mousemove", handleMove);
      document.removeEventListener("mouseup", handleUp);
      settings = {
        ...settings,
        layout: nextLayout
      };
      sidebarWidth = nextLayout.sidebarWidth;
      previewWidthState = nextLayout.previewWidth;
      layoutDirty = false;
      void saveSettings(settings);
    };

    document.addEventListener("mousemove", handleMove);
    document.addEventListener("mouseup", handleUp, { once: true });
  }

  function handleEditorScroll(ratio: number) {
    previewRef?.scrollToRatio(ratio);
  }

  function toggleLogDrawer() {
    showLogDrawer = !showLogDrawer;
    if (showLogDrawer) {
      showSettings = false;
      showImageBed = false;
    }
  }

  function toggleTerminalPanel() {
    showTerminalPanel = !showTerminalPanel;
  }

  function toggleSettingsPanel() {
    showSettings = !showSettings;
    if (showSettings) {
      showImageBed = false;
      showLogDrawer = false;
    }
  }

  function toggleImageBedPanel() {
    showImageBed = !showImageBed;
    if (showImageBed) {
      showSettings = false;
      showLogDrawer = false;
    }
  }

  function insertImageBedMarkdown(markdown: string) {
    if (!activePost) {
      appendLog("No active post selected for image insertion.");
      return;
    }
    editorRef?.insertText(markdown);
  }

  async function openProjectDialog() {
    const selected = await open({ directory: true, multiple: false, title: "选择 Hexo 项目目录" });
    if (typeof selected === "string") {
      await openProject(selected);
    }
  }

  async function openProject(path: string, log = true) {
    loading = true;
    try {
      project = await validateProject(path);
      settings = {
        ...settings,
        recentProjects: [project.rootPath, ...settings.recentProjects.filter((item) => item !== project?.rootPath)].slice(0, 8)
      };
      await saveSettings(settings);
      if (log) appendLog(`打开项目: ${project.rootPath}${project.warnings.length ? `\n提示: ${project.warnings.join("、")}` : ""}`);
      await refreshPosts();
    } catch (error) {
      appendLog(`打开项目失败: ${error}`);
    } finally {
      loading = false;
    }
  }

  async function refreshPosts() {
    if (!project) return;
    posts = await scanPosts(project.rootPath);
    if (activePost) {
      activePost = posts.find((post) => post.filePath === activePost?.filePath) ?? activePost;
    }
  }

  async function selectPost(post: Post) {
    if (activePost?.isDirty) {
      const keepGoing = confirm("当前文章尚未保存，是否继续切换？");
      if (!keepGoing) return;
    }
    activePost = post;
    content = await readPostContent(post.filePath);
    saveStatus = "已保存";
    lastSavedAt = "";
  }

  function handleContentChange(value: string) {
    content = value;
    if (!activePost) return;
    activePost = { ...activePost, isDirty: true };
    saveStatus = "未保存";
    scheduleAutoSave();
  }

  function scheduleAutoSave() {
    if (!settings.general.autoSave || !activePost) return;
    if (autoSaveTimer) clearTimeout(autoSaveTimer);
    autoSaveTimer = setTimeout(() => {
      saveActivePost();
    }, settings.general.autoSaveInterval);
  }

  async function saveActivePost() {
    if (!activePost) return;
    saveStatus = "保存中";
    try {
      if (settings.general.backupBeforeSave && project) {
        const backupPath = await backupPostContent(project.rootPath, activePost.filePath);
        appendLog(`保存前备份: ${backupPath}`);
      }
      await savePostContent(activePost.filePath, content);
      saveStatus = "已保存";
      lastSavedAt = dayjs().format("HH:mm:ss");
      activePost = { ...activePost, isDirty: false };
      await refreshPosts();
    } catch (error) {
      saveStatus = "保存失败";
      appendLog(`保存失败: ${error}`);
    }
  }

  async function newPost() {
    if (!project) return;
    const title = prompt("文章标题", "新的文章");
    if (!title) return;
    const fileName = prompt("文件名", slugifyTitle(title));
    if (!fileName) return;
    const categories = prompt("分类，多个用逗号分隔", "")?.split(",").map((item) => item.trim()).filter(Boolean) ?? [];
    const tags = prompt("标签，多个用逗号分隔", "")?.split(",").map((item) => item.trim()).filter(Boolean) ?? [];

    try {
      const filePath = await createPost(project.rootPath, {
        title,
        fileName,
        categories,
        tags,
        date: dayjs().format("YYYY-MM-DD HH:mm:ss")
      });
      await refreshPosts();
      const created = posts.find((post) => post.filePath === filePath);
      if (created) await selectPost(created);
      appendLog(`新建文章: ${filePath}`);
    } catch (error) {
      appendLog(`新建文章失败: ${error}`);
    }
  }

  async function uploadImageFromClipboard(file: File, data: number[]) {
    if (settings.uploader.defaultType === "cloudflare-imgbed") {
      return uploadClipboardImageToCloudflareImgBed(
        settings.uploader.apiUrl,
        settings.uploader.token,
        file.name || undefined,
        file.type || undefined,
        data
      );
    }
    if (!project) throw new Error("请先打开 Hexo 项目。");
    return saveClipboardImageToProject(project.rootPath, file.name || undefined, file.type || undefined, data);
  }

  async function handlePastedImages(files: File[]) {
    if (!project || !activePost) {
      appendLog("请先打开 Hexo 项目并选择文章，再粘贴图片。");
      return;
    }
    try {
      const inserted: string[] = [];
      for (const file of files) {
        const data = Array.from(new Uint8Array(await file.arrayBuffer()));
        const result = await uploadImageFromClipboard(file, data);
        inserted.push(result.markdown);
        appendLog(`Clipboard image uploaded: ${result.filePath}`);
      }
      if (inserted.length) {
        editorRef?.insertText(`\n${inserted.join("\n")}\n`);
      }
    } catch (error) {
      appendLog(`粘贴图片失败: ${error}`);
    }
  }

  async function startServer() {
    if (!project) return;
    try {
      const message = await startHexoServer(project.rootPath);
      runningServer = true;
      appendLog(message);
      await waitForHexoServer();
      await openCurrentPreview();
    } catch (error) {
      appendLog(`启动 Hexo Server 失败: ${error}`);
    }
  }

  async function stopServer() {
    try {
      const message = await stopHexoServer();
      runningServer = false;
      appendLog(message);
    } catch (error) {
      appendLog(`停止 Hexo Server 失败: ${error}`);
    }
  }

  async function runCommand(action: "generate" | "deploy" | "git") {
    if (!project) return;
    loading = true;
    try {
      if (settings.publish.saveBeforePublish && activePost?.isDirty && action !== "git") {
        await saveActivePost();
      }
      const result =
        action === "generate"
          ? await generateSite(project.rootPath)
          : action === "deploy"
            ? await generateAndDeploy(project.rootPath)
            : await getGitStatus(project.rootPath);
      appendCommandResult(result);
    } catch (error) {
      appendLog(`命令执行失败: ${error}`);
    } finally {
      loading = false;
    }
  }

  async function openCurrentPreview() {
    try {
      if (project && !runningServer) {
        appendLog("Hexo Server is not running, starting local preview...");
        const message = await startHexoServer(project.rootPath);
        runningServer = true;
        appendLog(message);
      }
      if (project) {
        await waitForHexoServer();
      }
      await openCurrentPostPreview(project, activePost, content);
    } catch (error) {
      appendLog(`打开预览失败: ${error}`);
    }
  }

  function refreshPreview() {
    previewRefreshToken = String(Date.now());
  }

  async function waitForHexoServer(timeoutMs = 15000) {
    const deadline = Date.now() + timeoutMs;
    let lastError: unknown = null;
    while (Date.now() < deadline) {
      try {
        await fetch("http://localhost:4000", {
          cache: "no-store",
          mode: "no-cors"
        });
        return;
      } catch (error) {
        lastError = error;
        await new Promise((resolve) => setTimeout(resolve, 500));
      }
    }
    throw new Error(`Hexo Server did not become ready in time: ${lastError ?? "timeout"}`);
  }

  async function runTerminal(command: string) {
    if (!project) {
      terminalOutput = ["请先打开 Hexo 项目。", ...terminalOutput];
      return;
    }
    terminalRunning = true;
    terminalOutput = [`$ ${command}\n执行中...`, ...terminalOutput];
    try {
      const result = await runTerminalCommand(project.rootPath, command);
      const output = `$ ${result.command}\n状态: ${result.success ? "成功" : `失败 ${result.code ?? ""}`}\n${result.stdout}${result.stderr ? `\n${result.stderr}` : ""}`;
      terminalOutput = [output, ...terminalOutput.slice(1)].slice(0, 100);
      appendCommandResult(result);
    } catch (error) {
      const output = `$ ${command}\n状态: 执行失败\n${error}`;
      terminalOutput = [output, ...terminalOutput.slice(1)].slice(0, 100);
      appendLog(`终端命令失败: ${error}`);
    } finally {
      terminalRunning = false;
    }
  }

  async function persistSettings(next: AppSettings) {
    settings = {
      ...next,
      appearance: {
        ...next.appearance,
        colorScheme: effectiveColorSchemeFor(next.appearance.themeMode)
      }
    };
    sidebarWidth = next.layout.sidebarWidth;
    previewWidthState = next.layout.previewWidth;
    layoutDirty = false;
    await saveSettings(settings);
    applyTheme();
  }

  function appendCommandResult(result: CommandResult) {
    appendLog(
      `$ ${result.command}\n状态: ${result.success ? "成功" : `失败 ${result.code ?? ""}`}\n${result.stdout}${result.stderr ? `\n${result.stderr}` : ""}`
    );
  }

  function appendLog(message: string) {
    logs = [`[${dayjs().format("HH:mm:ss")}] ${message}`, ...logs].slice(0, settings.general.maxLogCount);
  }
</script>

<svelte:head>
  <title>Hexo Lite Editor</title>
</svelte:head>

<div class="app-shell">
  <Toolbar
    hasProject={Boolean(project)}
    hasPost={Boolean(activePost)}
    {runningServer}
    {showLogDrawer}
    {showTerminalPanel}
    onOpenProject={openProjectDialog}
    onNewPost={newPost}
    onSave={saveActivePost}
    onOpenImageBed={toggleImageBedPanel}
    onStartServer={startServer}
    onStopServer={stopServer}
    onOpenPreview={openCurrentPreview}
    onGenerate={() => runCommand("generate")}
    onDeploy={() => runCommand("deploy")}
    onGitStatus={() => runCommand("git")}
    onToggleSettings={toggleSettingsPanel}
    onToggleLog={toggleLogDrawer}
    onToggleTerminal={toggleTerminalPanel}
  />

  <main
    class="workspace"
    bind:this={workspaceElement}
    style={`grid-template-columns: ${workspaceColumns}; grid-template-rows: ${workspaceRows}; --workspace-bottom-offset: ${workspaceBottomOffset}px;`}
  >
    <PostList
      {posts}
      activePath={activePost?.filePath ?? ""}
      showCover={settings.postList.showCover && settings.appearance.showPostCover}
      coverSourcePriority={settings.postList.coverSourcePriority}
      onSelect={selectPost}
      onNewPost={newPost}
    />

    <section class="editor-pane">
      <div class="editor-header">
        <div>
          <h1>{activePost?.title ?? "选择一篇文章开始编辑"}</h1>
          <span>{activePost?.filePath ?? "打开 Hexo 项目后，source/_posts 中的文章会显示在左侧。"}</span>
        </div>
        {#if loading}
          <strong>处理中...</strong>
        {:else}
          <strong>{saveStatus}</strong>
        {/if}
      </div>
      <div class="editor-body">
        {#if activePost}
          {#key `${settings.editor.fontSize}-${settings.editor.lineHeight}-${settings.editor.showLineNumbers}-${settings.editor.lineWrapping}-${settings.editor.highlightActiveLine}-${settings.editor.tabSize}-${activeColorScheme}`}
            <MarkdownEditor
              bind:this={editorRef}
              {content}
              colorScheme={activeColorScheme}
              fontSize={settings.editor.fontSize}
              lineHeight={settings.editor.lineHeight}
              showLineNumbers={settings.editor.showLineNumbers}
              lineWrapping={settings.editor.lineWrapping}
              highlightLine={settings.editor.highlightActiveLine}
              tabSize={settings.editor.tabSize}
              onChange={handleContentChange}
              onSave={saveActivePost}
              onNewPost={newPost}
              onScrollRatio={handleEditorScroll}
              onPasteImages={handlePastedImages}
            />
          {/key}
        {:else}
          <div class="welcome-panel">
            <h1>打开一个已有 Hexo 博客项目</h1>
            <p>应用会读取真实 Markdown 文件，不写入数据库，也不会主动修改主题、脚手架或 Hexo 配置。</p>
            <button on:click={openProjectDialog}>打开项目</button>
            {#if settings.recentProjects.length}
              <div class="recent-projects">
                <strong>最近项目</strong>
                {#each settings.recentProjects as path}
                  <button on:click={() => openProject(path)}>{path}</button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      </div>
    </section>

    <div class="right-rail">
      <MarkdownPreview
        bind:this={previewRef}
        content={content}
        refreshToken={previewRefreshToken}
        onRefresh={refreshPreview}
        onOpenPreview={openCurrentPreview}
      />
    </div>

    <button
      class="resize-handle vertical"
      class:active={activeResizeKind === "sidebar"}
      aria-label="Resize post list"
      title="拖拽调整文章栏宽度"
      style:left={`${sidebarHandleLeft}px`}
      on:mousedown={(event) => startResize("sidebar", event)}
    ></button>

    <button
      class="resize-handle vertical"
      class:active={activeResizeKind === "preview"}
      aria-label="Resize editor and preview"
      title="拖拽调整编辑器和预览宽度"
      style:left={`${previewHandleLeft}px`}
      on:mousedown={(event) => startResize("preview", event)}
    ></button>

    {#if showTerminalPanel}
      <div class="workspace-terminal-panel">
        <button
          class="resize-handle horizontal"
          class:active={activeResizeKind === "terminal"}
          aria-label="Resize terminal panel"
          title="拖拽调整终端高度"
          on:mousedown={(event) => startResize("terminal", event)}
        ></button>
        <TerminalPanel
          projectPath={project?.rootPath ?? ""}
          output={terminalOutput}
          running={terminalRunning}
          onRun={runTerminal}
          onClear={() => (terminalOutput = [])}
          onClose={() => (showTerminalPanel = false)}
        />
      </div>
    {/if}

    {#if showSettings}
      <SettingsPanel {settings} {project} onChange={persistSettings} onClose={() => (showSettings = false)} />
    {/if}

    {#if showImageBed}
      <ImageBedPanel
        {settings}
        onClose={() => (showImageBed = false)}
        onInsertMarkdown={insertImageBedMarkdown}
        onChange={persistSettings}
        onLog={appendLog}
      />
    {/if}

    {#if showLogDrawer}
      <LogDrawer {logs} onClear={() => (logs = [])} onClose={() => (showLogDrawer = false)} />
    {/if}
  </main>

  <StatusBar
    projectName={project?.name ?? "未打开项目"}
    fileName={activePost?.fileName ?? "未选择文章"}
    {saveStatus}
    {words}
    {lastSavedAt}
  />
</div>
