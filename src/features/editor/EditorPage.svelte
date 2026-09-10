<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { onDestroy, onMount } from "svelte";
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
    renderSafeMarkdown,
    replacePreviewImageWithPlaceholder
  } from "$shared/markdown/safeMarkdown";
  import { isTauri, platform, normalizeError } from "$platform/tauri";
  import { previewStateLabel } from "./previewModel";
  import type {
    AppConfigV3,
    ArticleKind,
    ArticleSummary,
    ProjectSessionView,
    RecentProjectView,
    SettingsSectionId,
    PreviewServerView,
    TaskType
  } from "$shared/types/app";
  import { PreviewAssetRegistry, replacePreviewAssetInPlace } from "./PreviewAssetRegistry";
  import PreviewModeSwitcher from "./preview/PreviewModeSwitcher.svelte";
  import HexoThemePreview from "./preview/HexoThemePreview.svelte";
  import type { PreviewMode } from "./preview/previewSession";
  import { pluginForProvider, uploadWithPlugin } from "$shared/plugins/PluginProviderRuntime";
  import type { PluginView } from "$shared/plugins/types";
  import { articleFileName, localDateTime } from "./editorChanges";
  import { syncStatusLabel } from "./syncStatusLabel";

  export let session: ProjectSessionView | null;
  export let articles: ArticleSummary[] = [];
  export let config: AppConfigV3;
  export let editorStore: EditorSessionStore;
  export let recentProjects: RecentProjectView[] = [];
  export let onOpenProject: () => void;
  export let onOpenRecentProject: (recentId: string) => void = () => {};
  export let onArticlesChange: (articles: ArticleSummary[]) => void = () => {};
  export let onConfigChange: (config: AppConfigV3) => void = () => {};
  export let onNotice: (message: string) => void = () => {};
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
  let editorState: EditorSessionState = store.getState();
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
  let previewHtml = "";
  let previewImageResults: Record<string, import("$shared/types/app").PreviewImageResult> = {};
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
  let previewScrollSync = true;
  let markdownPreview: HTMLElement;
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

  const unsubscribe = store.subscribe((state) => {
    editorState = state;
    activeArticleId = state.snapshot?.articleId ?? null;
  });

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
    unsubscribe();
    previewAssets.clear();
    clearTimeout(autoSaveTimer);
    window.removeEventListener("focus", handleWindowFocus);
    window.removeEventListener("pointerdown", closeFilterMenu);
    window.removeEventListener("keydown", closeFilterMenu);
    window.removeEventListener("hexo-editor-new-article", openCreateDialog);
  });

  async function refreshSyncStatus(run = false) {
    if (!session || syncBusy) return;
    if (run && (pendingImageUploads > 0 || editorState.externalChange || showSwitchGuard)) {
      onNotice("请先完成图片处理并解决文章外部更改，再同步。");
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
      if (run) onNotice(syncStatus.message || "同步检查完成。");
    } catch (error) {
      onNotice(normalizeError(error).message);
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

  $: previewImageSources = extractPreviewImageSources(editorState.content);
  $: previewImageKey = `${activeArticleId ?? ""}\u0000${previewImageSources.join("\u0000")}`;
  $: if (session && editorState.snapshot && previewImageKey !== lastValidatedImageKey) {
    lastValidatedImageKey = previewImageKey;
    void refreshPreviewImages();
  }
  $: previewHtml = renderSafeMarkdown(
    editorState.content,
    previewImageResults,
    previewImagesPending
  );
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
  $: wordCount = countWords(editorState.content);
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
    if (!autoSaveSuspended && !showSwitchGuard && !switchBusy && !deletingArticle && !editorState.externalChange && config.general.autoSave && editorState.dirty && !editorState.saving) {
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
      editorScrollTop = editorScrollByArticle.get(article.articleId) ?? 0;
      store.load(snapshot);
      resumePendingImageUploads(snapshot.content, article.articleId);
      previewImageResults = {};
      previewImagesPending = false;
      lastValidatedImageKey = "";
      requestAnimationFrame(() => {
        if (markdownPreview) markdownPreview.scrollTop = previewScrollByArticle.get(article.articleId) ?? 0;
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
      onNotice("图片正在上传并更新地址，请等待完成后再切换文章。");
      return;
    }
    if (editorState.externalChange) {
      onNotice("请先处理当前文章的外部更改，再切换文章。");
      return;
    }
    if (editorState.dirty || editorState.saving) {
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
      if (next) await openArticle(next);
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      switchBusy = false;
    }
  }

  async function saveCurrent() {
    try {
      await saveAndRefresh();
    } catch (error) {
      onNotice(normalizeError(error).message);
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
    if (pendingImageUploads > 0 || editorState.externalChange) {
      onNotice("请先等待图片处理完成并解决文章外部更改，再新建文章。");
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
      createError = "请填写标题和文件名。";
      return;
    }
    creating = true;
    createError = "";
    try {
      const project = session;
      const token = store.documentToken();
      if (editorState.snapshot) await store.saveUntilClean();
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
      else onNotice("文章已创建；当前文章有新的编辑，已保留当前内容。");
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
      throw new Error("文章会话已变化，图片未插入其他文章，请在原文章中重试。");
    }
    if (!bookmark.valid) throw new Error("插入位置的选中文字已被修改，已保留您的编辑，请重新选择图片插入位置。");
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
    if (!session || !editorState.snapshot || !files.length || showSwitchGuard || showCreate || deletingArticle || editorState.externalChange) return;
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
      onNotice(normalizeError(error).message);
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
    if (failed.length) onNotice(`${successes.length} 张图片已处理，${failed.length} 张失败：${failed[0].error?.message}`);
    else onNotice(`${successes.length} 张图片已插入文章。`);
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
      onNotice(normalizeError(error).message);
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
      onNotice(kind === "post" ? "文章已从草稿发布到文章列表。" : "文章已移到草稿。" );
    } catch (error) {
      onNotice(normalizeError(error).message);
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
      onNotice("文章已移到系统回收站，可从回收站恢复。" );
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      articleActionBusy = false;
    }
  }

  async function handleImagePaths(paths: string[]) {
    const imagePaths = paths.filter((path) => /\.(?:png|jpe?g|gif|webp)$/i.test(path));
    if (!session || !editorState.snapshot || !imagePaths.length || showSwitchGuard || showCreate || deletingArticle || editorState.externalChange) return;
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
      onNotice(normalizeError(error).message);
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
        onNotice(replaced ? "图片已上传，文章中的地址已自动更新。" : "图片已上传。");
        setTimeout(() => void refreshPreviewImages(true), 0);
      }
    } catch (error) {
      previewAssets.markFailed(uploadId);
      onNotice(`图片上传失败，本地图片已保留：${normalizeError(error).message}`);
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
    } catch (error) { onNotice(normalizeError(error).message); }
    finally { themePreviewBusy = false; }
  }

  async function runAdvanced(kind: TaskType) {
    if (!session) return;
    try {
      await platform.startTask(session.projectId, kind);
      onNotice(`${kind === "gitStatus" ? "Git 检查" : "任务"}已在后台开始。`);
    } catch (error) {
      onNotice(normalizeError(error).message);
    }
  }

  function recordEditorScroll(value: number, scrollHeight: number, clientHeight: number) {
    editorScrollTop = value;
    if (activeArticleId) editorScrollByArticle.set(activeArticleId, value);
    if (previewScrollSync && markdownPreview) {
      const editorRange = Math.max(1, scrollHeight - clientHeight);
      const previewRange = Math.max(0, markdownPreview.scrollHeight - markdownPreview.clientHeight);
      markdownPreview.scrollTop = (value / editorRange) * previewRange;
    }
  }

  function recordPreviewScroll() {
    if (activeArticleId && markdownPreview) {
      previewScrollByArticle.set(activeArticleId, markdownPreview.scrollTop);
    }
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
    void platform.openMarkdownLink(url).catch((error) => onNotice(normalizeError(error).message));
  }

  function countWords(content: string) {
    const body = content.replace(/^---[\s\S]*?---/, "");
    const chinese = body.match(/[\u3400-\u9fff]/g)?.length ?? 0;
    const words = body.match(/[A-Za-z0-9_]+(?:[-'][A-Za-z0-9_]+)*/g)?.length ?? 0;
    return chinese + words;
  }

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, value));
  }

  async function refreshPreviewImages(force = false) {
    if (!session || !editorState.snapshot) return;
    const sources = extractPreviewImageSources(editorState.content);
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
          replacePreviewImageWithPlaceholder(image, result?.message ?? "图片加载失败、返回为空或内容无法显示。");
        }
        return;
      } catch (error) {
        if (image.isConnected) replacePreviewImageWithPlaceholder(image, normalizeError(error).message);
        return;
      }
    }
    replacePreviewImageWithPlaceholder(image, "图片加载失败、返回为空或内容无法显示。");
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
        coverErrors = { ...coverErrors, [originalSource]: result?.message ?? "图片加载失败或返回为空" };
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
    coverErrors = { ...coverErrors, [originalSource]: coverErrors[originalSource] ?? "图片加载失败，正在后台检查" };
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
      saving={editorState.saving}
      saveDisabled={!editorState.dirty || editorState.saving}
      imageDisabled={!editorState.snapshot}
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
              <EmptyState title={$ui("没有匹配的文章")} description={$ui("调整搜索或筛选条件，也可以新建一篇文章。")} icon={Search} />
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
                  {#if activeArticleId === article.articleId && editorState.dirty}<span class="dirty-dot" title={$ui("未保存")}></span>{/if}
                </button>
              {/each}
            {/if}
          </div>
        </aside>
        <main class="writing-pane">
          {#if loading}
            <LoadingState label={$ui("正在读取文章")} />
          {:else if loadError}
            <ErrorState message={failedArticle ? `无法读取“${failedArticle.title}”：${loadError}` : loadError}><button class="button" type="button" disabled={!failedArticle} on:click={() => failedArticle && openArticle(failedArticle)}>{$ui("重试")}</button>{#if editorState.snapshot}<button class="button" type="button" on:click={() => { loadError = ""; failedArticle = null; }}>{$ui("返回当前文章")}</button>{/if}</ErrorState>
          {:else if editorState.snapshot}
            <MarkdownEditor
              content={editorState.content}
              documentInstance={editorState.documentInstance}
              imageUrlReplacements={editorState.imageUrlReplacements}
              fontSize={config.editor.fontSize}
              lineHeight={config.editor.lineHeight}
              showLineNumbers={config.editor.showLineNumbers}
              lineWrapping={config.editor.lineWrapping}
              highlightLine={config.editor.highlightActiveLine}
              tabSize={config.editor.tabSize}
              selectionFrom={editorState.selection.from}
              selectionTo={editorState.selection.to}
              scrollTop={editorScrollTop}
              onChange={(content, changes) => store.update(content, changes)}
              onSelectionChange={(from, to) => store.setSelection(from, to)}
              onScroll={recordEditorScroll}
              onImageFiles={(files) => void handleImageFiles(files)}
              onSave={saveCurrent}
              onNewArticle={openCreateDialog}
            />
          {:else}
            <EmptyState title={$ui("选择一篇文章")} description={$ui("文章内容会在这里打开，右侧同步显示安全预览。")} />
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
              <span>{previewMode === "quick" ? (previewImagesPending ? $ui("正在读取图片") : $ui("HTML 已安全渲染")) : $ui("当前 Hexo 项目")}</span>
              <button
                class:active={previewScrollSync}
                class="icon-button small"
                type="button"
                aria-pressed={previewScrollSync}
                title={previewScrollSync ? $ui("关闭编辑器与预览同步滚动") : $ui("开启编辑器与预览同步滚动")}
                on:click={() => (previewScrollSync = !previewScrollSync)}
              >
                {#if previewScrollSync}<Link2 size={14} />{:else}<Unlink2 size={14} />{/if}
              </button>
            </div>
            {#if !editorState.snapshot}
              <EmptyState title={$ui("暂无预览")} description={$ui("打开文章后显示渲染结果。")} />
            {:else if previewMode === "theme"}
              <HexoThemePreview running={previewServer?.state === "running"} busy={previewBusy || themePreviewBusy} onOpen={() => void openThemePreview()} onReload={() => void openThemePreview()} />
            {:else}
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <article class="markdown-preview" bind:this={markdownPreview} on:scroll={recordPreviewScroll} on:click={handlePreviewInteraction} on:keydown={handlePreviewInteraction} on:error|capture={handlePreviewImageError}>{@html previewHtml}</article>
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
    <span class:error={Boolean(editorState.error)} title={editorState.error || undefined}>{editorState.error ? $ui("保存失败") : editorState.saving ? $ui("正在保存") : editorState.dirty ? $ui("有未保存更改") : editorState.snapshot ? $ui("已保存") : $ui("就绪")}</span>
    <span>{wordCount} {$ui("字")}</span>
    {#if editorState.snapshot}<span>{$ui("第")} {editorState.content.slice(0, editorState.selection.from).split("\n").length} {$ui("行")}</span>{/if}
    <span class="status-spacer"></span>
    {#if session}<button class={`status-sync ${syncStatus.status}`} type="button" disabled={syncBusy || syncStatus.status === "checking"} title={syncStatus.message || $ui("内容同步")} on:click={handleSyncStatusClick}><RefreshCw size={12} class={syncBusy || syncStatus.status === "checking" ? "spin" : undefined} />{syncBusy ? $ui("同步中") : syncStatus.status === "conflict" ? $ui("{p0} 个文件需处理", { p0: syncStatus.conflicts.length }) : $ui(syncStatusLabel(syncStatus))}</button>{/if}
    {#if session?.warnings.length}<span class="status-warning" title={session.warnings.join("；")}>{$ui("诊断")} {session.warnings.length} {$ui("项")}</span>{/if}
    {#if editorState.savedAt}<span>{$ui("最后保存")} {new Date(editorState.savedAt).toLocaleTimeString()}</span>{/if}
    {#if session}<span>{$ui("预览")} {previewStateLabel(previewServer?.state)}</span>{/if}
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
  <ModalDialog title={$ui("将文章移到回收站？")} description={$ui("“{p0}”将被移到系统回收站，可以从回收站恢复。文章的图片与同名资源目录会保留。{p1}", { p0: deletingArticle.title, p1: deletingArticle.articleId === activeArticleId && editorState.dirty ? " 当前未保存的修改也会丢失。" : "" })} onClose={() => !articleActionBusy && (deletingArticle = null)}>
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={articleActionBusy} on:click={() => (deletingArticle = null)}>{$ui("取消")}</button><button class="button danger" type="button" data-autofocus disabled={articleActionBusy} on:click={confirmDeleteArticle}>{articleActionBusy ? $ui("正在处理…") : $ui("移到回收站")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if showSwitchGuard}
  <ModalDialog title={$ui("保存当前文章？")} description={$ui("切换文章前需要处理未保存的内容。")} onClose={() => resolveSwitch("cancel")}>
    <svelte:fragment slot="actions">
      <button class="button" type="button" disabled={switchBusy} on:click={() => resolveSwitch("cancel")}>{$ui("取消")}</button>
      <button class="button danger" type="button" disabled={switchBusy || editorState.saving} on:click={() => resolveSwitch("discard")}>{$ui("放弃更改")}</button>
      <button class="button primary" type="button" data-autofocus disabled={switchBusy} on:click={() => resolveSwitch("save")}>{switchBusy ? $ui("处理中") : $ui("保存并继续")}</button>
    </svelte:fragment>
  </ModalDialog>
{/if}

{#if showCreate}
  <ModalDialog title={$ui("新建文章")} description={$ui("中文文件名会被保留，只过滤跨平台不安全的字符。日期使用本机时间。")} onClose={() => !creating && (showCreate = false)}>
    <div class="content-stack">
      <label class="field"><span>{$ui("标题")}</span><input class="input" bind:value={createTitle} data-autofocus placeholder={$ui("例如：我的第一篇文章")} /></label>
      <label class="field"><span>{$ui("文件名")}</span><input class="input" bind:value={createFileName} on:input={() => (createFileNameEdited = true)} placeholder={$ui("支持中文，无需手动填写 .md")} /></label>
      {#if createFileNameEdited}<button class="button quiet" type="button" on:click={() => (createFileNameEdited = false)}>{$ui("由标题生成文件名")}</button>{/if}
      <label class="field"><span>{$ui("类型")}</span><select class="select" bind:value={createKind}><option value="post">{$ui("文章")}</option><option value="draft">{$ui("草稿")}</option></select></label>
      <label class="field"><span>{$ui("日期")}</span><input class="input" type="datetime-local" bind:value={createDate} /></label>
      <label class="field"><span>{$ui("标签")}</span><input class="input" bind:value={createTags} placeholder={$ui("中文、逗号或回车分隔")} /></label>
      <label class="field"><span>{$ui("分类")}</span><input class="input" bind:value={createCategories} placeholder={$ui("中文、逗号或回车分隔")} /></label>
      {#if createError}<div class="badge warning" role="alert">{createError}</div>{/if}
    </div>
    <svelte:fragment slot="actions">
      <button class="button" type="button" disabled={creating} on:click={() => (showCreate = false)}>{$ui("取消")}</button>
      <button class="button primary" type="button" disabled={creating} on:click={createArticle}>{creating ? $ui("创建中") : $ui("创建并打开")}</button>
    </svelte:fragment>
  </ModalDialog>
{/if}
