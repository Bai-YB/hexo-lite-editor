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
  import type {
    AppConfigV3,
    AppPage,
    ArticleSummary,
    CloseWindowState,
    ProjectSessionView,
    PreviewServerView,
    RecentProjectView,
    SettingsSectionId,
    TaskEvent,
    UpdateSnapshot
  } from "$shared/types/app";
  import { EditorSessionStore } from "$features/editor/EditorSessionStore";
  import { FileSessionStore } from "$features/files/FileSessionStore";
  import type { SettingsController } from "$features/settings/controller";
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
  let guardCompletion: ((accepted: boolean) => void) | null = null;
  let guardDescription = "";
  let guardBusy = false;
  type GuardSource = "editor" | "files" | "documents" | "settings" | "both";
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
  let autoUpdateReady: UpdateSnapshot | null = null;
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

  onMount(async () => {
    window.addEventListener("keydown", handleShortcut);
    // Warm the small page modules so the first navigation is as quick as later ones.
    void Promise.all(Object.values(pageLoaders).map((load) => load()));
    try {
      const loaded = await platform.loadConfig();
      config = loaded.config;
      initI18n(config.general.language);
      applyTheme(config.appearance.themeMode);
      configLoaded = true;
      if (loaded.warnings.length) showNotice(loaded.warnings[0]);
      if (config.update.checkOnStart) {
        const lastCheckKey = "hexo-lite-editor:update-last-auto-check";
        const lastCheck = Number(localStorage.getItem(lastCheckKey) ?? "0");
        if (!Number.isFinite(lastCheck) || Date.now() - lastCheck >= 86_400_000) {
          setTimeout(() => {
            void (async () => {
              let update: UpdateSnapshot;
              try {
                update = await platform.checkUpdate();
                localStorage.setItem(lastCheckKey, String(Date.now()));
              } catch (error) {
                console.info("启动更新检查未完成", normalizeError(error).message);
                return;
              }
              if (update.status !== "available") return;
              showNotice(`发现新版本 ${update.latestVersion ?? ""}，正在后台安全下载。`);
              try {
                const downloaded = await platform.downloadUpdate();
                if (downloaded.status === "downloaded") {
                  autoUpdateReady = downloaded;
                  showNotice(`版本 ${downloaded.latestVersion ?? ""} 已下载并验证签名，可重启安装。`);
                }
              } catch (error) {
                showNotice(`自动下载更新失败：${normalizeError(error).message}`, "error");
              }
            })();
          }, 3000);
        }
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
      if (event.kind === "finished") {
        const isPublish = event.taskId === publishTaskId;
        if (event.success === false) {
          const reason = latestTaskFailure(taskEvents, event.taskId);
          showNotice(
            isPublish
              ? `发布失败：${reason || "请检查任务详情；若已进入部署阶段，请核对远端结果。"}`
              : reason || "任务执行失败，请检查项目设置或网络连接后重试。",
            "error"
          );
        } else if (isPublish) {
          showNotice("博客发布完成，新生成的站点已上传。" );
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
        showNotice(view.message || `内容同步：${view.status}`, "error");
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
          showNotice(`还有 ${pendingImageUploads} 张图片正在上传，请等待完成后再退出。`, "error");
          return;
        }
        const settingsDirty = settingsController?.hasDirty() ?? false;
        const editorDirty = editorStore.hasDirty() || editorStore.getState().saving;
        if (!editorDirty && !filesDirty && !settingsDirty && !activeTask && !publishing) {
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
    unsubscribeFiles();
    disposePluginWorkers();
  });

  function applyTheme(mode: AppConfigV3["appearance"]["themeMode"]) {
    document.documentElement.dataset.theme = mode;
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
      if (page === "settings") {
        void settingsController?.save().catch((error) => showNotice(normalizeError(error).message, "error"));
      } else if (page === "editor") {
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
      showNotice("图片正在上传并更新地址，请等待完成后再离开写作页。", "error");
      return;
    }
    if (next === "settings") settingsInitialSection = settingsSection;
    if (page === "files" && fileStore.hasDirty()) {
      requestGuard("离开全部文件前，请保存或放弃当前文件的修改。", () => { page = next; }, "files");
      return;
    }
    if (page === "settings" && settingsController?.hasDirty()) {
      requestGuard("离开设置前需要保存或放弃本次设置修改。", () => { page = next; }, "settings");
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
    requestGuard("打开博文前，请保存或放弃当前文章和项目文件的修改。", async () => {
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
      showNotice("请等待后台任务完成，或取消任务后再切换博客。", "error");
      return;
    }
    if (pendingImageUploads > 0) {
      showNotice("图片正在上传并更新地址，请等待完成后再切换博客。", "error");
      return;
    }
    requestGuard("切换项目前需要处理文章、项目文件或设置中的未保存内容。", async () => {
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
    if (recoveryBusy) return;
    if (activeTask || publishing) {
      taskDetailsOpen = true;
      showNotice("请等待后台任务完成，或取消任务后再切换博客。", "error");
      return;
    }
    if (pendingImageUploads > 0) {
      showNotice("图片正在上传并更新地址，请等待完成后再切换博客。", "error");
      return;
    }
    requestGuard("切换项目前需要处理文章、项目文件或设置中的未保存内容。", async () => {
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
      if (result === "changed" || result === "deleted") showNotice("云端文章已变化，本地内容已保留。请先处理版本差异再保存。");
      else if (result === "refreshed") showNotice("远端内容已应用，文章列表已刷新。");
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
      showNotice("文章版本已变化，请核对刷新后的内容，再选择处理方式。", "error");
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
          showNotice("云端内容已更新，请核对当前比较后再次选择。", "error");
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
            title: activeDocumentTitle || "恢复的文章", fileName: `recovered-${Date.now()}`, kind: "draft",
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
        showNotice("本地内容已另存为草稿，请核对文章中的相对图片路径。");
      }
      if (!editorStore.getState().externalChange) recoveryOpen = false;
    } catch (error) {
      showNotice(`${normalizeError(error).message}${choice === "draft" && recoveryDraft ? " 恢复草稿已创建，再次点击另存草稿会继续保存到同一文件。" : ""}`, "error");
    } finally { recoveryBusy = false; if (!editorStore.getState().externalChange) recoveryOpen = false; }
  }

  function beforeSync(): Promise<boolean> {
    if (guardAction || recoveryBusy || pendingImageUploads > 0 || externalChange) {
      showNotice("请先完成图片操作或处理当前文章的版本差异，再同步。", "error");
      return Promise.resolve(false);
    }
    return new Promise((resolve) => {
      guardCompletion = resolve;
      requestGuard("同步前，请保存或放弃文章和项目文件的修改。", () => { resolve(true); guardCompletion = null; }, "documents");
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
    if (recoveryBusy) return;
    if (guardAction) return;
    if (activeTask || publishing) {
      taskDetailsOpen = true;
      showNotice("后台任务仍在运行，请等待完成或在任务详情中取消后再退出。", "error");
      return;
    }
    if (pendingImageUploads > 0) {
      showNotice(`还有 ${pendingImageUploads} 张图片正在上传，请等待完成后再退出。`, "error");
      return;
    }
    const settingsDirty = settingsController?.hasDirty() ?? false;
    const editorDirty = editorStore.hasDirty() || editorStore.getState().saving;
    closeWindowState = {
      ...closeWindowState,
      hasUnsavedChanges: settingsDirty || editorDirty || filesDirty
    };
    if (!closeWindowState.hasUnsavedChanges) {
      void closeWindowNow();
      return;
    }
    if (guardAction && guardIsClosing) return;
    guardDescription = "文章、项目文件或设置中有未保存修改。保存后再退出可避免丢失内容。";
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
    const settingsDirty = settingsController?.hasDirty() ?? false;
    const editorDirty = editorStore.hasDirty() || editorStore.getState().saving;
    const hasDirty = source === "settings"
      ? settingsDirty
      : source === "both"
        ? settingsDirty || editorDirty || filesDirty
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
      if (guardSource === "settings" || guardSource === "both") {
        if (choice === "save") await settingsController?.save();
        else await settingsController?.discard();
        if (settingsController?.hasDirty()) throw new Error("设置还有新的修改，请保存后继续。");
      }
      if (["editor", "documents", "both"].includes(guardSource)) {
        if (choice === "save") await editorStore.saveUntilClean();
        else {
          await editorStore.waitForSave().catch(() => null);
          if (editorStore.getState().externalChange) throw new Error("文章已在云端变化，请取消并处理版本差异后继续。");
          editorStore.discard();
        }
      }
      if (["editor", "documents", "both"].includes(guardSource) && (editorStore.getState().externalChange || editorStore.hasDirty())) throw new Error("文章仍有未处理的修改，请处理后继续。");
      if (["files", "documents", "both"].includes(guardSource)) {
        if (choice === "save") await fileStore.save();
        else await fileStore.discard();
        if (fileStore.hasDirty()) throw new Error("项目文件仍有未保存修改，请处理后继续。");
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
    const project = { ...session };
    const token = editorStore.documentToken();
    try {
      if (externalChange) throw new Error("请先处理当前文章的版本差异再发布。");
      await fileStore.save();
      await editorStore.saveUntilClean();
      if (session?.projectId !== project.projectId || session.generation !== project.generation || !editorStore.matchesDocument(token)) throw new Error("当前项目或文章已变化，请重新发布。");
      const nextArticles = await platform.listArticles(project.projectId, project.generation);
      if (session?.projectId !== project.projectId || session.generation !== project.generation || !editorStore.matchesDocument(token)) throw new Error("当前项目或文章已变化，请重新发布。");
      await editorStore.saveUntilClean();
      if (session?.projectId !== project.projectId || session.generation !== project.generation || !editorStore.matchesDocument(token) || editorStore.getState().externalChange) throw new Error("当前文章已变化，请处理后重新发布。");
      articles = nextArticles;
      await fileStore.save();
      const task = await platform.startTask(project.projectId, "publish");
      publishTaskId = task.taskId;
      showNotice("正在后台清理缓存、重新生成并发布博客。" );
    } catch (error) {
      publishing = false;
      showNotice(normalizeError(error).message, "error");
    }
  }

  function installUpdate() {
    if (pendingImageUploads > 0 || activeTask || publishing || previewServer?.state === "starting" || previewServer?.state === "stopping") {
      showNotice("请等待图片上传、Hexo 或发布任务完成后再安装更新。", "error");
      return;
    }
    autoUpdateReady = null;
    requestGuard("安装更新前需要保存或放弃文章、项目文件和设置中的修改。", async () => {
      try { await platform.installUpdate(); }
      catch (error) { showNotice(normalizeError(error).message, "error"); }
    }, "both");
  }

  async function ensurePreviewRunning(project = session && { ...session }) {
    if (!project) throw new Error("请先打开博客项目。");
    const assertCurrent = () => {
      if (session?.projectId !== project.projectId || session.generation !== project.generation) throw new Error("博客项目已切换，请重新打开预览。");
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
    if (view.state !== "running") throw view.error ?? new Error("Hexo 预览尚未就绪。");
    return view;
  }

  async function previewProject(openInBrowser = true) {
    if (!session || previewBusy) return "";
    const project = { ...session };
    const token = editorStore.documentToken();
    const articleId = editorStore.getState().snapshot?.articleId;
    const assertCurrent = () => {
      if (session?.projectId !== project.projectId || session.generation !== project.generation || !editorStore.matchesDocument(token)) throw new Error("当前文章已变化，请重新打开预览。");
    };
    previewBusy = true;
    try {
      if (pendingImageUploads > 0) throw new Error("请等待图片处理完成后再打开预览。");
      if (externalChange) throw new Error("请先处理当前文章的版本差异再预览。");
      await fileStore.save();
      await editorStore.saveUntilClean();
      assertCurrent();
      if (!articleId) throw new Error("请先打开一篇文章。");
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
    const effectiveSeverity = severity
      ?? (/失败|错误|冲突|不可用|无法|未完成/.test(message) ? "error" : "info");
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
      showNotice("已请求停止任务。已经执行的部署操作不会被撤回，请核对远端结果。");
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
      return "文章开头的信息格式有误，请修正提示的文章后重试。";
    }
    if (/Authentication failed|Permission denied|publickey|could not read Username/i.test(output)) {
      return "GitHub 身份验证失败，请检查 Git 凭据后重试。";
    }
    if (/Could not resolve host|timed out|Network is unreachable|connection reset/i.test(output)) {
      return "网络连接失败，请检查网络后重试。";
    }
    if (/输出了错误，发布已停止/.test(output)) {
      return "生成过程发现错误，已停止发布，未上传不完整的站点。";
    }
    return "任务没有成功完成。若已进入部署阶段，请核对远端结果后再重试。";
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
            onRegisterSettingsController={(controller: SettingsController | null) => (settingsController = controller)}
            onRemoveRecentProject={removeRecentProject}
            onClearRecentProjects={clearRecentProjectList}
            onPublish={publishFromEditor}
            onPreview={previewProject}
            onTogglePreviewServer={togglePreviewServer}
            onOpenPreviewHome={openPreviewHome}
            onNotice={showNotice}
            onPendingImageUploadsChange={(count: number) => (pendingImageUploads = count)}
            onInstallUpdate={installUpdate}
            onOpenSettings={(section?: SettingsSectionId) => navigate("settings", section ?? "maintenance")}
              />
            {/await}
          </PageTransition>
        {/key}
      {/if}
      <div class="status-toasts" aria-live="polite">
        {#if externalChange}
          <div class="task-indicator notice-indicator" role="status">
            <span>{externalChange === "deleted" ? $ui("当前文章已在云端删除，本地内容已保留。") : $ui("当前文章有云端更新，保存已暂停。")}</span>
            <button class="button" type="button" on:click={reviewExternalChange}>{$ui("处理版本差异")}</button>
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
            <span>{activeTask.step ?? $ui("正在处理项目")}</span>
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
  <ModalDialog title={$ui("后台任务")} description={$ui("查看任务阶段、输出与结果。取消不会撤回已经完成的远端操作。")} closeLabel={$ui("关闭")} onClose={() => (taskDetailsOpen = false)}>
    <p>{session?.name ?? $ui("博客项目")} · {activeTask ? $ui("运行中") : $ui("已结束")}</p>
    <pre class="task-log">{taskEvents.filter((event) => event.taskId === (activeTask?.taskId ?? taskEvents.at(-1)?.taskId)).map((event) => event.line ?? (event.kind === "finished" ? event.success ? $ui("任务完成") : $ui("任务未完成") : event.step ?? "")).filter(Boolean).join("\n")}</pre>
    <svelte:fragment slot="actions">
      <button class="button" type="button" on:click={() => (taskDetailsOpen = false)}>{$ui("关闭")}</button>
      {#if activeTask}<button class="button danger" type="button" disabled={cancelingTask} on:click={cancelActiveTask}>{cancelingTask ? $ui("正在请求停止") : $ui("取消任务")}</button>{/if}
    </svelte:fragment>
  </ModalDialog>
{/if}

{#if recoveryOpen && (externalChange || recoveryBusy)}
  <ModalDialog title={externalChange === "deleted" ? $ui("保留已删除文章的本地内容") : $ui("处理文章版本差异")}
    description={$ui("本地内容会保留到你明确选择处理方式。使用云端将替换当前本地编辑。")}
    onClose={() => !recoveryBusy && (recoveryOpen = false)}>
    <label class="field"><span>{$ui("本地内容")}</span><textarea class="input" rows="6" readonly value={recoveryLocal}></textarea></label>
    {#if externalChange === "changed"}<label class="field"><span>{$ui("云端内容")}</span><textarea class="input" rows="6" readonly value={recoveryRemote ?? ""}></textarea></label>{/if}
    <svelte:fragment slot="actions">
      <button class="button" type="button" disabled={recoveryBusy} on:click={() => (recoveryOpen = false)}>{$ui("稍后处理")}</button>
      <button class="button" type="button" disabled={recoveryBusy} on:click={() => resolveExternalChange("draft")}>{$ui("另存草稿")}</button>
      {#if externalChange === "changed"}
        <button class="button" type="button" disabled={recoveryBusy} on:click={() => resolveExternalChange("remote")}>{$ui("使用云端")}</button>
        <button class="button primary" type="button" disabled={recoveryBusy} on:click={() => resolveExternalChange("local")}>{$ui("保留本地并保存")}</button>
      {:else}
        <button class="button danger" type="button" disabled={recoveryBusy} on:click={() => resolveExternalChange("close")}>{$ui("放弃并关闭文章")}</button>
      {/if}
    </svelte:fragment>
  </ModalDialog>
{/if}

{#if autoUpdateReady}
  <ModalDialog
    title={$ui("更新 {p0} 已准备好", { p0: autoUpdateReady.latestVersion ?? "" })}
    description={$ui("更新包已在应用内自动下载并通过签名验证。可以现在重启安装，也可以稍后在“关于”页面安装。")}
    onClose={() => (autoUpdateReady = null)}
  >
    <svelte:fragment slot="actions"><button class="button" type="button" on:click={() => (autoUpdateReady = null)}>{$ui("稍后")}</button><button class="button primary" type="button" data-autofocus on:click={installUpdate}>{$ui("重启并安装")}</button></svelte:fragment>
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
        {guardBusy ? $ui("处理中") : guardIsClosing ? $ui("保存并退出") : $ui("保存并继续")}
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
