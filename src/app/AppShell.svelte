<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { LoaderCircle, X } from "@lucide/svelte";
  import TitleBar from "./TitleBar.svelte";
  import NavRail from "./NavRail.svelte";
  import LoadingState from "$shared/components/LoadingState.svelte";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import PageTransition from "$shared/components/PageTransition.svelte";
  import { isTauri, normalizeError, platform } from "$platform/tauri";
  import { defaultConfig } from "$shared/types/app";
  import type {
    AppConfigV3,
    AppPage,
    ArticleSummary,
    CloseWindowState,
    ProjectSessionView,
    PreviewServerView,
    RecentProjectView,
    SettingsSectionId,
    TaskEvent
  } from "$shared/types/app";
  import { EditorSessionStore } from "$features/editor/EditorSessionStore";
  import type { SettingsController } from "$features/settings/controller";

  const pageLoaders: Record<AppPage, () => Promise<{ default: any }>> = {
    editor: () => import("$features/editor/EditorPage.svelte"),
    imageBed: () => import("$features/image-bed/ImageBedPage.svelte"),
    settings: () => import("$features/settings/SettingsPage.svelte"),
    about: () => import("$features/about/AboutPage.svelte")
  };

  let page: AppPage = "editor";
  let pagePromise = pageLoaders.editor();
  let config: AppConfigV3 = structuredClone(defaultConfig);
  let configLoaded = false;
  let session: ProjectSessionView | null = null;
  let articles: ArticleSummary[] = [];
  const editorStore = new EditorSessionStore(platform.saveDocument);
  let settingsController: SettingsController | null = null;
  let recentProjects: RecentProjectView[] = [];
  let dirty = false;
  let activeArticleId: string | null = null;
  let maximized = false;
  let taskEvents: TaskEvent[] = [];
  let unlistenTask: (() => void) | undefined;
  let unlistenPreview: (() => void) | undefined;
  let unlistenSync: (() => void) | undefined;
  let unlistenSyncPhase: (() => void) | undefined;
  let unlistenRescan: (() => void) | undefined;
  let unlistenClose: (() => void) | undefined;
  let configTimer: ReturnType<typeof setTimeout> | undefined;
  let notice = "";
  let noticeSeverity: "info" | "error" = "info";
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;
  let guardAction: (() => void | Promise<void>) | null = null;
  let guardDescription = "";
  let guardBusy = false;
  let guardSource: "editor" | "settings" | "both" = "editor";
  let guardIsClosing = false;
  let allowWindowClose = false;
  let closeWindowState: CloseWindowState = { hasUnsavedChanges: false, isClosing: false };
  let previewServer: PreviewServerView | null = null;
  let previewBusy = false;
  let publishing = false;
  let pendingImageUploads = 0;
  let settingsInitialSection: SettingsSectionId | null = null;
  let configRevision = 0;

  const unsubscribeEditor = editorStore.subscribe((state) => {
    dirty = state.dirty;
    activeArticleId = state.snapshot?.articleId ?? null;
  });

  $: pagePromise = pageLoaders[page]();
  $: activeTask = findActiveTask(taskEvents);
  $: serverActive = previewServer?.state === "running";
  $: activeDocumentTitle = articles.find((article) => article.articleId === activeArticleId)?.title ?? "";

  onMount(async () => {
    window.addEventListener("keydown", handleShortcut);
    try {
      const loaded = await platform.loadConfig();
      config = loaded.config;
      applyTheme(config.appearance.themeMode);
      configLoaded = true;
      if (loaded.warnings.length) showNotice(loaded.warnings[0]);
      if (config.update.checkOnStart) {
        void platform
          .checkUpdate()
          .then((update) => {
            if (update.hasUpdate) showNotice(`发现新版本 ${update.latestVersion}，可在“关于”中查看。`);
          })
          .catch((error) => console.info("启动更新检查未完成", normalizeError(error).message));
      }
      recentProjects = await platform.listRecentProjects();
      if (config.general.openRecentProjectOnStart) {
        const recent = await platform.reopenRecentProject();
        if (recent) acceptProject(recent.session, recent.articles);
      }
    } catch (error) {
      configLoaded = true;
      showNotice(normalizeError(error).message);
    }
    unlistenTask = await platform.onTaskEvent((event) => {
      taskEvents = appendTaskEvent(taskEvents, event);
      if (event.kind === "finished" && event.success === false) {
        showNotice("任务执行失败，请检查 Hexo 项目配置或网络连接后重试。", "error");
      }
    });
    unlistenPreview = await platform.onPreviewStatus((view) => {
      if (view.projectId === session?.projectId && view.sessionGeneration === session.generation) {
        previewServer = view;
        if (view.state === "error" && view.error) showNotice(view.error.message, "error");
      }
    });
    unlistenSync = await platform.onContentSyncStatus((view) => {
      if (["offline", "authRequired", "remoteAhead", "conflict", "error"].includes(view.status)) {
        showNotice(view.message || `内容同步：${view.status}`, "error");
      }
    });
    unlistenSyncPhase = await platform.onContentSyncPhase((event) => {
      if (event.phase === "failed" && event.message) showNotice(event.message, "error");
    });
    unlistenRescan = await platform.onProjectRescanned((project) => void applyProjectRescan(project));
    if (isTauri()) {
      unlistenClose = await getCurrentWindow().onCloseRequested((event) => {
        if (allowWindowClose) return;
        if (pendingImageUploads > 0) {
          event.preventDefault();
          showNotice(`还有 ${pendingImageUploads} 张图片正在上传，请等待完成后再退出。`, "error");
          return;
        }
        const settingsDirty = settingsController?.hasDirty() ?? false;
        const editorDirty = editorStore.hasDirty();
        if (!editorDirty && !settingsDirty) {
          void platform.cleanupBeforeExit().catch(console.error);
          return;
        }
        event.preventDefault();
        requestClose();
      });
    }
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleShortcut);
    unlistenTask?.();
    unlistenPreview?.();
    unlistenSync?.();
    unlistenSyncPhase?.();
    unlistenRescan?.();
    unlistenClose?.();
    clearTimeout(configTimer);
    clearTimeout(noticeTimer);
    unsubscribeEditor();
  });

  function applyTheme(mode: AppConfigV3["appearance"]["themeMode"]) {
    document.documentElement.dataset.theme = mode;
  }

  function handleShortcut(event: KeyboardEvent) {
    const modifier = event.ctrlKey || event.metaKey;
    if (!modifier) return;
    const key = event.key.toLowerCase();
    const isAppShortcut = (event.shiftKey && key === "p")
      || (!event.shiftKey && ["s", "n", "o", ",", "\\"].includes(key))
      || /^Digit[1-4]$/.test(event.code);
    if (guardAction && isAppShortcut) {
      event.preventDefault();
      return;
    }
    if (event.shiftKey && key === "p") {
      event.preventDefault();
      event.stopPropagation();
      if (!event.repeat) void publishFromEditor();
    } else if (!event.shiftKey && key === "s") {
      event.preventDefault();
      event.stopPropagation();
      if (event.repeat) return;
      if (page === "settings") {
        void settingsController?.save().catch((error) => showNotice(normalizeError(error).message, "error"));
      } else if (page === "editor") {
        void editorStore.save().catch((error) => showNotice(normalizeError(error).message, "error"));
      }
    } else if (!event.shiftKey && key === "n" && page === "editor") {
      event.preventDefault();
      if (!event.repeat) window.dispatchEvent(new CustomEvent("hexo-editor-new-article"));
    } else if (key === "o") {
      event.preventDefault();
      if (!event.repeat) openProject();
    } else if (page === "editor" && event.code === "Backslash") {
      event.preventDefault();
      updateConfig({
        ...config,
        layout: { ...config.layout, previewVisible: !config.layout.previewVisible }
      });
    } else if (event.key === ",") {
      event.preventDefault();
      navigate("settings");
    } else if (/^Digit[1-4]$/.test(event.code)) {
      event.preventDefault();
      const pages: AppPage[] = ["editor", "imageBed", "settings", "about"];
      navigate(pages[Number(event.code.slice(-1)) - 1]);
    }
  }

  function navigate(next: AppPage, settingsSection: SettingsSectionId | null = null) {
    if (next === page) return;
    if (pendingImageUploads > 0) {
      showNotice("图片正在上传并更新地址，请等待完成后再离开写作页。", "error");
      return;
    }
    if (next === "settings") settingsInitialSection = settingsSection;
    if (page === "settings" && settingsController?.hasDirty()) {
      requestGuard("离开设置前需要保存或放弃本次设置修改。", () => { page = next; }, "settings");
      return;
    }
    if (page === "editor" && config.general.autoSave && editorStore.hasDirty()) {
      void editorStore.save().catch((error) => showNotice(normalizeError(error).message, "error"));
    }
    page = next;
  }

  function openProject() {
    if (pendingImageUploads > 0) {
      showNotice("图片正在上传并更新地址，请等待完成后再切换博客。", "error");
      return;
    }
    requestGuard("切换项目前需要处理当前文章或设置中的未保存内容。", async () => {
      try {
        const result = await platform.pickProject();
        if (result) {
          acceptProject(result.session, result.articles);
          page = "editor";
        }
      } catch (error) {
        showNotice(normalizeError(error).message);
      }
    }, "both");
  }

  async function openRecent(recentId: string) {
    if (pendingImageUploads > 0) {
      showNotice("图片正在上传并更新地址，请等待完成后再切换博客。", "error");
      return;
    }
    requestGuard("切换项目前需要处理当前文章或设置中的未保存内容。", async () => {
      try {
        const result = await platform.openRecentProject(recentId);
        acceptProject(result.session, result.articles);
        page = "editor";
        recentProjects = await platform.listRecentProjects();
      } catch (error) {
        showNotice(normalizeError(error).message);
      }
    }, "both");
  }

  async function applyProjectRescan(project: import("$shared/types/app").ProjectRescanResult) {
    if (!session || project.projectId !== session.projectId) return;
    const previousState = editorStore.getState();
    const previousArticleId = previousState.snapshot?.articleId ?? null;
    const nextSession = { ...session, generation: project.generation };
    session = nextSession;
    articles = project.articles;
    previewServer = null;

    if (previousState.snapshot?.projectId === project.projectId) {
      editorStore.rebaseSessionGeneration(project.generation);
      if (!previousState.dirty && previousArticleId && project.articles.some((item) => item.articleId === previousArticleId)) {
        try {
          const snapshot = await platform.loadDocument(project.projectId, previousArticleId, project.generation);
          editorStore.load(snapshot);
        } catch (error) {
          showNotice(normalizeError(error).message, "error");
        }
      }
    } else {
      editorStore.clear();
    }

    if (previousState.dirty) {
      showNotice("远端内容已刷新；当前未保存稿已保留，请核对后再保存。");
    } else {
      showNotice("远端内容已应用，文章列表已刷新。");
    }
  }

  function acceptProject(nextSession: ProjectSessionView, nextArticles: ArticleSummary[]) {
    session = nextSession;
    articles = nextArticles;
    const snapshot = editorStore.getState().snapshot;
    if (snapshot?.projectId !== nextSession.projectId || snapshot.sessionGeneration !== nextSession.generation) editorStore.clear();
    void platform.listRecentProjects().then((items) => (recentProjects = items));
    previewServer = null;
    if (nextSession.warnings.length) {
      showNotice(`项目诊断：${nextSession.warnings.join("；")}`);
    }
    void platform.getPreviewStatus(nextSession.projectId, nextSession.generation)
      .then((view) => {
        if (session?.projectId === view.projectId && session.generation === view.sessionGeneration) {
          previewServer = view;
          if (config.hexo.autoStartPreview && view.state === "stopped") {
            void platform.startPreviewServer(view.projectId, view.sessionGeneration).catch((error) => {
              showNotice(normalizeError(error).message);
            });
          }
        }
      })
      .catch((error) => showNotice(normalizeError(error).message));
  }

  function requestClose() {
    if (pendingImageUploads > 0) {
      showNotice(`还有 ${pendingImageUploads} 张图片正在上传，请等待完成后再退出。`, "error");
      return;
    }
    const settingsDirty = settingsController?.hasDirty() ?? false;
    const editorDirty = editorStore.hasDirty();
    closeWindowState = {
      ...closeWindowState,
      hasUnsavedChanges: settingsDirty || editorDirty
    };
    if (!closeWindowState.hasUnsavedChanges) {
      void closeWindowNow();
      return;
    }
    if (guardAction && guardIsClosing) return;
    guardDescription = settingsDirty && editorDirty
      ? "当前文章和设置都有未保存修改。"
      : settingsDirty
        ? "设置中有未保存修改。"
        : "当前文章有未保存修改。";
    guardSource = settingsDirty && editorDirty ? "both" : settingsDirty ? "settings" : "editor";
    guardIsClosing = true;
    guardAction = closeWindowNow;
  }

  async function closeWindowNow() {
    if (closeWindowState.isClosing) return;
    closeWindowState = { ...closeWindowState, isClosing: true };
    try {
      await flushPendingConfig();
      allowWindowClose = true;
      void platform.cleanupBeforeExit().catch(console.error);
      if (isTauri()) await getCurrentWindow().destroy();
    } catch (error) {
      allowWindowClose = false;
      closeWindowState = { ...closeWindowState, isClosing: false };
      throw error;
    }
  }

  function requestGuard(
    description: string,
    action: () => void | Promise<void>,
    source: "editor" | "settings" | "both" = "editor"
  ) {
    if (guardAction) return;
    const settingsDirty = settingsController?.hasDirty() ?? false;
    const editorDirty = editorStore.hasDirty();
    const hasDirty = source === "settings"
      ? settingsDirty
      : source === "both"
        ? settingsDirty || editorDirty
        : editorDirty;
    if (!hasDirty) {
      void action();
      return;
    }
    guardDescription = description;
    guardAction = action;
    guardSource = source;
    guardIsClosing = false;
  }

  async function resolveGuard(choice: "save" | "discard" | "cancel") {
    if (choice === "cancel") {
      guardAction = null;
      guardIsClosing = false;
      return;
    }
    guardBusy = true;
    const action = guardAction;
    try {
      if (guardSource === "settings" || guardSource === "both") {
        if (choice === "save") await settingsController?.save();
        else settingsController?.discard();
      }
      if (guardSource === "editor" || guardSource === "both") {
        if (choice === "save") await editorStore.save();
        else editorStore.discard();
      }
      guardAction = null;
      await action?.();
      if (!closeWindowState.isClosing) guardIsClosing = false;
    } catch (error) {
      guardAction = action;
      if (!closeWindowState.isClosing) guardIsClosing = action === closeWindowNow;
      showNotice(normalizeError(error).message);
    } finally {
      guardBusy = false;
    }
  }

  async function publishFromEditor() {
    if (!session || activeTask || publishing) return;
    if (pendingImageUploads > 0) {
      showNotice(`还有 ${pendingImageUploads} 张图片正在上传，请等待上传完成后再发布。`, "error");
      return;
    }
    publishing = true;
    try {
      if (editorStore.hasDirty()) {
        await editorStore.save();
        articles = await platform.listArticles(session.projectId, session.generation);
      }
      await platform.startTask(session.projectId, "publish");
      showNotice("发布任务已在后台开始，可继续写作。");
    } catch (error) {
      showNotice(normalizeError(error).message, "error");
    } finally {
      publishing = false;
    }
  }

  async function ensurePreviewRunning() {
    if (!session) throw new Error("请先打开博客项目。");
    let view = await platform.getPreviewStatus(session.projectId, session.generation);
    if (view.state !== "running") view = await platform.startPreviewServer(session.projectId, session.generation);
    for (let index = 0; index < 100 && view.state === "starting"; index += 1) {
      await new Promise((resolve) => setTimeout(resolve, 150));
      view = await platform.getPreviewStatus(session.projectId, session.generation);
    }
    previewServer = view;
    if (view.state !== "running") throw view.error ?? new Error("Hexo 预览尚未就绪。");
    return view;
  }

  async function previewProject(openInBrowser = true) {
    if (!session || previewBusy) return "";
    previewBusy = true;
    try {
      if (editorStore.hasDirty()) await editorStore.save();
      const articleId = editorStore.getState().snapshot?.articleId;
      if (!articleId) throw new Error("请先打开一篇文章。");
      await ensurePreviewRunning();
      const url = await platform.resolveArticlePreviewUrl(session.projectId, session.generation, articleId);
      if (openInBrowser) await platform.openMarkdownLink(url);
      return url;
    } catch (error) {
      showNotice(normalizeError(error).message, "error");
      return "";
    } finally {
      previewBusy = false;
    }
  }

  async function togglePreviewServer() {
    if (!session || previewBusy) return;
    previewBusy = true;
    try {
      previewServer = serverActive
        ? await platform.stopPreviewServer(session.projectId, session.generation)
        : await platform.startPreviewServer(session.projectId, session.generation);
    } catch (error) {
      showNotice(normalizeError(error).message, "error");
    } finally {
      previewBusy = false;
    }
  }

  async function openPreviewHome() {
    try {
      await ensurePreviewRunning();
      await platform.openExternalTarget("hexoPreview");
    } catch (error) {
      showNotice(normalizeError(error).message);
    }
  }

  function updateConfig(next: AppConfigV3) {
    config = next;
    applyTheme(next.appearance.themeMode);
    const revision = ++configRevision;
    clearTimeout(configTimer);
    configTimer = setTimeout(async () => {
      configTimer = undefined;
      const snapshot = structuredClone(next);
      try {
        const saved = await platform.saveConfig(snapshot);
        if (revision === configRevision) config = saved;
      } catch (error) {
        showNotice(normalizeError(error).message, "error");
      }
    }, 350);
  }

  async function flushPendingConfig() {
    if (!configTimer) return;
    clearTimeout(configTimer);
    configTimer = undefined;
    const revision = configRevision;
    const saved = await platform.saveConfig(structuredClone(config));
    if (revision === configRevision) config = saved;
  }

  async function saveConfigNow(next: AppConfigV3) {
    clearTimeout(configTimer);
    configTimer = undefined;
    configRevision += 1;
    const saved = await platform.saveConfig(next);
    config = saved;
    applyTheme(saved.appearance.themeMode);
    return saved;
  }

  async function removeRecentProject(recentId: string) {
    await platform.removeRecentProject(recentId);
    recentProjects = await platform.listRecentProjects();
  }

  async function clearRecentProjectList() {
    await platform.clearRecentProjects();
    recentProjects = [];
  }

  function showNotice(message: string, severity?: "info" | "error") {
    const effectiveSeverity = severity
      ?? (/失败|错误|冲突|不可用|无法|未完成/.test(message) ? "error" : "info");
    notice = message;
    noticeSeverity = effectiveSeverity;
    clearTimeout(noticeTimer);
    const duration = Math.max(5000, Math.min(12_000, message.length * 90));
    if (effectiveSeverity === "info") noticeTimer = setTimeout(() => (notice = ""), duration);
  }

  function dismissNotice() {
    clearTimeout(noticeTimer);
    notice = "";
  }

  function appendTaskEvent(events: TaskEvent[], event: TaskEvent) {
    const next = [...events, event];
    if (next.length <= 2500) return next;
    const firstLog = next.findIndex((item) => item.kind === "log");
    if (firstLog >= 0) next.splice(firstLog, 1);
    else next.shift();
    return next;
  }

  function findActiveTask(events: TaskEvent[]) {
    const finished = new Set(
      events.filter((event) => event.kind === "finished").map((event) => event.taskId)
    );
    return [...events].reverse().find((event) => !finished.has(event.taskId));
  }
</script>

<div class:is-maximized={maximized} class="app-window">
  <TitleBar
    documentTitle={activeDocumentTitle}
    {dirty}
    onRequestClose={requestClose}
    onMaximizedChange={(value) => (maximized = value)}
  />
  <div class="app-body">
    <NavRail {page} onNavigate={navigate} />
    <main class="workspace">
      {#if !configLoaded}
        <LoadingState label="正在初始化桌面工作区" />
      {:else}
        {#key page}
          <PageTransition pageKey={page}>
            {#await pagePromise}
              <LoadingState label="正在加载页面" />
            {:then module}
              {@const Page = module.default}
              <Page
            {session}
            {articles}
            {config}
            {taskEvents}
            {editorStore}
            {recentProjects}
            {previewServer}
            initialSection={settingsInitialSection}
            autoSaveSuspended={guardIsClosing}
            taskBusy={Boolean(activeTask) || publishing}
            {previewBusy}
            onOpenProject={openProject}
            onOpenRecentProject={openRecent}
            onArticlesChange={(next: ArticleSummary[]) => (articles = next)}
            onConfigChange={updateConfig}
            onSaveConfig={saveConfigNow}
            onThemePreview={(mode: AppConfigV3["appearance"]["themeMode"]) => applyTheme(mode)}
            onRegisterSettingsController={(controller: SettingsController | null) => (settingsController = controller)}
            onRemoveRecentProject={removeRecentProject}
            onClearRecentProjects={clearRecentProjectList}
            onPublish={publishFromEditor}
            onPreview={previewProject}
            onTogglePreviewServer={togglePreviewServer}
            onOpenPreviewHome={openPreviewHome}
            onNotice={showNotice}
            onPendingImageUploadsChange={(count: number) => (pendingImageUploads = count)}
            onOpenSettings={(section?: SettingsSectionId) => navigate("settings", section ?? "maintenance")}
              />
            {/await}
          </PageTransition>
        {/key}
      {/if}
      <div class="status-toasts" aria-live="polite">
        {#if notice}
          <div
            class:error={noticeSeverity === "error"}
            class="task-indicator notice-indicator"
            role={noticeSeverity === "error" ? "alert" : "status"}
            in:fly={{ y: 8, duration: 160 }}
            out:fade={{ duration: 120 }}
          >
            <span>{notice}</span>
            <button class="notice-close" type="button" aria-label="关闭通知" on:click={dismissNotice}><X size={14} /></button>
          </div>
        {/if}
        {#if activeTask}
          <div class="task-indicator" role="status" in:fly={{ y: 8, duration: 160 }} out:fade={{ duration: 120 }}>
            <LoaderCircle size={15} class="spin" />
            <span>{activeTask.step ?? "正在处理项目"}</span>
          </div>
        {/if}
      </div>
    </main>
  </div>
</div>

{#if guardAction}
  <ModalDialog
    title={guardIsClosing ? "退出 Hexo Lite Editor？" : "有未保存的内容"}
    description={guardDescription}
    onClose={() => !guardBusy && resolveGuard("cancel")}
  >
    <svelte:fragment slot="actions">
      <button class="button" type="button" disabled={guardBusy || closeWindowState.isClosing} on:click={() => resolveGuard("cancel")}>取消</button>
      <button class="button danger" type="button" disabled={guardBusy || closeWindowState.isClosing} on:click={() => resolveGuard("discard")}>
        {guardIsClosing ? "不保存退出" : "放弃"}
      </button>
      <button class="button primary" type="button" data-autofocus disabled={guardBusy || closeWindowState.isClosing} on:click={() => resolveGuard("save")}>
        {guardBusy ? "处理中" : guardIsClosing ? "保存并退出" : "保存并继续"}
      </button>
    </svelte:fragment>
  </ModalDialog>
{/if}

<style>
  :global(.spin) {
    animation: rotate 1s linear infinite;
  }
  @keyframes rotate {
    to { transform: rotate(360deg); }
  }
</style>
