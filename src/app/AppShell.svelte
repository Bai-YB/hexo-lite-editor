<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { onDestroy, onMount } from "svelte";
  import { fade, fly } from "svelte/transition";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { LoaderCircle, X } from "@lucide/svelte";
  import TitleBar from "./TitleBar.svelte";
  import NavRail from "./NavRail.svelte";
  import LoadingState from "$shared/components/LoadingState.svelte";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import { hasOpenModal } from "$shared/components/modalStack";
  import { reconcileProjectRescan } from "./projectRescan";
  import { checkRecoveryRemote, matchesRecovery, persistRecoveryDraft, waitForRecovery, type RecoveryIdentity } from "./documentRecovery";
  import { disposePluginWorkers } from "$shared/plugins/PluginProviderRuntime";
  import PageTransition from "$shared/components/PageTransition.svelte";
  import { isTauri, normalizeError, platform } from "$platform/tauri";
  import { defaultConfig } from "$shared/types/app";
  import { shouldAutoDownload, updateStore } from "$features/update/updateStore";
  import { updateViewModel } from "$features/update/updateViewModel";
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
  import { FileSessionStore } from "$features/files/FileSessionStore";
  import { initI18n, t } from "$shared/i18n";

  const pageLoaders: Record<AppPage, () => Promise<{ default: any }>> = {
    editor: () => import("$features/editor/EditorPage.svelte"),
    imageBed: () => import("$features/image-bed/ImageBedPage.svelte"),
    plugins: () => import("$features/plugins/PluginsPage.svelte"),
    files: () => import("$features/files/FilesPage.svelte"),
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
  const fileStore = new FileSessionStore(platform.saveProjectFile);
  let filesDirty = false;
  let activeFilePath = "";
  let fileArticleOpenSequence = 0;
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
  let unlistenUpdate: (() => void) | undefined;
  let updateCheckTimer: ReturnType<typeof setTimeout> | undefined;
  let configTimer: ReturnType<typeof setTimeout> | undefined;
  let pageWarmupCanceled = false;
  let pageWarmupIdle: number | undefined;
  let notice = "";
  let noticeSeverity: "info" | "error" = "info";
  let noticeTimer: ReturnType<typeof setTimeout> | undefined;
  let guardAction: (() => void | Promise<void>) | null = null;
  let guardCompletion: ((accepted: boolean) => void) | null = null;
  let guardDescription = "";
  let guardBusy = false;
  type GuardSource = "editor" | "files" | "documents" | "both";
  let guardSource: GuardSource = "editor";
  let guardIsClosing = false;
  let allowWindowClose = false;
  let closeWindowState: CloseWindowState = { hasUnsavedChanges: false, isClosing: false };
  let previewServer: PreviewServerView | null = null;
  let previewBusy = false;
  let publishing = false;
  let publishTaskId = "";
  let pendingImageUploads = 0;
  let settingsInitialSection: SettingsSectionId | null = null;
  let dismissedUpdateVersion = "";
  let configRevision = 0;
  let configSaveQueue: Promise<unknown> = Promise.resolve();
  let externalChange: "changed" | "deleted" | null = null;
  let recoveryOpen = false;
  let recoveryBusy = false;
  let recoveryRemote: string | null = null;
  let recoveryLocal = "";
  let recoveryIdentity: RecoveryIdentity | null = null;
  let recoveryDraft: { projectId: string; generation: number; token: number; article: ArticleSummary } | null = null;
  let taskDetailsOpen = false;
  let cancelingTask = false;

  const unsubscribeEditor = editorStore.subscribe((state) => {
    dirty = state.dirty;
    activeArticleId = state.snapshot?.articleId ?? null;
    externalChange = state.externalChange ?? null;
    if (!externalChange && !recoveryBusy) recoveryOpen = false;
  });
  const unsubscribeFiles = fileStore.subscribe((state) => {
    filesDirty = state.dirty || state.saving;
    activeFilePath = state.snapshot?.path ?? "";
  });

  $: pagePromise = pageLoaders[page]();
  $: activeTask = findActiveTask(taskEvents);
  $: serverActive = previewServer?.state === "running";
  $: activeDocumentTitle = articles.find((article) => article.articleId === activeArticleId)?.title ?? "";
  $: globalUpdateModel = updateViewModel($updateStore);

  onMount(async () => {
    window.addEventListener("keydown", handleShortcut);
    try {
      const loaded = await platform.loadConfig();
      config = loaded.config;
      initI18n(config.general.language);
      applyTheme(config.appearance.themeMode);
      configLoaded = true;
      if (loaded.warnings.length) showNotice(loaded.warnings[0]);
    } catch (error) {
      configLoaded = true;
      showNotice(normalizeError(error).message, "error");
    }
    unlistenTask = await platform.onTaskEvent((event) => {
      taskEvents = appendTaskEvent(taskEvents, event);
      if (event.kind === "finished") {
        const isPublish = event.taskId === publishTaskId;
        if (event.success === false) {
          const reason = latestTaskFailure(taskEvents, event.taskId);
          showNotice(
            isPublish
              ? $ui("发布失败：{p0}", { p0: reason || $ui("具体看任务详情。") })
              : reason || $ui("任务失败了，检查一下项目设置和网络，再试一次。"),
            "error"
          );
        } else if (isPublish) {
          showNotice($ui("发布完成，站点已上传。"));
        }
        if (isPublish) {
          publishTaskId = "";
          publishing = false;
        }
      }
    });
    unlistenPreview = await platform.onPreviewStatus((view) => {
      if (view.projectId === session?.projectId && view.sessionGeneration === session.generation) {
        previewServer = view;
        if (view.state === "error" && view.error) showNotice(view.error.message, "error");
      }
    });
    unlistenSync = await platform.onContentSyncStatus((view) => {
      if (view.projectId !== session?.projectId || view.sessionGeneration !== session?.generation) return;
      if (["offline", "authRequired", "remoteAhead", "conflict", "error"].includes(view.status)) {
        showNotice(view.message || $ui("云端同步：{p0}", { p0: view.status }), "error");
      }
    });
    unlistenSyncPhase = await platform.onContentSyncPhase((event) => {
      if (event.projectId !== session?.projectId || event.sessionGeneration !== session?.generation) return;
      if (event.phase === "failed" && event.message) showNotice(event.message, "error");
    });
    unlistenRescan = await platform.onProjectRescanned((project) => void applyProjectRescan(project));
    if (isTauri()) {
      unlistenClose = await getCurrentWindow().onCloseRequested((event) => {
        if (allowWindowClose) return;
        if (recoveryBusy) { event.preventDefault(); return; }
        if (pendingImageUploads > 0) {
          event.preventDefault();
          showNotice($ui("{p0} 张图片还在上传，等完成后再退出。", { p0: pendingImageUploads }), "error");
          return;
        }
        const editorDirty = editorStore.hasDirty() || editorStore.getState().saving;
        if (!editorDirty && !filesDirty && !activeTask && !publishing) {
          void platform.cleanupBeforeExit().catch(console.error);
          return;
        }
        event.preventDefault();
        requestClose();
      });
    }
    // The shell is interactive before update IPC or project scanning starts.
    // Native project opening also runs on a blocking worker.
    requestAnimationFrame(() => setTimeout(() => {
      void initializeUpdateState();
      void initializeRecentWorkspace();
      schedulePageWarmup();
    }, 0));
  });

  async function initializeUpdateState() {
    try {
      let receivedUpdateEvent = false;
      unlistenUpdate = await platform.onUpdateSnapshot((snapshot) => {
        receivedUpdateEvent = true;
        updateStore.set(snapshot);
      });
      const initialUpdate = await platform.getUpdateSnapshot();
      if (!receivedUpdateEvent) updateStore.set(initialUpdate);
    } catch (error) {
      console.info("更新状态暂不可用", normalizeError(error).message);
    }
    if (!config.update.checkOnStart) return;
    const lastCheckKey = "hexo-lite-editor:update-last-auto-check";
    const lastCheck = Number(localStorage.getItem(lastCheckKey) ?? "0");
    if (Number.isFinite(lastCheck) && Date.now() - lastCheck < 86_400_000) return;
    updateCheckTimer = setTimeout(() => {
      void (async () => {
        try {
          const update = await platform.checkUpdate();
          updateStore.set(update);
          localStorage.setItem(lastCheckKey, String(Date.now()));
          if (shouldAutoDownload(update, config.update.autoDownload)) {
            updateStore.set(await platform.downloadUpdate());
          }
        } catch (error) {
          console.info("后台更新未完成", normalizeError(error).message);
        }
      })();
    }, 3000);
  }

  async function initializeRecentWorkspace() {
    try {
      const recentProjectsTask = platform.listRecentProjects();
      const reopened = config.general.openRecentProjectOnStart
        ? await platform.reopenRecentProject()
        : null;
      recentProjects = await recentProjectsTask;
      if (reopened && !session) acceptProject(reopened.session, reopened.articles);
    } catch (error) {
      showNotice(normalizeError(error).message, "error");
    }
  }

  function schedulePageWarmup() {
    const warm = async () => {
      for (const [name, load] of Object.entries(pageLoaders)) {
        if (pageWarmupCanceled || name === "editor") continue;
        await load();
        await new Promise((resolve) => setTimeout(resolve, 40));
      }
    };
    const idleWindow = window as unknown as {
      requestIdleCallback?: (callback: () => void, options?: { timeout: number }) => number;
    };
    if (typeof idleWindow.requestIdleCallback === "function") {
      pageWarmupIdle = idleWindow.requestIdleCallback(() => void warm(), { timeout: 1200 });
    } else {
      pageWarmupIdle = setTimeout(() => void warm(), 500) as unknown as number;
    }
  }

  onDestroy(() => {
    pageWarmupCanceled = true;
    if (pageWarmupIdle !== undefined) {
      const idleWindow = window as unknown as { cancelIdleCallback?: (handle: number) => void };
      if (typeof idleWindow.cancelIdleCallback === "function") idleWindow.cancelIdleCallback(pageWarmupIdle);
      else clearTimeout(pageWarmupIdle);
    }
    window.removeEventListener("keydown", handleShortcut);
    unlistenTask?.();
    unlistenPreview?.();
    unlistenSync?.();
    unlistenSyncPhase?.();
    unlistenRescan?.();
    unlistenClose?.();
    unlistenUpdate?.();
    clearTimeout(updateCheckTimer);
    clearTimeout(configTimer);
    clearTimeout(noticeTimer);
    unsubscribeEditor();
    unsubscribeFiles();
    releaseThemeListener();
    disposePluginWorkers();
  });

  let themeSystemMedia: MediaQueryList | undefined;
  let themeSystemHandler: (() => void) | undefined;

  function releaseThemeListener() {
    if (themeSystemMedia && themeSystemHandler) themeSystemMedia.removeEventListener("change", themeSystemHandler);
    themeSystemMedia = undefined;
    themeSystemHandler = undefined;
  }

  function applyTheme(mode: AppConfigV3["appearance"]["themeMode"]) {
    releaseThemeListener();
    let resolved = mode;
    if (mode === "system" && typeof window.matchMedia === "function") {
      themeSystemMedia = window.matchMedia("(prefers-color-scheme: dark)");
      themeSystemHandler = () => {
        document.documentElement.dataset.theme = themeSystemMedia?.matches ? "dark" : "light";
      };
      themeSystemMedia.addEventListener("change", themeSystemHandler);
      resolved = themeSystemMedia.matches ? "dark" : "light";
    }
    document.documentElement.dataset.theme = resolved;
  }

  function handleShortcut(event: KeyboardEvent) {
    if (event.defaultPrevented || event.isComposing || event.keyCode === 229) return;
    const modifier = event.ctrlKey || event.metaKey;
    if (!modifier) return;
    const key = event.key.toLowerCase();
    const isAppShortcut = (event.shiftKey && key === "p")
      || (!event.shiftKey && ["s", "n", "o", ",", "\\"].includes(key))
      || /^Digit[1-6]$/.test(event.code);
    if ((guardAction || recoveryBusy || hasOpenModal()) && isAppShortcut) {
      event.preventDefault();
      event.stopPropagation();
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
      if (page === "editor") {
        void editorStore.save().catch((error) => showNotice(normalizeError(error).message, "error"));
      } else if (page === "files") {
        void fileStore.save().catch((error) => showNotice(normalizeError(error).message, "error"));
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
    } else if (/^Digit[1-6]$/.test(event.code)) {
      event.preventDefault();
      const pages: AppPage[] = ["editor", "imageBed", "settings", "about", "plugins", "files"];
      navigate(pages[Number(event.code.slice(-1)) - 1]);
    }
  }

  function navigate(next: AppPage, settingsSection: SettingsSectionId | null = null) {
    if (recoveryBusy) return;
    if (next === page) return;
    if (pendingImageUploads > 0) {
      showNotice($ui("图片还在上传，等完成后再离开写作页。"), "error");
      return;
    }
    if (next === "settings") settingsInitialSection = settingsSection;
    if (page === "files" && fileStore.hasDirty()) {
      requestGuard($ui("先保存或放弃当前文件的修改，再离开。"), () => { page = next; }, "files");
      return;
    }
    if (page === "editor" && config.general.autoSave && editorStore.hasDirty()) {
      void editorStore.save().catch((error) => showNotice(normalizeError(error).message, "error"));
    }
    page = next;
  }

  function openArticleFromFiles(articleId: string) {
    if (!session || recoveryBusy) return;
    const project = { ...session };
    const sequence = ++fileArticleOpenSequence;
    requestGuard($ui("先保存或放弃当前修改，再打开博文。"), async () => {
      try {
        const token = editorStore.documentToken();
        const snapshot = await platform.loadDocument(project.projectId, articleId, project.generation);
        if (sequence !== fileArticleOpenSequence || page !== "files" || session?.projectId !== project.projectId || session.generation !== project.generation || editorStore.documentToken() !== token) return;
        editorStore.load(snapshot);
        page = "editor";
      } catch (error) { showNotice(normalizeError(error).message, "error"); }
    }, "documents");
  }

  function openProject() {
    if (recoveryBusy) return;
    if (activeTask || publishing) {
      taskDetailsOpen = true;
      showNotice($ui("后台任务还在跑，等它结束或先取消。"), "error");
      return;
    }
    if (pendingImageUploads > 0) {
      showNotice($ui("图片还在上传，等完成后再切换博客。"), "error");
      return;
    }
    requestGuard($ui("文章或文件里有没保存的内容。"), async () => {
      try {
        const result = await platform.pickProject();
        if (result) {
          acceptProject(result.session, result.articles);
          page = "editor";
        }
      } catch (error) {
        showNotice(normalizeError(error).message, "error");
      }
    }, "both");
  }

  async function openRecent(recentId: string) {
    if (recoveryBusy) return;
    if (activeTask || publishing) {
      taskDetailsOpen = true;
      showNotice($ui("后台任务还在跑，等它结束或先取消。"), "error");
      return;
    }
    if (pendingImageUploads > 0) {
      showNotice($ui("图片还在上传，等完成后再切换博客。"), "error");
      return;
    }
    requestGuard($ui("文章或文件里有没保存的内容。"), async () => {
      try {
        const result = await platform.openRecentProject(recentId);
        acceptProject(result.session, result.articles);
        page = "editor";
        recentProjects = await platform.listRecentProjects();
      } catch (error) {
        showNotice(normalizeError(error).message, "error");
      }
    }, "both");
  }

  async function applyProjectRescan(project: import("$shared/types/app").ProjectRescanResult) {
    let fileRefresh = Promise.resolve();
    try {
      const articleRefresh = reconcileProjectRescan(project, {
        store: editorStore,
        getSession: () => session,
        accept: (next) => {
          session = { ...session!, generation: next.generation };
          articles = next.articles;
          previewServer = null;
          fileRefresh = fileStore.refresh({ ...session! }, (current, path) => platform.loadProjectFile(current.projectId, current.generation, path));
        },
        loadDocument: platform.loadDocument
      });
      const [articleResult, fileResult] = await Promise.allSettled([articleRefresh, fileRefresh]);
      if (fileResult.status === "rejected") showNotice(normalizeError(fileResult.reason).message, "error");
      if (articleResult.status === "rejected") throw articleResult.reason;
      const result = articleResult.value;
      if (result === "changed" || result === "deleted") showNotice($ui("云端有更新，本地内容已保留，先处理差异再保存。"), "error");
      else if (result === "refreshed") showNotice($ui("云端内容已应用，文章列表已刷新。"));
    } catch (error) { showNotice(normalizeError(error).message, "error"); }
  }

  async function reviewExternalChange() {
    if (!session || !externalChange || recoveryBusy) return;
    const state = editorStore.getState();
    const token = editorStore.documentToken();
    const project = { ...session };
    recoveryLocal = state.content;
    recoveryIdentity = { projectId: project.projectId, generation: project.generation, token, revision: state.revision, externalChange };
    const identity = recoveryIdentity;
    recoveryRemote = null;
    recoveryOpen = true;
    if (externalChange === "deleted" || !state.snapshot) return;
    recoveryBusy = true;
    try {
      const remote = await checkRecoveryRemote(identity, recoveryContext(), null);
      if (remote.status !== "stale") recoveryRemote = remote.snapshot.content;
    } catch (error) { showNotice(normalizeError(error).message, "error"); }
    finally { recoveryBusy = false; }
  }

  function recoveryContext() {
    return { store: editorStore, getSession: () => session, loadDocument: platform.loadDocument };
  }

  async function resolveExternalChange(choice: "local" | "remote" | "draft" | "close") {
    if (!session || recoveryBusy || !externalChange) return;
    recoveryBusy = true;
    const project = { ...session };
    const token = editorStore.documentToken();
    const state = editorStore.getState();
    const identity = recoveryIdentity;
    const context = recoveryContext();
    if (!identity || !matchesRecovery(identity, context)) {
      recoveryBusy = false;
      showNotice($ui("文章又变了，看清内容后再选怎么处理。"), "error");
      await reviewExternalChange();
      return;
    }
    const current = () => editorStore.matchesDocument(token) && session?.projectId === project.projectId && session.generation === project.generation;
    try {
      if (!await waitForRecovery(identity, context) || !state.snapshot) return;
      if (externalChange === "changed" && (choice === "local" || choice === "remote")) {
        const result = await checkRecoveryRemote(identity, context, recoveryRemote);
        if (result.status === "stale") return;
        if (result.status === "review") {
          recoveryRemote = result.snapshot.content;
          showNotice($ui("云端内容又更新了，重新对比后再选一次。"), "error");
          return;
        }
        if (choice === "remote") { editorStore.load(result.snapshot); recoveryOpen = false; return; }
      }
      if (choice === "close") editorStore.clear();
      else if (choice === "local") {
        editorStore.allowExternalOverwrite();
        try { await editorStore.saveUntilClean(); }
        catch (error) { if (current()) editorStore.markExternalChange("changed"); throw error; }
      } else if (choice === "remote") {
        return;
      } else {
        const article = recoveryDraft?.projectId === project.projectId && recoveryDraft.generation === project.generation && recoveryDraft.token === token
          ? recoveryDraft.article : null;
        const saved = await persistRecoveryDraft({
          identity, context, article, content: state.content,
          request: {
            projectId: project.projectId, sessionGeneration: project.generation,
            title: activeDocumentTitle || $ui("恢复的文章"), fileName: `recovered-${Date.now()}`, kind: "draft",
            date: new Date().toLocaleString("sv-SE"), tags: [], categories: []
          },
          createArticle: platform.createArticle, saveDocument: platform.saveDocument,
          remember: (created) => {
            recoveryDraft = { projectId: project.projectId, generation: project.generation, token, article: created };
            if (current() && !articles.some((item) => item.articleId === created.articleId)) articles = [created, ...articles];
          }
        });
        editorStore.load(saved);
        recoveryDraft = null;
        showNotice($ui("本地内容已存为草稿，注意检查文章里的图片路径。"));
      }
      if (!editorStore.getState().externalChange) recoveryOpen = false;
    } catch (error) {
      showNotice(`${normalizeError(error).message}${choice === "draft" && recoveryDraft ? " " + $ui("再点一次「存为草稿」会继续写进同一个文件。") : ""}`, "error");
    } finally { recoveryBusy = false; if (!editorStore.getState().externalChange) recoveryOpen = false; }
  }

  function beforeSync(): Promise<boolean> {
    if (activeTask || publishing) {
      showNotice($ui("等 Hexo 或发布任务结束后再同步。"), "error");
      return Promise.resolve(false);
    }
    if (guardAction || recoveryBusy || pendingImageUploads > 0 || externalChange) {
      showNotice($ui("先等图片处理完、冲突解决，再同步。"), "error");
      return Promise.resolve(false);
    }
    return new Promise((resolve) => {
      guardCompletion = resolve;
      requestGuard($ui("同步前，先保存或放弃文章和文件的修改。"), () => {
        const busy = Boolean(activeTask) || publishing || pendingImageUploads > 0 || recoveryBusy || Boolean(externalChange);
        if (busy) showNotice($ui("任务还没结束，稍后再同步。"), "error");
        resolve(!busy);
        guardCompletion = null;
      }, "documents");
    });
  }

  function acceptProject(nextSession: ProjectSessionView, nextArticles: ArticleSummary[]) {
    session = nextSession;
    articles = nextArticles;
    const snapshot = editorStore.getState().snapshot;
    if (snapshot?.projectId !== nextSession.projectId || snapshot.sessionGeneration !== nextSession.generation) editorStore.clear();
    const file = fileStore.getState().snapshot;
    if (file?.projectId !== nextSession.projectId || file.sessionGeneration !== nextSession.generation) fileStore.clear();
    void platform.listRecentProjects().then((items) => (recentProjects = items));
    previewServer = null;
    if (nextSession.warnings.length) {
      showNotice($ui("项目诊断：{p0}", { p0: nextSession.warnings.join("；") }));
    }
    void platform.getPreviewStatus(nextSession.projectId, nextSession.generation)
      .then((view) => {
        if (session?.projectId === view.projectId && session.generation === view.sessionGeneration) {
          previewServer = view;
          if (config.hexo.autoStartPreview && view.state === "stopped") {
            void platform.startPreviewServer(view.projectId, view.sessionGeneration).catch((error) => {
              showNotice(normalizeError(error).message, "error");
            });
          }
        }
      })
      .catch((error) => showNotice(normalizeError(error).message, "error"));
  }

  function requestClose() {
    if (recoveryBusy) return;
    if (guardAction) return;
    if (activeTask || publishing) {
      taskDetailsOpen = true;
      showNotice($ui("后台任务还在跑，等结束或在任务详情里取消，再退出。"), "error");
      return;
    }
    if (pendingImageUploads > 0) {
      showNotice($ui("{p0} 张图片还在上传，等完成后再退出。", { p0: pendingImageUploads }), "error");
      return;
    }
    const editorDirty = editorStore.hasDirty() || editorStore.getState().saving;
    closeWindowState = {
      ...closeWindowState,
      hasUnsavedChanges: editorDirty || filesDirty
    };
    if (!closeWindowState.hasUnsavedChanges) {
      void closeWindowNow();
      return;
    }
    if (guardAction && guardIsClosing) return;
    guardDescription = $ui("文章或文件里有没保存的内容。");
    guardSource = "both";
    guardIsClosing = true;
    guardAction = closeWindowNow;
  }

  async function closeWindowNow() {
    if (closeWindowState.isClosing) return;
    closeWindowState = { ...closeWindowState, isClosing: true };
    try {
      await flushPendingConfig();
      disposePluginWorkers();
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
    source: GuardSource = "editor"
  ) {
    if (guardAction) return;
    const editorDirty = editorStore.hasDirty() || editorStore.getState().saving;
    const hasDirty = source === "both"
      ? editorDirty || filesDirty
      : source === "files" ? filesDirty
        : source === "documents" ? editorDirty || filesDirty : editorDirty;
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
    if (guardBusy) return;
    if (choice === "cancel") {
      guardCompletion?.(false);
      guardCompletion = null;
      guardAction = null;
      guardIsClosing = false;
      return;
    }
    guardBusy = true;
    const action = guardAction;
    try {
      if (["editor", "documents", "both"].includes(guardSource) && externalChange) {
        guardCompletion?.(false);
        guardCompletion = null;
        guardAction = null;
        guardIsClosing = false;
        await reviewExternalChange();
        return;
      }
      if (["editor", "documents", "both"].includes(guardSource)) {
        if (choice === "save") await editorStore.saveUntilClean();
        else {
          await editorStore.waitForSave().catch(() => null);
          if (editorStore.getState().externalChange) throw new Error($ui("云端有更新，先处理差异再继续。"));
          editorStore.discard();
        }
      }
      if (["editor", "documents", "both"].includes(guardSource) && (editorStore.getState().externalChange || editorStore.hasDirty())) throw new Error($ui("文章还有没处理完的修改。"));
      if (["files", "documents", "both"].includes(guardSource)) {
        if (choice === "save") await fileStore.save();
        else await fileStore.discard();
        if (fileStore.hasDirty()) throw new Error($ui("项目文件还没保存，先处理再继续。"));
      }
      guardAction = null;
      await action?.();
      if (!closeWindowState.isClosing) guardIsClosing = false;
    } catch (error) {
      guardAction = action;
      if (!closeWindowState.isClosing) guardIsClosing = action === closeWindowNow;
      showNotice(normalizeError(error).message, "error");
    } finally {
      guardBusy = false;
    }
  }

  async function publishFromEditor() {
    if (!session || activeTask || publishing) return;
    if (pendingImageUploads > 0) {
      showNotice($ui("{p0} 张图片还在上传，等完成后再发布。", { p0: pendingImageUploads }), "error");
      return;
    }
    publishing = true;
    const project = { ...session };
    const token = editorStore.documentToken();
    try {
      if (externalChange) throw new Error($ui("先处理云端更新，再发布。"));
      await fileStore.save();
      await editorStore.saveUntilClean();
      if (session?.projectId !== project.projectId || session.generation !== project.generation || !editorStore.matchesDocument(token)) throw new Error($ui("项目或文章切换了，请重新发布。"));
      const nextArticles = await platform.listArticles(project.projectId, project.generation);
      if (session?.projectId !== project.projectId || session.generation !== project.generation || !editorStore.matchesDocument(token)) throw new Error($ui("项目或文章切换了，请重新发布。"));
      await editorStore.saveUntilClean();
      if (session?.projectId !== project.projectId || session.generation !== project.generation || !editorStore.matchesDocument(token) || editorStore.getState().externalChange) throw new Error($ui("云端内容又更新了，处理后再重新发布。"));
      articles = nextArticles;
      await fileStore.save();
      // An image import can begin while the saves above are in flight.  The
      // initial guard alone is therefore not enough: starting Hexo here could
      // publish the temporary hlex-asset URL before the upload replaces it.
      if (pendingImageUploads > 0) throw new Error($ui("{p0} 张图片还在上传，等完成后再发布。", { p0: pendingImageUploads }));
      const task = await platform.startTask(project.projectId, "publish");
      publishTaskId = task.taskId;
      showNotice($ui("已在后台开始清理、生成、发布。"));
    } catch (error) {
      publishing = false;
      showNotice(normalizeError(error).message, "error");
    }
  }

  function installUpdate() {
    if (pendingImageUploads > 0 || activeTask || publishing || previewServer?.state === "starting" || previewServer?.state === "stopping") {
      showNotice($ui("等图片上传和 Hexo 任务结束后再安装更新。"), "error");
      return;
    }
    requestGuard($ui("安装更新前，先保存或放弃没保存的内容。"), async () => {
      try {
        await flushPendingConfig();
        if (pendingImageUploads > 0 || activeTask || publishing || previewServer?.state === "starting" || previewServer?.state === "stopping") {
          showNotice($ui("等当前任务结束再安装更新。"), "error");
          return;
        }
        await platform.installUpdate();
      }
      catch (error) { showNotice(normalizeError(error).message, "error"); }
    }, "both");
  }

  async function ensurePreviewRunning(project = session && { ...session }) {
    if (!project) throw new Error($ui("先打开一个博客项目。"));
    const assertCurrent = () => {
      if (session?.projectId !== project.projectId || session.generation !== project.generation) throw new Error($ui("项目已经切换，重新打开预览。"));
    };
    let view = await platform.getPreviewStatus(project.projectId, project.generation);
    assertCurrent();
    if (view.state !== "running") view = await platform.startPreviewServer(project.projectId, project.generation);
    for (let index = 0; index < 100 && view.state === "starting"; index += 1) {
      await new Promise((resolve) => setTimeout(resolve, 150));
      assertCurrent();
      view = await platform.getPreviewStatus(project.projectId, project.generation);
    }
    assertCurrent();
    previewServer = view;
    if (view.state !== "running") throw view.error ?? new Error($ui("预览服务还没就绪。"));
    return view;
  }

  async function previewProject(openInBrowser = true) {
    if (!session || previewBusy) return "";
    const project = { ...session };
    const token = editorStore.documentToken();
    const articleId = editorStore.getState().snapshot?.articleId;
    const assertCurrent = () => {
      if (session?.projectId !== project.projectId || session.generation !== project.generation || !editorStore.matchesDocument(token)) throw new Error($ui("文章已经切换，重新打开预览。"));
    };
    previewBusy = true;
    try {
      if (pendingImageUploads > 0) throw new Error($ui("等图片处理完再打开预览。"));
      if (externalChange) throw new Error($ui("先处理云端更新，再预览。"));
      await fileStore.save();
      await editorStore.saveUntilClean();
      assertCurrent();
      if (!articleId) throw new Error($ui("先打开一篇文章。"));
      await ensurePreviewRunning(project);
      assertCurrent();
      await editorStore.saveUntilClean();
      assertCurrent();
      const url = await platform.resolveArticlePreviewUrl(project.projectId, project.generation, articleId);
      assertCurrent();
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
      showNotice(normalizeError(error).message, "error");
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
        const saved = await persistConfigSnapshot(snapshot);
        if (revision === configRevision) config = saved;
      } catch (error) {
        showNotice(normalizeError(error).message, "error");
      }
    }, 350);
  }

  async function flushPendingConfig() {
    if (!configTimer) { await configSaveQueue; return; }
    clearTimeout(configTimer);
    configTimer = undefined;
    const revision = configRevision;
    const saved = await persistConfigSnapshot(structuredClone(config));
    if (revision === configRevision) config = saved;
  }

  async function saveConfigNow(next: AppConfigV3) {
    clearTimeout(configTimer);
    configTimer = undefined;
    const revision = ++configRevision;
    const saved = await persistConfigSnapshot(next);
    if (revision === configRevision) {
      config = saved;
      applyTheme(saved.appearance.themeMode);
    }
    return saved;
  }

  function persistConfigSnapshot(next: AppConfigV3): Promise<AppConfigV3> {
    const snapshot = structuredClone(next);
    const save = configSaveQueue.catch(() => undefined).then(() => platform.saveConfig(snapshot));
    configSaveQueue = save;
    return save;
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
    const effectiveSeverity = severity ?? "info";
    notice = message;
    noticeSeverity = effectiveSeverity;
    clearTimeout(noticeTimer);
    const duration = Math.max(5000, Math.min(12_000, message.length * 90));
    if (effectiveSeverity === "info") noticeTimer = setTimeout(() => (notice = ""), duration);
  }

  async function cancelActiveTask() {
    if (!activeTask || cancelingTask) return;
    const taskId = activeTask.taskId;
    cancelingTask = true;
    try {
      await platform.cancelTask(taskId);
      showNotice($ui("已请求停止。部署出去的部分不会撤回，去云端确认。"));
    } catch (error) { showNotice(normalizeError(error).message, "error"); }
    finally { cancelingTask = false; }
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

  function latestTaskFailure(events: TaskEvent[], taskId: string) {
    const output = events
      .filter((event) => event.taskId === taskId && event.kind === "log")
      .map((event) => event.line ?? "")
      .join("\n")
      .replace(/\x1b\[[0-9;]*[A-Za-z]/g, "");
    if (/文章信息格式无效|YAMLException|Process failed/i.test(output)) {
      return $ui("有文章的开头信息格式不对，改一下提示的那篇再试。");
    }
    if (/Authentication failed|Permission denied|publickey|could not read Username/i.test(output)) {
      return $ui("GitHub 验证失败，检查一下 Git 凭据再试。");
    }
    if (/Could not resolve host|timed out|Network is unreachable|connection reset/i.test(output)) {
      return $ui("网络连接不上，检查网络后再试。");
    }
    if (/输出了错误，发布已停止/.test(output)) {
      return $ui("生成时报错，发布已停止，不会上传半成品。");
    }
    return $ui("任务没有完成。如果到了部署阶段，先去云端确认结果。");
  }
</script>

<div class:is-maximized={maximized} class="app-window">
  <TitleBar
    documentTitle={page === "files" ? activeFilePath : activeDocumentTitle}
    dirty={dirty || filesDirty}
    onRequestClose={requestClose}
    onMaximizedChange={(value) => (maximized = value)}
  />
  <div class="app-body">
    <NavRail {page} onNavigate={navigate} />
    <main class="workspace">
      {#if !configLoaded}
        <LoadingState label={t("loading.workspace")} />
      {:else}
        {#key page}
          <PageTransition pageKey={page}>
            {#await pagePromise}
              <LoadingState label={t("loading.page")} />
            {:then module}
              {@const Page = module.default}
              <Page
            {session}
            {articles}
            {config}
            {taskEvents}
            {editorStore}
            {fileStore}
            {recentProjects}
            {previewServer}
            initialSection={settingsInitialSection}
            autoSaveSuspended={Boolean(guardAction) || Boolean(externalChange)}
            taskBusy={Boolean(activeTask) || publishing}
            {previewBusy}
            onOpenProject={openProject}
            onOpenArticle={openArticleFromFiles}
            onOpenRecentProject={openRecent}
            onArticlesChange={(next: ArticleSummary[]) => (articles = next)}
            onConfigChange={updateConfig}
            onSaveConfig={saveConfigNow}
            onBeforeSync={beforeSync}
            onThemePreview={(mode: AppConfigV3["appearance"]["themeMode"]) => applyTheme(mode)}
            onRemoveRecentProject={removeRecentProject}
            onClearRecentProjects={clearRecentProjectList}
            onPublish={publishFromEditor}
            onPreview={previewProject}
            onTogglePreviewServer={togglePreviewServer}
            onOpenPreviewHome={openPreviewHome}
            onNotice={showNotice}
            onPendingImageUploadsChange={(count: number) => (pendingImageUploads = count)}
            onInstallUpdate={installUpdate}
            onOpenUpdates={() => navigate("about")}
            onOpenSettings={(section?: SettingsSectionId) => navigate("settings", section ?? "maintenance")}
              />
            {/await}
          </PageTransition>
        {/key}
      {/if}
      <div class="status-toasts" aria-live="polite">
        {#if page !== "about" && ["available", "downloading", "verifying", "downloaded", "error"].includes($updateStore.status) && dismissedUpdateVersion !== `${$updateStore.latestVersion ?? ""}:${$updateStore.status}`}
          <div class="task-indicator notice-indicator" role="status">
            <span>{$ui(globalUpdateModel.stageLabel)}{#if $updateStore.latestVersion} · {$updateStore.latestVersion}{/if}{#if $updateStore.status === "downloading" && globalUpdateModel.percent !== null} · {Math.floor(globalUpdateModel.percent)}%{/if}</span>
            {#if globalUpdateModel.canInstall}<button class="button" type="button" on:click={installUpdate}>{$ui("安装更新")}</button>{:else}<button class="button" type="button" on:click={() => navigate("about")}>{$ui("查看更新")}</button>{/if}
            <button class="notice-close" type="button" aria-label={$ui("稍后")} on:click={() => (dismissedUpdateVersion = `${$updateStore.latestVersion ?? ""}:${$updateStore.status}`)}><X size={14} /></button>
          </div>
        {/if}
        {#if externalChange}
          <div class="task-indicator notice-indicator" role="status">
            <span>{externalChange === "deleted" ? $ui("这篇文章在云端被删了，本地内容还留着。") : $ui("云端有更新，暂时不能保存。")}</span>
            <button class="button" type="button" on:click={reviewExternalChange}>{$ui("查看差异")}</button>
          </div>
        {/if}
        {#if notice}
          <div
            class:error={noticeSeverity === "error"}
            class="task-indicator notice-indicator"
            role={noticeSeverity === "error" ? "alert" : "status"}
            in:fly={{ y: 8, duration: 160 }}
            out:fade={{ duration: 120 }}
          >
            <span>{notice}</span>
            <button class="notice-close" type="button" aria-label={$ui("关闭通知")} on:click={dismissNotice}><X size={14} /></button>
          </div>
        {/if}
        {#if activeTask}
          <div class="task-indicator" role="status" in:fly={{ y: 8, duration: 160 }} out:fade={{ duration: 120 }}>
            <LoaderCircle size={15} class="spin" />
            <span>{activeTask.step ?? $ui("任务运行中")}</span>
            <button class="button" type="button" on:click={() => (taskDetailsOpen = true)}>{$ui("任务详情")}</button>
          </div>
        {:else if taskEvents.length}
          <button class="button" type="button" on:click={() => (taskDetailsOpen = true)}>{$ui("查看最近任务")}</button>
        {/if}
      </div>
    </main>
  </div>
</div>

{#if taskDetailsOpen}
  <ModalDialog title={$ui("后台任务")} description={$ui("查看任务的阶段、输出和结果。取消不会撤回已经完成的云端操作。")} closeLabel={$ui("关闭")} onClose={() => (taskDetailsOpen = false)}>
    <p>{session?.name ?? $ui("博客项目")} · {activeTask ? $ui("运行中") : $ui("已结束")}</p>
    <pre class="task-log">{taskEvents.filter((event) => event.taskId === (activeTask?.taskId ?? taskEvents.at(-1)?.taskId)).map((event) => event.line ?? (event.kind === "finished" ? event.success ? $ui("任务完成") : $ui("任务未完成") : event.step ?? "")).filter(Boolean).join("\n")}</pre>
    <svelte:fragment slot="actions">
      <button class="button" type="button" on:click={() => (taskDetailsOpen = false)}>{$ui("关闭")}</button>
      {#if activeTask}<button class="button danger" type="button" disabled={cancelingTask} on:click={cancelActiveTask}>{cancelingTask ? $ui("正在请求停止") : $ui("取消任务")}</button>{/if}
    </svelte:fragment>
  </ModalDialog>
{/if}

{#if recoveryOpen && (externalChange || recoveryBusy)}
  <ModalDialog title={externalChange === "deleted" ? $ui("文章已在云端被删除") : $ui("本地和云端内容不一致")}
    description={$ui("本地内容会一直保留。选「使用云端」会替换你的编辑。")}
    onClose={() => !recoveryBusy && (recoveryOpen = false)}>
    <label class="field"><span>{$ui("本地内容")}</span><textarea class="input" rows="6" readonly value={recoveryLocal}></textarea></label>
    {#if externalChange === "changed"}<label class="field"><span>{$ui("云端内容")}</span><textarea class="input" rows="6" readonly value={recoveryRemote ?? ""}></textarea></label>{/if}
    <svelte:fragment slot="actions">
      <button class="button" type="button" disabled={recoveryBusy} on:click={() => (recoveryOpen = false)}>{$ui("稍后")}</button>
      <button class="button" type="button" disabled={recoveryBusy} on:click={() => resolveExternalChange("draft")}>{$ui("存为草稿")}</button>
      {#if externalChange === "changed"}
        <button class="button" type="button" disabled={recoveryBusy} on:click={() => resolveExternalChange("remote")}>{$ui("使用云端")}</button>
        <button class="button primary" type="button" disabled={recoveryBusy} on:click={() => resolveExternalChange("local")}>{$ui("保留本地并保存")}</button>
      {:else}
        <button class="button danger" type="button" disabled={recoveryBusy} on:click={() => resolveExternalChange("close")}>{$ui("放弃本地并关闭")}</button>
      {/if}
    </svelte:fragment>
  </ModalDialog>
{/if}

{#if guardAction}
  <ModalDialog
    title={guardIsClosing ? $ui("退出 Hexo Lite Editor？") : $ui("有未保存的内容")}
    description={guardDescription}
    onClose={() => !guardBusy && resolveGuard("cancel")}
  >
    <svelte:fragment slot="actions">
      <button class="button" type="button" disabled={guardBusy || closeWindowState.isClosing} on:click={() => resolveGuard("cancel")}>{$ui("取消")}</button>
      <button class="button danger" type="button" disabled={guardBusy || closeWindowState.isClosing} on:click={() => resolveGuard("discard")}>
        {guardIsClosing ? $ui("不保存退出") : $ui("放弃")}
      </button>
      <button class="button primary" type="button" data-autofocus disabled={guardBusy || closeWindowState.isClosing} on:click={() => resolveGuard("save")}>
        {guardBusy ? $ui("正在保存…") : guardIsClosing ? $ui("保存并退出") : $ui("保存并继续")}
      </button>
    </svelte:fragment>
  </ModalDialog>
{/if}

<style>
  .task-log { max-height: 360px; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; font-size: 12px; }
  :global(.spin) {
    animation: rotate 1s linear infinite;
  }
  @keyframes rotate {
    to { transform: rotate(360deg); }
  }
</style>
