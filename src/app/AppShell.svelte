<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { LoaderCircle } from "@lucide/svelte";
  import TitleBar from "./TitleBar.svelte";
  import NavRail from "./NavRail.svelte";
  import LoadingState from "$shared/components/LoadingState.svelte";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import { isTauri, normalizeError, platform } from "$platform/tauri";
  import { defaultConfig } from "$shared/types/app";
  import type {
    AppConfigV3,
    AppPage,
    ArticleSummary,
    ProjectSessionView,
    PreviewServerView,
    RecentProjectView,
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
  let maximized = false;
  let taskEvents: TaskEvent[] = [];
  let unlistenTask: (() => void) | undefined;
  let unlistenPreview: (() => void) | undefined;
  let unlistenClose: (() => void) | undefined;
  let configTimer: ReturnType<typeof setTimeout> | undefined;
  let notice = "";
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;
  let guardAction: (() => void | Promise<void>) | null = null;
  let guardDescription = "";
  let guardBusy = false;
  let guardSource: "editor" | "settings" = "editor";
  let previewServer: PreviewServerView | null = null;

  const unsubscribeEditor = editorStore.subscribe((state) => {
    dirty = state.dirty;
  });

  $: pagePromise = pageLoaders[page]();
  $: activeTask = findActiveTask(taskEvents);
  $: serverActive = previewServer?.state === "running";

  onMount(async () => {
    window.addEventListener("keydown", handleShortcut);
    try {
      const loaded = await platform.loadConfig();
      config = loaded.config;
      applyTheme(config.appearance.themeMode);
      configLoaded = true;
      if (loaded.warnings.length) showNotice(loaded.warnings[0]);
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
        showNotice("任务执行失败，可在设置 → 诊断与日志中查看详情。");
      }
    });
    unlistenPreview = await platform.onPreviewStatus((view) => {
      if (view.projectId === session?.projectId && view.sessionGeneration === session.generation) {
        previewServer = view;
        if (view.state === "error" && view.error) showNotice(view.error.message);
      }
    });
    if (isTauri()) {
      unlistenClose = await getCurrentWindow().onCloseRequested((event) => {
        const settingsDirty = settingsController?.hasDirty() ?? false;
        if (!dirty && !settingsDirty) return;
        event.preventDefault();
        requestGuard(
          settingsDirty ? "关闭窗口前需要处理未保存的设置。" : "关闭窗口前需要处理当前文章的未保存内容。",
          async () => { await getCurrentWindow().close(); },
          settingsDirty ? "settings" : "editor"
        );
      });
    }
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleShortcut);
    unlistenTask?.();
    unlistenPreview?.();
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
    if (event.shiftKey && event.key.toLowerCase() === "p") {
      event.preventDefault();
      event.stopPropagation();
      if (!event.repeat) void publishFromEditor();
    } else if (event.key.toLowerCase() === "o") {
      event.preventDefault();
      openProject();
    } else if (event.key === ",") {
      event.preventDefault();
      navigate("settings");
    } else if (/^[1-4]$/.test(event.key)) {
      event.preventDefault();
      const pages: AppPage[] = ["editor", "imageBed", "settings", "about"];
      navigate(pages[Number(event.key) - 1]);
    }
  }

  function navigate(next: AppPage) {
    if (next === page) return;
    if (page === "settings" && settingsController?.hasDirty()) {
      requestGuard("离开设置前需要保存或放弃本次设置修改。", () => { page = next; }, "settings");
      return;
    }
    page = next;
  }

  function openProject() {
    requestGuard("切换项目前需要处理当前文章的未保存内容。", async () => {
      try {
        const result = await platform.pickProject();
        if (result) {
          acceptProject(result.session, result.articles);
          page = "editor";
        }
      } catch (error) {
        showNotice(normalizeError(error).message);
      }
    }, "editor");
  }

  async function openRecent(recentId: string) {
    requestGuard("切换项目前需要处理当前文章的未保存内容。", async () => {
      try {
        const result = await platform.openRecentProject(recentId);
        acceptProject(result.session, result.articles);
        page = "editor";
        recentProjects = await platform.listRecentProjects();
      } catch (error) {
        showNotice(normalizeError(error).message);
      }
    }, "editor");
  }

  function acceptProject(nextSession: ProjectSessionView, nextArticles: ArticleSummary[]) {
    session = nextSession;
    articles = nextArticles;
    if (editorStore.getState().snapshot?.projectId !== nextSession.projectId) editorStore.clear();
    void platform.listRecentProjects().then((items) => (recentProjects = items));
    previewServer = null;
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
    const settingsDirty = settingsController?.hasDirty() ?? false;
    requestGuard(
      settingsDirty ? "关闭窗口前需要处理未保存的设置。" : "关闭窗口前需要处理当前文章的未保存内容。",
      async () => { if (isTauri()) await getCurrentWindow().close(); },
      settingsDirty ? "settings" : "editor"
    );
  }

  function requestGuard(
    description: string,
    action: () => void | Promise<void>,
    source: "editor" | "settings" = "editor"
  ) {
    const hasDirty = source === "settings" ? settingsController?.hasDirty() : editorStore.hasDirty();
    if (!hasDirty) {
      void action();
      return;
    }
    guardDescription = description;
    guardAction = action;
    guardSource = source;
  }

  async function resolveGuard(choice: "save" | "discard" | "cancel") {
    if (choice === "cancel") {
      guardAction = null;
      return;
    }
    guardBusy = true;
    try {
      if (guardSource === "settings") {
        if (choice === "save") await settingsController?.save();
        else settingsController?.discard();
      } else if (choice === "save") await editorStore.save();
      else editorStore.discard();
      const action = guardAction;
      guardAction = null;
      await action?.();
    } catch (error) {
      showNotice(normalizeError(error).message);
    } finally {
      guardBusy = false;
    }
  }

  async function publishFromEditor() {
    if (!session || activeTask) return;
    try {
      if (config.publish.saveBeforeRun && editorStore.hasDirty()) {
        await editorStore.save();
        articles = await platform.listArticles(session.projectId, session.generation);
      }
      await platform.startTask(session.projectId, "publish");
      showNotice("发布任务已在后台开始，可继续写作。");
    } catch (error) {
      showNotice(normalizeError(error).message);
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
    if (!session) return "";
    try {
      if (editorStore.hasDirty()) await editorStore.save();
      const articleId = editorStore.getState().snapshot?.articleId;
      if (!articleId) throw new Error("请先打开一篇文章。");
      await ensurePreviewRunning();
      const url = await platform.resolveArticlePreviewUrl(session.projectId, session.generation, articleId);
      if (openInBrowser) await platform.openMarkdownLink(url);
      return url;
    } catch (error) {
      showNotice(normalizeError(error).message);
      return "";
    }
  }

  async function togglePreviewServer() {
    if (!session) return;
    try {
      previewServer = serverActive
        ? await platform.stopPreviewServer(session.projectId, session.generation)
        : await platform.startPreviewServer(session.projectId, session.generation);
    } catch (error) {
      showNotice(normalizeError(error).message);
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
    clearTimeout(configTimer);
    configTimer = setTimeout(async () => {
      try {
        config = await platform.saveConfig(config);
      } catch (error) {
        showNotice(normalizeError(error).message);
      }
    }, 350);
  }

  async function saveConfigNow(next: AppConfigV3) {
    clearTimeout(configTimer);
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

  function showNotice(message: string) {
    notice = message;
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => (notice = ""), 5000);
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
  <TitleBar onRequestClose={requestClose} onMaximizedChange={(value) => (maximized = value)} />
  <div class="app-body">
    <NavRail {page} onNavigate={navigate} />
    <main class="workspace">
      {#if !configLoaded}
        <LoadingState label="正在初始化桌面工作区" />
      {:else}
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
            taskBusy={Boolean(activeTask)}
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
            onOpenSettings={() => navigate("settings")}
          />
        {/await}
      {/if}
      {#if activeTask}
        <div class="task-indicator" role="status" aria-live="polite">
          <LoaderCircle size={15} class="spin" />
          <span>{activeTask.step ?? "项目任务"}{activeTask.line ? ` · ${activeTask.line}` : ""}</span>
        </div>
      {:else if notice}
        <div class="task-indicator" role="status"><span>{notice}</span></div>
      {/if}
    </main>
  </div>
</div>

{#if guardAction}
  <ModalDialog title="有未保存的内容" description={guardDescription} onClose={() => resolveGuard("cancel")}>
    <svelte:fragment slot="actions">
      <button class="button" type="button" disabled={guardBusy} on:click={() => resolveGuard("cancel")}>取消</button>
      <button class="button danger" type="button" disabled={guardBusy} on:click={() => resolveGuard("discard")}>放弃</button>
      <button class="button primary" type="button" data-autofocus disabled={guardBusy} on:click={() => resolveGuard("save")}>{guardBusy ? "处理中" : "保存并继续"}</button>
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
