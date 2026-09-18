<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { onDestroy, onMount, tick } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import {
    RefreshCw,
    Search,
    SlidersHorizontal,
    ImageOff,
    Link2,
    Unlink2,
    FileText,
    FolderOpen,
    Send,
    ArchiveRestore,
    Trash2
  } from "@lucide/svelte";
  import MarkdownEditor from "./MarkdownEditor.svelte";
  import EditorToolbar from "./EditorToolbar.svelte";
  import WelcomePanel from "./WelcomePanel.svelte";
  import {
    findPendingEditorImages,
    type InsertionBookmark,
    type EditorSessionStore,
    type EditorSessionState
  } from "./EditorSessionStore";
  import EmptyState from "$shared/components/EmptyState.svelte";
  import ErrorState from "$shared/components/ErrorState.svelte";
  import LoadingState from "$shared/components/LoadingState.svelte";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import {
    chunkPreviewImageSources,
    extractPreviewImageSources,
    isRemoteImageSource,
    renderMarkdownPreview,
    replacePreviewImageWithPlaceholder
  } from "$shared/markdown/safeMarkdown";
  import { isTauri, platform, normalizeError } from "$platform/tauri";
  import { hashPreviewText, PreviewBlockCache, renderPreviewBlocks, type PreviewBlock } from "$shared/markdown/previewBlocks";
  import { countWords, lineAt } from "$shared/markdown/wordCount";
  import { previewStateLabel } from "./previewModel";
  import type {
    AppConfigV3,
    ArticleKind,
    ArticleSummary,
    DocumentSnapshot,
    ProjectSessionView,
    RecentProjectView,
    SettingsSectionId,
    PreviewServerView,
    TaskType
  } from "$shared/types/app";
  import { PreviewAssetRegistry, replacePreviewAssetInPlace } from "./PreviewAssetRegistry";
  import PreviewModeSwitcher from "./preview/PreviewModeSwitcher.svelte";
  import { previewSurface } from "./preview/previewSurface";
  import HexoThemePreview from "./preview/HexoThemePreview.svelte";
  import type { PreviewMode } from "./preview/previewSession";
  import { createAdaptiveScheduler } from "./preview/renderScheduler";
  import { pluginForProvider, uploadWithPlugin } from "$shared/plugins/PluginProviderRuntime";
  import type { PluginView } from "$shared/plugins/types";
  import { articleFileName, localDateTime } from "./editorChanges";
  import { syncStatusLabel } from "./syncStatusLabel";
  import { collectPreviewAnchors, previewTopForSourceLine, ScrollSyncOwner, SCROLL_ANCHOR_INSET, type SourceAnchor } from "./preview/sourceScrollSync";

  export let session: ProjectSessionView | null;
  export let articles: ArticleSummary[] = [];
  export let config: AppConfigV3;
  export let editorStore: EditorSessionStore;
  export let recentProjects: RecentProjectView[] = [];
  export let onOpenProject: () => void;
  export let onOpenRecentProject: (recentId: string) => void = () => {};
  export let onArticlesChange: (articles: ArticleSummary[]) => void = () => {};
  export let onConfigChange: (config: AppConfigV3) => void = () => {};
  export let onNotice: (message: string, severity?: "info" | "error") => void = () => {};
  export let onPublish: () => void = () => {};
  export let onPreview: (openInBrowser?: boolean) => Promise<string> = async () => "";
  export let onTogglePreviewServer: () => void = () => {};
  export let onOpenPreviewHome: () => void = () => {};
  export let onOpenSettings: (section?: SettingsSectionId) => void = () => {};
  export let previewServer: PreviewServerView | null = null;
  export let taskBusy = false;
  export let previewBusy = false;
  export let autoSaveSuspended = false;
  export let onPendingImageUploadsChange: (count: number) => void = () => {};
  export let onBeforeSync: () => Promise<boolean> = async () => true;

  const store = editorStore;
  let content = "";
  let documentInstance = 0;
  let snapshot: DocumentSnapshot | null = null;
  let dirty = false;
  let saving = false;
  let savedAt: string | null = null;
  let sessionError: string | null = null;
  let externalChange: "changed" | "deleted" | null = null;
  let pendingCreate = false;
  let imageUrlReplacements: Record<string, string> = {};
  let selectionFrom = 0;
  let selectionTo = 0;
  let activeArticleId: string | null = store.activeArticleId();
  let query = "";
  let filter: "all" | ArticleKind = "all";
  let category = "";
  let tag = "";
  let sortMode: "modifiedDesc" | "createdDesc" | "dateDesc" | "titleAsc" = "modifiedDesc";
  let filterMenuOpen = false;
  let filterButton: HTMLButtonElement;
  let loading = false;
  let loadError = "";
  let failedArticle: ArticleSummary | null = null;
  let previewBlocks: PreviewBlock[] = [];
  let previewStyles: string[] = [];
  let previewLayoutToken = 0;
  const previewBlockCache = new PreviewBlockCache();
  let previewDocumentInstance = -1;
  let previewImageResults: Record<string, import("$shared/types/app").PreviewImageResult> = {};
  let previewRendering = false;
  let previewRenderSequence = 0;
  const previewJob = {
    source: "",
    instance: 0,
    imageResults: {} as Record<string, import("$shared/types/app").PreviewImageResult>,
    imagePending: false,
    sequence: 0,
    active: false
  };
  let previewScheduleKey = "";
  let previewHasRendered = false;
  const previewScheduler = createAdaptiveScheduler({ run: runPreviewRender });
  let autoSaveTimer: ReturnType<typeof setTimeout> | undefined;
  let lastProjectKey: string | null = store.getState().snapshot
    ? `${store.getState().snapshot!.projectId}:${store.getState().snapshot!.sessionGeneration}`
    : null;
  let pendingArticle: ArticleSummary | null = null;
  let showSwitchGuard = false;
  let switchBusy = false;
  let showCreate = false;
  let createTitle = "";
  let createFileName = "";
  let createFileNameEdited = false;
  let createKind: ArticleKind = "post";
  let createDate = localDateTime();
  let createTags = "";
  let createCategories = "";
  let createError = "";
  let creating = false;
  let articleWidth = config.layout.articleListWidth;
  let previewRatio = config.layout.previewRatio ?? 0.5;
  let editorGrid: HTMLDivElement;
  let imageInput: HTMLInputElement;
  let previewImageSources: string[] = [];
  let previewImageKey = "";
  let previewImagesPending = false;
  let lastValidatedImageKey = "";
  let imageValidationSequence = 0;
  let articleLoadSequence = 0;
  let editorScrollTop = 0;
  let editorScrollHeight = 0;
  let editorClientHeight = 0;
  let previewScrollSync = true;
  let markdownPreview: HTMLElement;
  let markdownEditor: MarkdownEditor | undefined;
  let previewAnchors: SourceAnchor[] = [];
  // Incremented whenever the preview DOM is replaced or remeasured.  Scroll
  // alignment must never consume coordinates collected from the previous DOM.
  let previewLayoutSequence = 0;
  let scrollSourceLine = 1;
  const scrollOwner = new ScrollSyncOwner();
  const editorScrollByArticle = new Map<string, number>();
  const previewScrollByArticle = new Map<string, number>();

  let componentAlive = true;
  let pendingImageUploads = 0;
  const activeImageUploads = new Set<string>();
  let unlistenFileDrop: (() => void) | undefined;
  let coverErrors: Record<string, string> = {};
  let coverFallbackUrls: Record<string, string> = {};
  const coverChecksInFlight = new Set<string>();
  const coverLastCheckedAt = new Map<string, number>();
  const coverRecheckIntervalMs = 60_000;
  let syncStatus: import("$shared/types/app").ContentSyncView = { enabled: false, status: "off", provider: "github", conflicts: [] };
  let syncBusy = false;
  let syncStatusSequence = 0;
  let unlistenSync: (() => void) | undefined;
  let articleResizeActive = false;
  let contentResizeActive = false;
  let articleContext: { article: ArticleSummary; x: number; y: number; opener: HTMLElement } | null = null;
  let articleContextMenu: HTMLDivElement;
  let deletingArticle: ArticleSummary | null = null;
  let articleActionBusy = false;
  const previewAssets = new PreviewAssetRegistry();
  let previewMode: PreviewMode = "quick";
  let plugins: PluginView[] = [];
  let themePreviewBusy = false;

  function applySessionState(state: EditorSessionState) {
    content = state.content;
    documentInstance = state.documentInstance;
    snapshot = state.snapshot;
    dirty = state.dirty;
    saving = state.saving;
    savedAt = state.savedAt;
    sessionError = state.error;
    externalChange = state.externalChange;
    imageUrlReplacements = state.imageUrlReplacements;
    selectionFrom = state.selection.from;
    selectionTo = state.selection.to;
    activeArticleId = state.snapshot?.articleId ?? null;
  }

  const unsubscribeStore = [
    store.subscribeContent(applySessionState),
    store.subscribeSelection(applySessionState),
    store.subscribeSaving(applySessionState),
    store.subscribeSession(applySessionState)
  ];

  onMount(() => {
    window.addEventListener("focus", handleWindowFocus);
    window.addEventListener("pointerdown", closeFilterMenu);
    window.addEventListener("keydown", closeFilterMenu);
    window.addEventListener("hexo-editor-new-article", openCreateDialog);
    void platform.onContentSyncStatus((view) => {
      if (!componentAlive || view.projectId !== session?.projectId || view.sessionGeneration !== session?.generation) return;
      syncStatusSequence += 1;
      syncStatus = view;
    }).then((unlisten) => { if (componentAlive) unlistenSync = unlisten; else unlisten(); }).catch(() => {});
    if (isTauri()) {
      void getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "drop") void handleImagePaths(event.payload.paths);
      }).then((unlisten) => { if (componentAlive) unlistenFileDrop = unlisten; else unlisten(); });
    }
    if (!session) return;
    void platform.listPlugins().then((items) => (plugins = items)).catch(() => (plugins = []));
    void refreshSyncStatus();
    if (!store.getState().snapshot && articles.length) void openArticle(articles[0]);
  });

  onDestroy(() => {
    componentAlive = false;
    unlistenFileDrop?.();
    unlistenSync?.();
    unsubscribeStore.forEach((off) => off());
    previewAssets.clear();
    clearTimeout(autoSaveTimer);
    previewScheduler.cancel();
    window.removeEventListener("focus", handleWindowFocus);
    window.removeEventListener("pointerdown", closeFilterMenu);
    window.removeEventListener("keydown", closeFilterMenu);
    window.removeEventListener("hexo-editor-new-article", openCreateDialog);
  });

  async function refreshSyncStatus(run = false) {
    if (!session || syncBusy) return;
    if (run && (pendingImageUploads > 0 || externalChange || showSwitchGuard)) {
      onNotice($ui("等图片上传完成、云端更新处理完后再同步。"), "error");
      return;
    }
    const project = session;
    const documentToken = store.documentToken();
    const statusSequence = syncStatusSequence;
    syncBusy = true;
    try {
      if (run && !await onBeforeSync()) return;
      if (run && (!componentAlive || session?.projectId !== project.projectId || session.generation !== project.generation || store.documentToken() !== documentToken)) return;
      if (run) await store.saveUntilClean();
      const result = run
        ? await platform.runContentSync(project.projectId, project.generation)
        : await platform.getContentSyncStatus(project.projectId, project.generation);
      if (!componentAlive || session?.projectId !== project.projectId) return;
      if (statusSequence === syncStatusSequence && session.generation === project.generation) syncStatus = result;
      if (run) onNotice(syncStatus.message || $ui("检查完成。"));
    } catch (error) {
      onNotice(normalizeError(error).message, "error");
    } finally {
      syncBusy = false;
    }
  }

  function handleSyncStatusClick() {
    if (!syncStatus.enabled || ["remoteAhead", "conflict", "authRequired"].includes(syncStatus.status)) {
      onOpenSettings("sync");
      return;
    }
    void refreshSyncStatus(true);
  }

  function closeFilterMenu(event: Event) {
    if (articleContext) {
      if (event instanceof KeyboardEvent && event.key === "Escape") {
        event.preventDefault();
        const opener = articleContext.opener;
        articleContext = null;
        requestAnimationFrame(() => opener.focus());
        return;
      }
      const target = event.target as HTMLElement;
      if (!(target.closest?.(".article-context-menu") || articleContext.opener.contains(target))) {
        articleContext = null;
      }
    }
    if (!filterMenuOpen) return;
    if (event instanceof KeyboardEvent) {
      if (event.key !== "Escape") return;
      event.preventDefault();
      filterMenuOpen = false;
      requestAnimationFrame(() => filterButton?.focus());
      return;
    }
    const target = event.target as HTMLElement;
    if (target.closest?.(".article-filter-popover, .filter-menu-button")) return;
    filterMenuOpen = false;
  }

  $: schedulePreviewRender(
    content,
    documentInstance,
    previewImageResults,
    previewImagesPending,
    config.layout.previewVisible,
    previewMode
  );
  $: previewImageKey = `${activeArticleId ?? ""}\u0000${previewImageSources.join("\u0000")}`;
  $: if (session && snapshot && previewImageKey !== lastValidatedImageKey) {
    lastValidatedImageKey = previewImageKey;
    void refreshPreviewImages();
  }
  $: availableCategories = [...new Set(articles.flatMap((article) => article.categories))].sort((a, b) => a.localeCompare(b, "zh-CN"));
  $: availableTags = [...new Set(articles.flatMap((article) => article.tags))].sort((a, b) => a.localeCompare(b, "zh-CN"));
  $: filteredArticles = articles
    .filter((article) => {
      const matchesFilter = filter === "all" || article.kind === filter;
      const matchesCategory = !category || article.categories.includes(category);
      const matchesTag = !tag || article.tags.includes(tag);
      const needle = query.trim().toLocaleLowerCase();
      const matchesQuery = !needle || [article.title, article.relativePath, ...article.tags, ...article.categories].some((value) => value.toLocaleLowerCase().includes(needle));
      return matchesFilter && matchesCategory && matchesTag && matchesQuery;
    })
    .sort((a, b) => {
      if (sortMode === "titleAsc") return a.title.localeCompare(b.title, "zh-CN");
      const field = sortMode === "createdDesc" ? "createdAt" : sortMode === "dateDesc" ? "frontMatterDate" : "modifiedAt";
      return Date.parse(b[field] ?? "") - Date.parse(a[field] ?? "");
    });

  function schedulePreviewRender(
    source: string,
    instance: number,
    imageResults: Record<string, import("$shared/types/app").PreviewImageResult>,
    imagePending: boolean,
    previewVisible: boolean,
    mode: PreviewMode
  ) {
    previewJob.source = source;
    previewJob.instance = instance;
    previewJob.imageResults = imageResults;
    previewJob.imagePending = imagePending;
    previewJob.sequence = ++previewRenderSequence;
    previewJob.active = previewVisible && mode === "quick";
    if (!previewJob.active) {
      previewRendering = false;
      previewScheduleKey = "";
      previewScheduler.cancel();
      return;
    }
    previewRendering = true;
    const key = `${instance}|${mode}`;
    const rebind = key !== previewScheduleKey;
    previewScheduleKey = key;
    // A new article or re-opened pane paints immediately; keystrokes coalesce.
    if (rebind && previewHasRendered) previewScheduler.force();
    else previewScheduler.schedule();
  }

  function runPreviewRender() {
    if (!componentAlive || !previewJob.active || previewJob.sequence !== previewRenderSequence) return;
    if (previewJob.instance !== previewDocumentInstance) {
      previewDocumentInstance = previewJob.instance;
      previewBlockCache.clear();
    }
    const startedAt = performance.now();
    const incremental = renderPreviewBlocks(previewJob.source, previewJob.imageResults, previewJob.imagePending, previewBlockCache, document);
    if (!componentAlive || previewJob.sequence !== previewRenderSequence) return;
    if (incremental) {
      previewBlocks = incremental.blocks;
      previewStyles = incremental.styles;
      previewImageSources = incremental.imageSources;
    } else {
      const rendered = renderMarkdownPreview(previewJob.source, previewJob.imageResults, previewJob.imagePending, true);
      if (!componentAlive || previewJob.sequence !== previewRenderSequence) return;
      previewBlocks = [{ key: `full:${hashPreviewText(rendered.html)}`, html: rendered.html }];
      previewStyles = [];
      previewImageSources = rendered.imageSources;
    }
    previewLayoutToken += 1;
    previewHasRendered = true;
    previewRendering = false;
    if (import.meta.env.DEV) {
      const durationMs = performance.now() - startedAt;
      const stats = previewStats();
      stats.renders += 1;
      stats.totalMs += durationMs;
      stats.maxMs = Math.max(stats.maxMs, durationMs);
      stats.blocks = previewBlocks.length;
      stats.reused = incremental?.reused ?? 0;
    }
  }

  function previewStats() {
    const scope = window as unknown as {
      __previewStats?: { renders: number; totalMs: number; maxMs: number; blocks: number; reused: number };
    };
    return (scope.__previewStats ??= { renders: 0, totalMs: 0, maxMs: 0, blocks: 0, reused: 0 });
  }
  $: wordCount = countWords(content);
  $: if (`${session?.projectId ?? ""}:${session?.generation ?? 0}` !== lastProjectKey) {
    lastProjectKey = session ? `${session.projectId}:${session.generation}` : null;
    const currentSnapshot = store.getState().snapshot;
    const snapshotMatchesSession = Boolean(
      session
      && currentSnapshot?.projectId === session.projectId
      && currentSnapshot.sessionGeneration === session.generation
    );
    activeArticleId = snapshotMatchesSession ? currentSnapshot?.articleId ?? null : null;
    if (!snapshotMatchesSession) store.clear();
    previewImageResults = {};
    previewImagesPending = false;
    coverErrors = {};
    coverFallbackUrls = {};
    coverChecksInFlight.clear();
    coverLastCheckedAt.clear();
    lastValidatedImageKey = "";
    syncStatus = { enabled: false, status: "off", provider: "github", conflicts: [] };
    if (session) {
      void refreshSyncStatus();
      if (snapshotMatchesSession && currentSnapshot) {
        resumePendingImageUploads(currentSnapshot.content, currentSnapshot.articleId);
      }
      if (!snapshotMatchesSession && articles.length) void openArticle(articles[0]);
    }
  }

  $: {
    clearTimeout(autoSaveTimer);
    if (!autoSaveSuspended && !showSwitchGuard && !switchBusy && !deletingArticle && !externalChange && config.general.autoSave && dirty && !saving) {
      autoSaveTimer = setTimeout(() => void saveCurrent(), config.general.autoSaveDelayMs);
    }
  }

  async function openArticle(article: ArticleSummary) {
    if (!session) return;
    const sequence = ++articleLoadSequence;
    loading = true;
    loadError = "";
    failedArticle = null;
    const expectedId = article.articleId;
    const expectedProjectId = session.projectId;
    const expectedGeneration = session.generation;
    try {
      const snapshot = await platform.loadDocument(
        session.projectId,
        article.articleId,
        session.generation
      );
      if (
        !componentAlive || sequence !== articleLoadSequence
        ||
        expectedId !== snapshot.articleId
        || session?.projectId !== expectedProjectId
        || session.generation !== expectedGeneration
        || snapshot.projectId !== expectedProjectId
        || snapshot.sessionGeneration !== expectedGeneration
      ) return;
      activeArticleId = article.articleId;
      claimEditorScroll();
      editorScrollTop = editorScrollByArticle.get(article.articleId) ?? 0;
      store.load(snapshot);
      resumePendingImageUploads(snapshot.content, article.articleId);
      previewImageResults = {};
      previewImagesPending = false;
      lastValidatedImageKey = "";
      requestAnimationFrame(() => {
        if (markdownPreview && !previewScrollSync) markdownPreview.scrollTop = previewScrollByArticle.get(article.articleId) ?? 0;
      });
    } catch (error) {
      if (componentAlive && sequence === articleLoadSequence && session?.projectId === expectedProjectId && session.generation === expectedGeneration) {
        loadError = normalizeError(error).message;
        failedArticle = article;
      }
    } finally {
      if (sequence === articleLoadSequence) loading = false;
    }
  }

  function requestArticle(article: ArticleSummary) {
    if (article.articleId === activeArticleId) return;
    if (pendingImageUploads > 0) {
      onNotice($ui("图片还在上传，等完成后再切换文章。"));
      return;
    }
    if (externalChange) {
      onNotice($ui("云端有更新，先处理差异再切换文章。"), "error");
      return;
    }
    if (dirty) {
      clearTimeout(autoSaveTimer);
      pendingArticle = article;
      showSwitchGuard = true;
    } else {
      void openArticle(article);
    }
  }

  function handleArticleKeydown(event: KeyboardEvent, article: ArticleSummary) {
    if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
    event.preventDefault();
    const index = filteredArticles.findIndex((item) => item.articleId === article.articleId);
    const offset = event.key === "ArrowDown" ? 1 : -1;
    const next = filteredArticles[index + offset];
    if (!next) return;
    requestArticle(next);
    requestAnimationFrame(() => {
      document.querySelector<HTMLElement>(`[data-article-id="${next.articleId}"]`)?.focus();
    });
  }

  async function resolveSwitch(action: "save" | "discard" | "cancel") {
    if (switchBusy) return;
    if (action === "cancel") {
      pendingArticle = null;
      pendingCreate = false;
      showSwitchGuard = false;
      return;
    }
    switchBusy = true;
    try {
      if (action === "save") { await store.saveUntilClean(); await saveAndRefresh(); }
      else { await store.waitForSave().catch(() => null); store.discard(); }
      const next = pendingArticle;
      pendingArticle = null;
      showSwitchGuard = false;
      if (pendingCreate) {
        pendingCreate = false;
        openCreateDialog();
        return;
      }
      if (next) await openArticle(next);
    } catch (error) {
      onNotice(normalizeError(error).message, "error");
    } finally {
      switchBusy = false;
    }
  }

  async function saveCurrent() {
    try {
      await saveAndRefresh();
    } catch (error) {
      onNotice(normalizeError(error).message, "error");
    }
  }

  async function saveAndRefresh() {
    const project = session;
    await store.save();
    if (!project) return;
    const next = await platform.listArticles(project.projectId, project.generation);
    if (!componentAlive || session?.projectId !== project.projectId || session.generation !== project.generation) return;
    articles = next;
    onArticlesChange(next);
  }

  function openCreateDialog() {
    if (!session) return;
    if (pendingImageUploads > 0 || externalChange) {
      onNotice($ui("等图片上传完成、云端更新处理完后再新建文章。"), "error");
      return;
    }
    if (dirty) {
      clearTimeout(autoSaveTimer);
      pendingCreate = true;
      showSwitchGuard = true;
      return;
    }
    createTitle = "";
    createFileName = "";
    createFileNameEdited = false;
    createKind = "post";
    createDate = localDateTime();
    createTags = "";
    createCategories = "";
    createError = "";
    showCreate = true;
  }

  $: if (!createFileNameEdited) {
    createFileName = articleFileName(createTitle);
  }

  async function createArticle() {
    if (creating || pendingImageUploads > 0) return;
    if (!session || !createTitle.trim() || !createFileName.trim()) {
      createError = $ui("标题和文件名不能为空。");
      return;
    }
    creating = true;
    createError = "";
    try {
      const project = session;
      const token = store.documentToken();
      if (snapshot) await store.saveUntilClean();
      if (!componentAlive || session?.projectId !== project.projectId || session.generation !== project.generation) return;
      const summary = await platform.createArticle({
        projectId: project.projectId,
        sessionGeneration: project.generation,
        title: createTitle.trim(),
        fileName: createFileName.trim(),
        kind: createKind,
        date: createDate.replace("T", " "),
        tags: splitLabels(createTags),
        categories: splitLabels(createCategories)
      });
      if (!componentAlive || session?.projectId !== project.projectId || session.generation !== project.generation) return;
      const next = [summary, ...articles];
      articles = next;
      onArticlesChange(next);
      showCreate = false;
      if (store.documentToken() === token && !store.hasDirty()) await openArticle(summary);
      else onNotice($ui("文章已创建，但当前文章还在编辑，没有切换过去。"));
    } catch (error) {
      createError = normalizeError(error).message;
    } finally {
      creating = false;
    }
  }

  function splitLabels(value: string) {
    return [...new Set(value.split(/[,，\n]+/).map((item) => item.trim()).filter(Boolean))];
  }

  function changePendingImages(delta: number) {
    pendingImageUploads = Math.max(0, pendingImageUploads + delta);
    onPendingImageUploadsChange(pendingImageUploads);
  }

  function requireImageTarget(bookmark: InsertionBookmark, project: ProjectSessionView) {
    if (!componentAlive || !store.matchesDocument(bookmark.documentInstance) || session?.projectId !== project.projectId || session.generation !== project.generation) {
      throw new Error($ui("文章已经切换，图片没有插入，请回到原来的文章重试。"));
    }
    if (!bookmark.valid) throw new Error($ui("选中的文字变了，编辑没有丢，重新选一次插入位置。"));
  }

  async function importPluginImages(plugin: PluginView, files: Array<{ name: string; mime: string; bytes: number[] }>) {
    const results = await Promise.all(files.map(async (file) => {
      try {
        const uploaded = await uploadWithPlugin(plugin, file);
        return { fileName: file.name, url: uploaded.url, markdown: uploaded.markdown ?? `![${file.name}](${uploaded.url})` };
      } catch (error) {
        return { fileName: file.name, error: normalizeError(error) };
      }
    }));
    return results;
  }

  async function handleImageFiles(files: File[]) {
    if (!session || !snapshot || !files.length || showSwitchGuard || showCreate || deletingArticle || externalChange) return;
    const project = session;
    const provider = config.imageBed.defaultProvider;
    const bookmark = store.createInsertionBookmark();
    changePendingImages(1);
    try {
      const ordered = [];
      for (const file of files) {
        ordered.push({ name: file.name || `image-${Date.now()}.png`, mime: file.type, bytes: Array.from(new Uint8Array(await file.arrayBuffer())) });
      }
      requireImageTarget(bookmark, project);
      const plugin = pluginForProvider(plugins, provider);
      const results = plugin
        ? await importPluginImages(plugin, ordered)
        : await platform.importEditorImages(project.projectId, project.generation, provider, ordered);
      await finishImageImport(results, bookmark, project);
    } catch (error) {
      onNotice(normalizeError(error).message, "error");
    } finally {
      store.releaseInsertionBookmark(bookmark);
      changePendingImages(-1);
    }
  }

  async function finishImageImport(results: import("$shared/types/app").ImageImportResult[], bookmark: InsertionBookmark, project: ProjectSessionView) {
    requireImageTarget(bookmark, project);
    const successes = results.filter((result) => result.markdown);
    const articleId = store.activeArticleId()!;
    if (successes.length && !store.insertMarkdown(successes.map((result) => result.markdown).join("\n"), bookmark)) return;
    const failed = results.filter((result) => result.error);
    if (failed.length) onNotice($ui("{p0} 张图片已处理，{p1} 张失败：{p2}", { p0: successes.length, p1: failed.length, p2: failed[0].error?.message ?? "" }), "error");
    else onNotice($ui("{p0} 张图片已插入文章。", { p0: successes.length }));
    const pending = successes.filter((result) => result.uploadId && result.url);
    if (pending.length) {
      await store.save();
      requireImageTarget({ ...bookmark, valid: true }, project);
      await Promise.all(pending.map((result) => uploadPendingImage(result.uploadId!, result.url!, articleId)));
    }
    if (componentAlive) void refreshPreviewImages(true);
  }

  function showArticleContext(event: MouseEvent, article: ArticleSummary, opener: HTMLElement) {
    event.preventDefault();
    const width = 210;
    const height = 190;
    articleContext = {
      article,
      x: Math.max(6, Math.min(event.clientX, window.innerWidth - width - 6)),
      y: Math.max(6, Math.min(event.clientY, window.innerHeight - height - 6)),
      opener
    };
    requestAnimationFrame(() => articleContextMenu?.querySelector<HTMLButtonElement>("button")?.focus());
  }

  function handleArticleMenuKeydown(event: KeyboardEvent, article: ArticleSummary) {
    if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) {
      event.preventDefault();
      const target = event.currentTarget as HTMLElement;
      const bounds = target.getBoundingClientRect();
      showArticleContext(
        new MouseEvent("contextmenu", { clientX: bounds.left + 20, clientY: bounds.top + 20 }),
        article,
        target
      );
      return;
    }
    handleArticleKeydown(event, article);
  }

  async function revealArticle(article: ArticleSummary) {
    if (!session) return;
    articleContext = null;
    try {
      await platform.revealArticle(session.projectId, session.generation, article.articleId);
    } catch (error) {
      onNotice(normalizeError(error).message, "error");
    }
  }

  async function moveArticle(article: ArticleSummary, kind: ArticleKind) {
    if (!session || articleActionBusy || taskBusy || pendingImageUploads > 0) return;
    articleContext = null;
    articleActionBusy = true;
    const project = session;
    try {
      if (article.articleId === activeArticleId) await store.saveUntilClean();
      const updated = await platform.moveArticle(
        project.projectId,
        project.generation,
        article.articleId,
        kind
      );
      if (!componentAlive || session?.projectId !== project.projectId || session.generation !== project.generation) return;
      const next = articles.map((item) => item.articleId === updated.articleId ? updated : item);
      articles = next;
      onArticlesChange(next);
      if (article.articleId === activeArticleId) void refreshPreviewImages(true);
      onNotice(kind === "post" ? $ui("已转为正式文章。") : $ui("已移到草稿。"));
    } catch (error) {
      onNotice(normalizeError(error).message, "error");
    } finally {
      articleActionBusy = false;
    }
  }

  async function confirmDeleteArticle() {
    if (!session || !deletingArticle || articleActionBusy || taskBusy || pendingImageUploads > 0) return;
    const article = deletingArticle;
    articleActionBusy = true;
    try {
      await platform.deleteArticle(session.projectId, session.generation, article.articleId);
      const next = articles.filter((item) => item.articleId !== article.articleId);
      articles = next;
      onArticlesChange(next);
      deletingArticle = null;
      if (article.articleId === activeArticleId) {
        store.clear();
        activeArticleId = null;
        if (next.length) await openArticle(next[0]);
      }
      onNotice($ui("已移到系统回收站，需要的话可以从回收站恢复。"));
    } catch (error) {
      onNotice(normalizeError(error).message, "error");
    } finally {
      articleActionBusy = false;
    }
  }

  async function handleImagePaths(paths: string[]) {
    const imagePaths = paths.filter((path) => /\.(?:png|jpe?g|gif|webp)$/i.test(path));
    if (!session || !snapshot || !imagePaths.length || showSwitchGuard || showCreate || deletingArticle || externalChange) return;
    const project = session;
    const provider = config.imageBed.defaultProvider;
    const bookmark = store.createInsertionBookmark();
    changePendingImages(1);
    try {
      const plugin = pluginForProvider(plugins, provider);
      let results: import("$shared/types/app").ImageImportResult[];
      if (plugin) {
        const files = await platform.readPluginEditorImagePaths(project.projectId, project.generation, imagePaths);
        requireImageTarget(bookmark, project);
        results = await importPluginImages(plugin, files);
      } else {
        results = await platform.importEditorImagePaths(project.projectId, project.generation, provider, imagePaths);
      }
      await finishImageImport(results, bookmark, project);
    } catch (error) {
      onNotice(normalizeError(error).message, "error");
    } finally {
      store.releaseInsertionBookmark(bookmark);
      changePendingImages(-1);
    }
  }

  async function uploadPendingImage(uploadId: string, localUrl: string, articleId: string) {
    if (!session || activeImageUploads.has(uploadId)) return;
    const projectId = session.projectId;
    const generation = session.generation;
    const token = store.documentToken();
    activeImageUploads.add(uploadId);
    previewAssets.register({ uploadId, localPreviewUrl: localUrl, status: "uploading" });
    changePendingImages(1);
    try {
      const result = await platform.uploadCachedEditorImage(projectId, generation, articleId, uploadId);
      if (result.url) {
        if (!componentAlive || !store.matchesDocument(token) || session?.projectId !== projectId || session.generation !== generation) return;
        if (markdownPreview) replacePreviewAssetInPlace(markdownPreview, uploadId, result.url);
        previewAssets.resolveRemote(uploadId, result.url);
        const replaced = store.replaceMarkdownImageUrl(localUrl, result.url, articleId);
        await store.save();
        if (!store.matchesDocument(token)) return;
        const savedGeneration = store.getState().snapshot?.sessionGeneration ?? generation;
        await platform.finalizeCachedEditorImage(projectId, savedGeneration, uploadId);
        onNotice(replaced ? $ui("图片已上传，文章里的地址已更新。") : $ui("图片已上传。"));
        setTimeout(() => void refreshPreviewImages(true), 0);
      }
    } catch (error) {
      previewAssets.markFailed(uploadId);
      onNotice($ui("图片上传失败，本地文件还在：{p0}", { p0: normalizeError(error).message }), "error");
    } finally {
      activeImageUploads.delete(uploadId);
      changePendingImages(-1);
    }
  }

  function resumePendingImageUploads(content: string, articleId: string) {
    for (const pending of findPendingEditorImages(content)) {
      void uploadPendingImage(pending.uploadId, pending.localUrl, articleId);
    }
  }

  function togglePreview() {
    const next = {
      ...config,
      layout: { ...config.layout, previewVisible: !config.layout.previewVisible }
    };
    onConfigChange(next);
  }

  async function openThemePreview() {
    if (!session || !activeArticleId || themePreviewBusy || previewBusy) return;
    const token = store.documentToken();
    themePreviewBusy = true;
    try {
      const url = await onPreview(false);
      if (url && componentAlive && store.matchesDocument(token)) await platform.openHexoPreviewWebview(url);
    } catch (error) { onNotice(normalizeError(error).message, "error"); }
    finally { themePreviewBusy = false; }
  }

  async function runAdvanced(kind: TaskType) {
    if (!session) return;
    try {
      await platform.startTask(session.projectId, kind);
      onNotice(kind === "gitStatus" ? $ui("Git 检查已在后台开始。") : $ui("任务已在后台开始。"));
    } catch (error) {
      onNotice(normalizeError(error).message, "error");
    }
  }

  function recordEditorScroll(value: number, scrollHeight: number, clientHeight: number, line = 1) {
    editorScrollTop = value;
    editorScrollHeight = scrollHeight;
    editorClientHeight = clientHeight;
    if (activeArticleId) editorScrollByArticle.set(activeArticleId, value);
    if (scrollOwner.canDrive("editor")) {
      scrollSourceLine = line;
      alignPreviewToSource();
    }
  }

  function recordPreviewScroll() {
    if (activeArticleId && markdownPreview) previewScrollByArticle.set(activeArticleId, markdownPreview.scrollTop);
  }

  function claimEditorScroll() {
    scrollOwner.claim("editor");
  }

  function claimPreviewScroll() {
    // Reading the preview is its own activity: until the editor scrolls again,
    // layout rebuilds must leave the preview where the user put it.
    scrollOwner.claim("preview");
  }

  function alignPreviewToSource(useEditorBounds = true) {
    if (!previewScrollSync || !markdownPreview?.isConnected || !previewAnchors.length) return;
    const editorMaxScroll = Math.max(0, editorScrollHeight - editorClientHeight);
    const previewMaxScroll = Math.max(0, markdownPreview.scrollHeight - markdownPreview.clientHeight);
    const target = scrollSourceLine <= 1 || (useEditorBounds && editorScrollTop <= 1)
      ? 0
      : !Number.isFinite(scrollSourceLine) || (useEditorBounds && editorScrollTop >= editorMaxScroll - 1)
        ? previewMaxScroll
        : Math.max(0, Math.min(previewMaxScroll, previewTopForSourceLine(previewAnchors, scrollSourceLine) - SCROLL_ANCHOR_INSET));
    if (Math.abs(markdownPreview.scrollTop - target) > 0.5) markdownPreview.scrollTop = target;
  }

  function togglePreviewScrollSync() {
    previewScrollSync = !previewScrollSync;
    if (previewScrollSync) {
      claimEditorScroll();
      scrollSourceLine = markdownEditor?.sourceLineAtScroll() ?? 1;
      alignPreviewToSource();
    }
  }

  function observePreviewLayout(node: HTMLElement, _layoutToken: number) {
    let frame = 0;
    let alive = true;
    let layoutSequence = ++previewLayoutSequence;
    const rebuild = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        if (!alive || !node.isConnected || layoutSequence !== previewLayoutSequence) return;
        const nextAnchors = collectPreviewAnchors(node);
        // A DOM replacement can briefly produce no markers while Svelte is
        // committing the new HTML. Keep the old coordinates out of the
        // synchronizer during that window, then align once the new layout is
        // measured.
        previewAnchors = nextAnchors;
        if (scrollOwner.canDrive("editor")) {
          scrollSourceLine = markdownEditor?.sourceLineAtScroll() ?? scrollSourceLine;
          // Preserve the source location when images, fonts or pane widths change.
          alignPreviewToSource();
        }
      });
    };
    const resize = new ResizeObserver(rebuild);
    const observeBlocks = () => {
      resize.disconnect();
      resize.observe(node);
      node.querySelectorAll<HTMLElement>("[data-source-line], img").forEach((element) => resize.observe(element));
      rebuild();
    };
    const mutation = new MutationObserver(observeBlocks);
    mutation.observe(node, { childList: true, subtree: true });
    for (const event of ["wheel", "pointerdown", "touchstart", "keydown"]) node.addEventListener(event, claimPreviewScroll, { passive: true });
    node.addEventListener("load", rebuild, true);
    observeBlocks();
    return {
      update() {
        // Invalidate anchors synchronously.  Otherwise an editor geometry
        // event between the HTML update and the next tick can map against the
        // previous article/content and visibly jump to the wrong section.
        layoutSequence = ++previewLayoutSequence;
        cancelAnimationFrame(frame);
        previewAnchors = [];
        void tick().then(() => { if (alive && layoutSequence === previewLayoutSequence) observeBlocks(); });
      },
      destroy() {
        alive = false;
        ++previewLayoutSequence;
        cancelAnimationFrame(frame);
        resize.disconnect();
        mutation.disconnect();
        node.removeEventListener("load", rebuild, true);
        for (const event of ["wheel", "pointerdown", "touchstart", "keydown"]) node.removeEventListener(event, claimPreviewScroll);
        previewAnchors = [];
      }
    };
  }

  function startArticleResize(event: PointerEvent) {
    const startX = event.clientX;
    const startWidth = articleWidth;
    articleResizeActive = true;
    beginResize(event, (moveEvent) => {
      const delta = moveEvent.clientX - startX;
      articleWidth = clamp(startWidth + delta, 220, 420);
    }, () => {
      articleResizeActive = false;
      onConfigChange({
        ...config,
        layout: { ...config.layout, articleListWidth: articleWidth }
      });
    });
  }

  function resetArticleWidth() {
    articleWidth = 280;
    onConfigChange({ ...config, layout: { ...config.layout, articleListWidth: articleWidth } });
  }

  function handleArticleResizeKeydown(event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    articleWidth = clamp(articleWidth + (event.key === "ArrowRight" ? 8 : -8), 220, 420);
    onConfigChange({ ...config, layout: { ...config.layout, articleListWidth: articleWidth } });
  }

  function beginResize(
    event: PointerEvent,
    onMove: (event: PointerEvent) => void,
    onEnd: () => void
  ) {
    event.preventDefault();
    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture?.(event.pointerId);
    document.body.classList.add("is-col-resizing");
    const move = (moveEvent: PointerEvent) => {
      onMove(moveEvent);
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", up);
      document.body.classList.remove("is-col-resizing");
      onEnd();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up, { once: true });
    window.addEventListener("pointercancel", up, { once: true });
  }

  function handlePreviewInteraction(event: MouseEvent | KeyboardEvent) {
    if (event instanceof KeyboardEvent && event.key !== "Enter" && event.key !== " ") return;
    const retry = (event.target as HTMLElement).closest<HTMLElement>("[data-preview-image-retry]");
    if (retry) {
      event.preventDefault();
      const placeholder = retry.closest<HTMLElement>("[data-image-source]");
      const source = placeholder?.dataset.imageSource;
      if (source && isRemoteImageSource(source)) {
        const image = document.createElement("img");
        image.src = source;
        image.dataset.imageSource = source;
        image.alt = placeholder?.getAttribute("aria-label") ?? "";
        image.loading = "lazy";
        const style = placeholder?.getAttribute("style");
        if (style) image.setAttribute("style", style);
        placeholder?.replaceWith(image);
      } else {
        void refreshPreviewImages(true);
      }
      return;
    }
    const target = (event.target as HTMLElement).closest<HTMLElement>("[data-external-href]");
    const url = target?.dataset.externalHref;
    if (!url) return;
    event.preventDefault();
    void platform.openMarkdownLink(url).catch((error) => onNotice(normalizeError(error).message, "error"));
  }

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, value));
  }

  async function refreshPreviewImages(force = false) {
    if (!session || !snapshot) return;
    const sources = extractPreviewImageSources(content);
    const validationKey = `${activeArticleId ?? ""}\u0000${sources.join("\u0000")}`;
    if (!force && validationKey !== previewImageKey) return;
    const sequence = ++imageValidationSequence;
    const expectedArticleId = activeArticleId;
    const expectedProjectId = session.projectId;
    const expectedGeneration = session.generation;
    previewImagesPending = sources.length > 0;
    previewImageResults = {};
    if (!sources.length) {
      previewImagesPending = false;
      return;
    }
    try {
      const results: import("$shared/types/app").PreviewImageResult[] = [];
      for (const batchSources of chunkPreviewImageSources(sources)) {
        const batch = await platform.resolveArticlePreviewImages({
          projectId: expectedProjectId,
          sessionGeneration: expectedGeneration,
          articleId: expectedArticleId ?? "",
          sources: batchSources
        });
        results.push(...batch);
      }
      if (
        sequence !== imageValidationSequence
        || activeArticleId !== expectedArticleId
        || session?.projectId !== expectedProjectId
        || session.generation !== expectedGeneration
      ) return;
      previewImageResults = Object.fromEntries(results.map((result) => [result.originalSource, result]));
    } catch (error) {
      if (sequence === imageValidationSequence) {
        previewImageResults = Object.fromEntries(sources.map((originalSource) => [originalSource, {
          originalSource,
          state: "unavailable",
          failureKind: "network",
          message: normalizeError(error).message
        }]));
      }
    } finally {
      if (sequence === imageValidationSequence) previewImagesPending = false;
    }
  }

  async function handlePreviewImageError(event: Event) {
    if (!(event.target instanceof HTMLImageElement)) return;
    const image = event.target;
    const originalSource = image.dataset.imageSource || image.getAttribute("src") || "";
    if (session && activeArticleId && isRemoteImageSource(originalSource) && !image.dataset.fallbackAttempted) {
      image.dataset.fallbackAttempted = "true";
      try {
        const [result] = await platform.resolveArticlePreviewImages({
          projectId: session.projectId,
          sessionGeneration: session.generation,
          articleId: activeArticleId,
          sources: [originalSource]
        });
        if (image.isConnected && result?.state === "ready" && result.previewUrl) {
          image.src = result.previewUrl;
          return;
        }
        if (image.isConnected) {
          replacePreviewImageWithPlaceholder(image, result?.message ?? $ui("图片加载失败或内容为空"));
        }
        return;
      } catch (error) {
        if (image.isConnected) replacePreviewImageWithPlaceholder(image, normalizeError(error).message);
        return;
      }
    }
    replacePreviewImageWithPlaceholder(image, $ui("图片加载失败或内容为空"));
  }

  function coverSource(article: ArticleSummary) {
    return article.cover.originalSource || article.cover.previewUrl || "";
  }

  function clearCoverError(source: string) {
    const { [source]: _removed, ...remaining } = coverErrors;
    coverErrors = remaining;
  }

  async function checkFailedCover(article: ArticleSummary, force = false) {
    const originalSource = coverSource(article);
    if (!session || !originalSource || !isRemoteImageSource(originalSource)) return;
    const lastCheckedAt = coverLastCheckedAt.get(originalSource) ?? 0;
    if (
      coverChecksInFlight.has(originalSource)
      || (!force && Date.now() - lastCheckedAt < coverRecheckIntervalMs)
    ) return;
    const expectedProjectId = session.projectId;
    const expectedGeneration = session.generation;
    coverChecksInFlight.add(originalSource);
    coverLastCheckedAt.set(originalSource, Date.now());
    try {
      const [result] = await platform.resolveArticlePreviewImages({
        projectId: expectedProjectId,
        sessionGeneration: expectedGeneration,
        articleId: article.articleId,
        sources: [originalSource]
      });
      if (session?.projectId !== expectedProjectId || session.generation !== expectedGeneration) return;
      if (result?.state === "ready" && result.previewUrl) {
        coverFallbackUrls = { ...coverFallbackUrls, [originalSource]: result.previewUrl };
        clearCoverError(originalSource);
      } else {
        coverErrors = { ...coverErrors, [originalSource]: result?.message ?? $ui("图片加载失败或内容为空") };
      }
    } catch (error) {
      if (session?.projectId === expectedProjectId && session.generation === expectedGeneration) {
        coverErrors = { ...coverErrors, [originalSource]: normalizeError(error).message };
      }
    } finally {
      coverChecksInFlight.delete(originalSource);
    }
  }

  function handleCoverError(article: ArticleSummary) {
    const originalSource = coverSource(article);
    if (!originalSource) return;
    coverErrors = { ...coverErrors, [originalSource]: coverErrors[originalSource] ?? $ui("图片加载失败，正在后台重试") };
    if (coverFallbackUrls[originalSource]) {
      const { [originalSource]: _removed, ...remaining } = coverFallbackUrls;
      coverFallbackUrls = remaining;
    }
    void checkFailedCover(article, true);
  }

  async function recheckFailedCovers(articleList: ArticleSummary[]) {
    await Promise.all(articleList
      .filter((article) => Boolean(coverErrors[coverSource(article)]))
      .map((article) => checkFailedCover(article)));
  }

  function startContentResize(event: PointerEvent) {
    contentResizeActive = true;
    beginResize(event, (moveEvent) => {
      const bounds = editorGrid.getBoundingClientRect();
      const available = Math.max(1, bounds.width - articleWidth - 4);
      const writingWidth = moveEvent.clientX - bounds.left - articleWidth;
      const minimumShare = Math.min(0.4, Math.max(0.15, 240 / available));
      const writingRatio = clamp(writingWidth / available, minimumShare, 1 - minimumShare);
      previewRatio = 1 - writingRatio;
    }, () => {
      contentResizeActive = false;
      onConfigChange({
        ...config,
        layout: { ...config.layout, previewRatio }
      });
    });
  }

  function resetPreviewRatio() {
    previewRatio = 0.5;
    onConfigChange({ ...config, layout: { ...config.layout, previewRatio } });
  }

  function handleContentResizeKeydown(event: KeyboardEvent) {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    previewRatio = clamp(previewRatio + (event.key === "ArrowLeft" ? 0.02 : -0.02), 0.15, 0.85);
    onConfigChange({ ...config, layout: { ...config.layout, previewRatio } });
  }

  function handleWindowFocus() {
    void refreshVisibleImages();
  }

  async function refreshVisibleImages() {
    if (!session) return;
    const expectedProjectId = session.projectId;
    const expectedGeneration = session.generation;
    await refreshPreviewImages(true);
    try {
      const next = await platform.listArticles(expectedProjectId, expectedGeneration);
      if (session?.projectId !== expectedProjectId || session.generation !== expectedGeneration) return;
      articles = next;
      onArticlesChange(next);
      void recheckFailedCovers(next);
    } catch {
      // Image refresh failures do not interrupt editing.
    }
  }

</script>

<div class="editor-page">
  {#if session}
    <EditorToolbar
      {session}
      {recentProjects}
      previewVisible={config.layout.previewVisible}
      {previewServer}
      {taskBusy}
      {previewBusy}
      saving={saving}
      saveDisabled={!dirty || saving || Boolean(externalChange)}
      saveTitle={externalChange ? $ui("云端有更新，先处理差异再保存。") : ""}
      publishDisabled={Boolean(externalChange)}
      publishTitle={externalChange ? $ui("云端有更新，处理后再发布。") : ""}
      imageDisabled={!snapshot}
      {onOpenProject}
      {onOpenRecentProject}
      onPreview={() => void onPreview(true)}
      onCreate={openCreateDialog}
      onSelectImages={() => imageInput?.click()}
      onSave={saveCurrent}
      onTogglePreview={togglePreview}
      onRunAdvanced={(task) => void runAdvanced(task)}
      {onTogglePreviewServer}
      {onOpenPreviewHome}
      {onOpenSettings}
      {onPublish}
    />
  {:else}
    <header class="editor-toolbar"><strong class="editor-toolbar-brand">Hexo Lite Editor</strong></header>
  {/if}
  <input hidden bind:this={imageInput} type="file" accept="image/png,image/jpeg,image/gif,image/webp" multiple on:change={(event) => { void handleImageFiles(Array.from(event.currentTarget.files ?? [])); event.currentTarget.value = ""; }} />

  {#if !session}
    <WelcomePanel {recentProjects} {onOpenProject} {onOpenRecentProject} {onOpenSettings} />
  {:else}
    <div class="editor-grid-wrap" style={`--article-width:${articleWidth}px; --writing-ratio:${config.layout.previewVisible ? 1 - previewRatio : 1}; --preview-ratio:${previewRatio}`}>
      <div class:preview-hidden={!config.layout.previewVisible} class="editor-grid" bind:this={editorGrid}>
        <aside class="article-pane" aria-label={$ui("文章列表")}>
          <div class="pane-toolbar">
            <Search size={15} aria-hidden="true" />
            <input class="input" bind:value={query} aria-label={$ui("搜索文章")} placeholder={$ui("搜索标题或路径")} />
          </div>
          <div class="filter-row" aria-label={$ui("文章筛选")}>
            <button class:active={filter === "all"} class="filter-chip" type="button" on:click={() => (filter = "all")}>{$ui("全部")}</button>
            <button class:active={filter === "post"} class="filter-chip" type="button" on:click={() => (filter = "post")}>{$ui("文章")}</button>
            <button class:active={filter === "draft"} class="filter-chip" type="button" on:click={() => (filter = "draft")}>{$ui("草稿")}</button>
            <span class="filter-spacer"></span>
             <button bind:this={filterButton} class:active={Boolean(category || tag || sortMode !== "modifiedDesc")} class="filter-menu-button" type="button" aria-label={$ui("筛选与排序")} aria-expanded={filterMenuOpen} on:click={() => (filterMenuOpen = !filterMenuOpen)}><SlidersHorizontal size={15} /></button>
          </div>
          {#if filterMenuOpen}
            <div class="article-filter-popover">
              <label><span>{$ui("分类")}</span><select class="select" bind:value={category}><option value="">{$ui("全部分类")}</option>{#each availableCategories as item}<option value={item}>{item}</option>{/each}</select></label>
              <label><span>{$ui("标签")}</span><select class="select" bind:value={tag}><option value="">{$ui("全部标签")}</option>{#each availableTags as item}<option value={item}>{item}</option>{/each}</select></label>
              <label><span>{$ui("排序")}</span><select class="select" bind:value={sortMode}><option value="modifiedDesc">{$ui("最近修改")}</option><option value="createdDesc">{$ui("创建时间")}</option><option value="dateDesc">{$ui("文章日期")}</option><option value="titleAsc">{$ui("标题 A–Z")}</option></select></label>
              <div class="filter-popover-actions"><button class="button" type="button" on:click={() => { category = ""; tag = ""; sortMode = "modifiedDesc"; }}>{$ui("重置")}</button><button class="button primary" type="button" on:click={() => (filterMenuOpen = false)}>{$ui("完成")}</button></div>
            </div>
          {/if}
          <div class="article-list">
            {#if !filteredArticles.length}
              <EmptyState title={$ui("没有匹配的文章")} description={$ui("换个搜索词或筛选条件试试。")} icon={Search} />
            {:else}
              {#each filteredArticles as article (article.articleId)}
                <button
                  class:active={activeArticleId === article.articleId}
                  class="article-item"
                  type="button"
                  data-article-id={article.articleId}
                  on:click={() => requestArticle(article)}
                  on:contextmenu={(event) => showArticleContext(event, article, event.currentTarget)}
                  on:keydown={(event) => handleArticleMenuKeydown(event, article)}
                >
                  {#if config.articleList.showCover}
                    {@const originalCoverSource = coverSource(article)}
                    {@const coverUrl = coverFallbackUrls[originalCoverSource] || article.cover.previewUrl || article.cover.originalSource}
                    {#if coverErrors[originalCoverSource]}
                      <span class="article-cover image-error" title={coverErrors[originalCoverSource]} aria-label={coverErrors[originalCoverSource]}><ImageOff size={18} /></span>
                    {:else if coverUrl}
                      <img class="article-cover" src={coverUrl} alt={article.cover.alt} loading="lazy" decoding="async" on:error={() => handleCoverError(article)} />
                    {:else}
                      <span class="article-cover placeholder" aria-hidden="true">{article.title.slice(0, 1)}</span>
                    {/if}
                  {/if}
                  <span class="article-copy"><span class="article-title">{article.title}</span><span class="article-meta">{article.kind === "draft" ? $ui("草稿") : $ui("文章")} · {new Date(article.modifiedAt).toLocaleDateString()}</span></span>
                  {#if activeArticleId === article.articleId && dirty}<span class="dirty-dot" title={$ui("未保存")}></span>{/if}
                </button>
              {/each}
            {/if}
          </div>
        </aside>
        <main class="writing-pane">
          {#if loading}
            <LoadingState label={$ui("正在读取文章")} />
          {:else if loadError}
            <ErrorState message={failedArticle ? $ui("读不了“{p0}”：{p1}", { p0: failedArticle.title, p1: loadError }) : loadError}><button class="button" type="button" disabled={!failedArticle} on:click={() => failedArticle && openArticle(failedArticle)}>{$ui("重试")}</button>{#if snapshot}<button class="button" type="button" on:click={() => { loadError = ""; failedArticle = null; }}>{$ui("返回当前文章")}</button>{/if}</ErrorState>
          {:else if snapshot}
            <MarkdownEditor
              bind:this={markdownEditor}
              content={content}
              documentInstance={documentInstance}
              imageUrlReplacements={imageUrlReplacements}
              fontSize={config.editor.fontSize}
              lineHeight={config.editor.lineHeight}
              showLineNumbers={config.editor.showLineNumbers}
              lineWrapping={config.editor.lineWrapping}
              highlightLine={config.editor.highlightActiveLine}
              tabSize={config.editor.tabSize}
              selectionFrom={selectionFrom}
              selectionTo={selectionTo}
              scrollTop={editorScrollTop}
              onChange={(content, changes) => store.update(content, changes)}
              onSelectionChange={(from, to) => store.setSelection(from, to)}
              onScroll={recordEditorScroll}
              onScrollInteraction={claimEditorScroll}
              onImageFiles={(files) => void handleImageFiles(files)}
              onSave={saveCurrent}
              onNewArticle={openCreateDialog}
            />
          {:else}
            <EmptyState title={$ui("选择一篇文章")} description={$ui("从左侧列表选一篇文章，这里就会显示内容。")} />
          {/if}
        </main>
        {#if config.layout.previewVisible}
          <div
            class:dragging={contentResizeActive}
            class="content-resize-handle"
            role="slider"
            tabindex="0"
            aria-label={$ui("调整编辑与预览比例")}
            aria-orientation="vertical"
            aria-valuemin="15"
            aria-valuemax="85"
            aria-valuenow={Math.round(previewRatio * 100)}
            on:pointerdown={startContentResize}
            on:dblclick={resetPreviewRatio}
            on:keydown={handleContentResizeKeydown}
          ></div>
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <section class="preview-pane" aria-label={$ui("文章预览")}>
            <div class="preview-mode-bar">
              <PreviewModeSwitcher mode={previewMode} onChange={(mode) => (previewMode = mode)} />
              <span>{previewMode === "quick" ? (previewRendering ? $ui("正在渲染…") : previewImagesPending ? $ui("正在读取图片…") : $ui("预览已就绪")) : $ui("主题预览中")}</span>
              <button
                class:active={previewScrollSync}
                class="icon-button small"
                type="button"
                aria-pressed={previewScrollSync}
                disabled={previewMode !== "quick"}
                title={previewScrollSync ? $ui("停止预览跟随") : $ui("开启预览跟随")}
                on:click={togglePreviewScrollSync}
              >
                {#if previewScrollSync}<Link2 size={14} />{:else}<Unlink2 size={14} />{/if}
              </button>
            </div>
            {#if !snapshot}
              <EmptyState title={$ui("暂无预览")} description={$ui("打开文章后，这里会显示预览。")} />
            {:else if previewMode === "theme"}
              <HexoThemePreview running={previewServer?.state === "running"} busy={previewBusy || themePreviewBusy} onOpen={() => void openThemePreview()} onReload={() => void openThemePreview()} />
            {:else}
              <!-- Keyboard users need to focus the independently scrollable preview. -->
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_noninteractive_tabindex -->
              <article class="markdown-preview" bind:this={markdownPreview} use:previewSurface={{ blocks: previewBlocks, styles: previewStyles, token: previewLayoutToken }} use:observePreviewLayout={previewLayoutToken} tabindex="0" aria-label={$ui("文章预览")} on:scroll={recordPreviewScroll} on:click={handlePreviewInteraction} on:keydown={handlePreviewInteraction} on:error|capture={handlePreviewImageError}></article>
            {/if}
          </section>
        {/if}
      </div>
      <div
        class:dragging={articleResizeActive}
        class="resize-handle"
        style={`left:${articleWidth}px`}
        role="slider"
        tabindex="0"
        aria-label={$ui("调整文章列表宽度")}
        aria-orientation="vertical"
        aria-valuemin="220"
        aria-valuemax="420"
        aria-valuenow={articleWidth}
        on:pointerdown={startArticleResize}
        on:dblclick={resetArticleWidth}
        on:keydown={handleArticleResizeKeydown}
      ></div>
    </div>
  {/if}

  <footer class="editor-status">
    <span class:error={Boolean(sessionError)} title={sessionError || undefined}>{sessionError ? $ui("保存失败") : saving ? $ui("正在保存…") : dirty ? $ui("未保存") : snapshot ? $ui("已保存") : ""}</span>
    <span>{$ui("{p0} 字", { p0: wordCount })}</span>
    {#if snapshot}<span>{$ui("第 {p0} 行", { p0: lineAt(content, selectionFrom) })}</span>{/if}
    <span class="status-spacer"></span>
    {#if session}<button class={`status-sync ${syncStatus.status}`} type="button" disabled={syncBusy || syncStatus.status === "checking"} title={syncStatus.message || $ui("云端同步")} on:click={handleSyncStatusClick}><RefreshCw size={12} class={syncBusy || syncStatus.status === "checking" ? "spin" : undefined} />{syncBusy ? $ui("正在同步…") : syncStatus.status === "conflict" ? $ui("{p0} 个文件有冲突", { p0: syncStatus.conflicts.length }) : $ui(syncStatusLabel(syncStatus))}</button>{/if}
    {#if session?.warnings.length}<span class="status-warning" title={session.warnings.join("；")}>{$ui("{p0} 条诊断", { p0: session.warnings.length })}</span>{/if}
    {#if savedAt}<span>{$ui("上次保存 {p0}", { p0: new Date(savedAt).toLocaleTimeString() })}</span>{/if}
    {#if session}<span>{$ui("网站预览")} {$ui(previewStateLabel(previewServer?.state))}</span>{/if}
  </footer>
</div>

{#if articleContext}
  <div bind:this={articleContextMenu} class="article-context-menu quiet-menu" role="menu" style={`left:${articleContext.x}px;top:${articleContext.y}px`}>
    <button type="button" role="menuitem" on:click={() => { const article = articleContext!.article; articleContext = null; requestArticle(article); }}><FileText size={14} />{$ui("打开文章")}</button>
    <button type="button" role="menuitem" on:click={() => revealArticle(articleContext!.article)}><FolderOpen size={14} />{$ui("在文件夹中显示")}</button>
    <div class="menu-separator"></div>
    {#if articleContext.article.kind === "draft"}
      <button type="button" role="menuitem" disabled={taskBusy || articleActionBusy} on:click={() => moveArticle(articleContext!.article, "post")}><Send size={14} />{$ui("转为正式文章")}</button>
    {:else}
      <button type="button" role="menuitem" disabled={taskBusy || articleActionBusy} on:click={() => moveArticle(articleContext!.article, "draft")}><ArchiveRestore size={14} />{$ui("移到草稿")}</button>
    {/if}
    <div class="menu-separator"></div>
    <button class="danger" type="button" role="menuitem" disabled={taskBusy || articleActionBusy} on:click={() => { deletingArticle = articleContext!.article; articleContext = null; }}><Trash2 size={14} />{$ui("移到回收站")}</button>
  </div>
{/if}

{#if deletingArticle}
  <ModalDialog title={$ui("将文章移到回收站？")} description={$ui("“{p0}”会移到系统回收站，可以找回；图片和资源目录保留。{p1}", { p0: deletingArticle.title, p1: deletingArticle.articleId === activeArticleId && dirty ? $ui("未保存的修改也会丢失。") : "" })} onClose={() => !articleActionBusy && (deletingArticle = null)}>
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={articleActionBusy} data-autofocus on:click={() => (deletingArticle = null)}>{$ui("取消")}</button><button class="button danger" type="button" disabled={articleActionBusy} on:click={confirmDeleteArticle}>{articleActionBusy ? $ui("正在处理…") : $ui("移到回收站")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if showSwitchGuard}
  <ModalDialog title={$ui("保存当前文章？")} description={$ui("切换文章前，先处理未保存的内容。")} onClose={() => resolveSwitch("cancel")}>
    <svelte:fragment slot="actions">
      <button class="button" type="button" disabled={switchBusy} on:click={() => resolveSwitch("cancel")}>{$ui("取消")}</button>
      <button class="button danger" type="button" disabled={switchBusy || saving} on:click={() => resolveSwitch("discard")}>{$ui("放弃修改")}</button>
      <button class="button primary" type="button" data-autofocus disabled={switchBusy} on:click={() => resolveSwitch("save")}>{switchBusy ? $ui("正在保存…") : $ui("保存并继续")}</button>
    </svelte:fragment>
  </ModalDialog>
{/if}

{#if showCreate}
  <ModalDialog title={$ui("新建文章")} description={$ui("文件名可以用中文，不安全的字符会自动去掉。日期按你电脑的时间。")} onClose={() => !creating && (showCreate = false)}>
    <div class="content-stack">
      <label class="field"><span>{$ui("标题")}</span><input class="input" bind:value={createTitle} data-autofocus placeholder={$ui("例如：我的第一篇文章")} /></label>
      <label class="field"><span>{$ui("文件名")}</span><input class="input" bind:value={createFileName} on:input={() => (createFileNameEdited = true)} placeholder={$ui("不填就从标题生成")} /></label>
      {#if createFileNameEdited}<button class="button quiet" type="button" on:click={() => (createFileNameEdited = false)}>{$ui("由标题生成文件名")}</button>{/if}
      <label class="field"><span>{$ui("类型")}</span><select class="select" bind:value={createKind}><option value="post">{$ui("文章")}</option><option value="draft">{$ui("草稿")}</option></select></label>
      <label class="field"><span>{$ui("日期")}</span><input class="input" type="datetime-local" bind:value={createDate} /></label>
      <label class="field"><span>{$ui("标签")}</span><input class="input" bind:value={createTags} placeholder={$ui("逗号或换行分隔")} /></label>
      <label class="field"><span>{$ui("分类")}</span><input class="input" bind:value={createCategories} placeholder={$ui("逗号或换行分隔")} /></label>
      {#if createError}<div class="badge warning" role="alert">{createError}</div>{/if}
    </div>
    <svelte:fragment slot="actions">
      <button class="button" type="button" disabled={creating} on:click={() => (showCreate = false)}>{$ui("取消")}</button>
      <button class="button primary" type="button" disabled={creating} on:click={createArticle}>{creating ? $ui("正在创建…") : $ui("创建并打开")}</button>
    </svelte:fragment>
  </ModalDialog>
{/if}
