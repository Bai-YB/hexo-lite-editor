<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import {
    Eye,
    EyeOff,
    ChevronDown,
    FilePlus2,
    FolderOpen,
    ImagePlus,
    Rocket,
    Save,
    Search,
    Server,
    SlidersHorizontal,
    MoreHorizontal,
    RefreshCw
  } from "@lucide/svelte";
  import MarkdownEditor from "./MarkdownEditor.svelte";
  import type { EditorSessionStore, EditorSessionState } from "./EditorSessionStore";
  import EmptyState from "$shared/components/EmptyState.svelte";
  import ErrorState from "$shared/components/ErrorState.svelte";
  import LoadingState from "$shared/components/LoadingState.svelte";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import { extractRemoteImageUrls, renderSafeMarkdown } from "$shared/markdown/safeMarkdown";
  import { platform, normalizeError } from "$platform/tauri";
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
  export let autoSaveSuspended = false;

  const store = editorStore;
  let editorState: EditorSessionState = store.getState();
  let activeArticleId: string | null = store.activeArticleId();
  let query = "";
  let filter: "all" | ArticleKind = "all";
  let category = "";
  let tag = "";
  let sortMode: "modifiedDesc" | "createdDesc" | "dateDesc" | "titleAsc" = "modifiedDesc";
  let filterMenuOpen = false;
  let loading = false;
  let loadError = "";
  let previewHtml = "";
  let previewAssets: Record<string, string> = {};
  let autoSaveTimer: ReturnType<typeof setTimeout> | undefined;
  let lastProjectId: string | null = store.getState().snapshot?.projectId ?? null;
  let pendingArticle: ArticleSummary | null = null;
  let showSwitchGuard = false;
  let showCreate = false;
  let createTitle = "";
  let createFileName = "";
  let createKind: ArticleKind = "post";
  let createDate = new Date().toISOString().slice(0, 16);
  let createTags = "";
  let createCategories = "";
  let createError = "";
  let creating = false;
  let articleWidth = config.layout.articleListWidth;
  let previewWidth = config.layout.previewWidth;
  let imageInput: HTMLInputElement;
  let projectMenuOpen = false;
  let advancedMenuOpen = false;
  let remotePreviewAssets: Record<string, string | null> = {};
  let remotePreviewPending = false;
  let remoteImageUrls: string[] = [];
  let remoteImageKey = "";
  let lastValidatedImageKey = "";
  let imageValidationSequence = 0;
  let editorScrollTop = 0;
  let markdownPreview: HTMLElement;
  const editorScrollByArticle = new Map<string, number>();
  const previewScrollByArticle = new Map<string, number>();
  let assetRefreshAttempted = false;

  const unsubscribe = store.subscribe((state) => {
    editorState = state;
  });

  onMount(() => {
    window.addEventListener("pointerdown", closeEditorMenus);
    window.addEventListener("keydown", closeEditorMenus);
    window.addEventListener("focus", handleWindowFocus);
    if (!session) return;
    void loadPreviewAssets(session);
    if (!store.getState().snapshot && articles.length) void openArticle(articles[0]);
  });

  onDestroy(() => {
    unsubscribe();
    clearTimeout(autoSaveTimer);
    window.removeEventListener("pointerdown", closeEditorMenus);
    window.removeEventListener("keydown", closeEditorMenus);
    window.removeEventListener("focus", handleWindowFocus);
  });

  function closeEditorMenus(event: Event) {
    if (event instanceof KeyboardEvent && event.key !== "Escape") return;
    const target = event.target as HTMLElement;
    if (!(event instanceof KeyboardEvent) && target.closest?.(".project-switcher-wrap, .advanced-menu-wrap")) return;
    projectMenuOpen = false;
    advancedMenuOpen = false;
  }

  $: remoteImageUrls = extractRemoteImageUrls(editorState.content, previewAssets);
  $: remoteImageKey = `${activeArticleId ?? ""}\u0000${remoteImageUrls.join("\u0000")}`;
  $: if (session && editorState.snapshot && remoteImageKey !== lastValidatedImageKey) {
    lastValidatedImageKey = remoteImageKey;
    void refreshRemotePreviewImages();
  }
  $: previewHtml = renderSafeMarkdown(
    editorState.content,
    previewAssets,
    remotePreviewAssets,
    remotePreviewPending
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

  $: if (session?.projectId !== lastProjectId) {
    lastProjectId = session?.projectId ?? null;
    activeArticleId = null;
    store.clear();
    previewAssets = {};
    remotePreviewAssets = {};
    lastValidatedImageKey = "";
    if (session) {
      void loadPreviewAssets(session);
      if (articles.length) void openArticle(articles[0]);
    }
  }

  $: {
    clearTimeout(autoSaveTimer);
    if (!autoSaveSuspended && config.general.autoSave && editorState.dirty && !editorState.saving) {
      autoSaveTimer = setTimeout(() => void saveCurrent(), config.general.autoSaveDelayMs);
    }
  }

  async function openArticle(article: ArticleSummary) {
    if (!session) return;
    loading = true;
    loadError = "";
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
        expectedId !== snapshot.articleId
        || session?.projectId !== expectedProjectId
        || session.generation !== expectedGeneration
        || snapshot.projectId !== expectedProjectId
        || snapshot.sessionGeneration !== expectedGeneration
      ) return;
      activeArticleId = article.articleId;
      editorScrollTop = editorScrollByArticle.get(article.articleId) ?? 0;
      store.load(snapshot);
      remotePreviewAssets = {};
      lastValidatedImageKey = "";
      requestAnimationFrame(() => {
        if (markdownPreview) markdownPreview.scrollTop = previewScrollByArticle.get(article.articleId) ?? 0;
      });
    } catch (error) {
      loadError = normalizeError(error).message;
    } finally {
      loading = false;
    }
  }

  async function loadPreviewAssets(project: ProjectSessionView) {
    try {
      const localImages = await platform.listLocalImages(project.projectId, project.generation);
      const next: Record<string, string> = {};
      for (const image of localImages) {
        const relative = image.relativePath.replace(/\\/g, "/");
        const imageRelative = relative.replace(/^source\//, "");
        const markdownUrl = image.markdownUrl.replace(/\\/g, "/");
        next[relative] = image.previewUrl;
        next[`/${relative}`] = image.previewUrl;
        next[imageRelative] = image.previewUrl;
        next[`/${imageRelative}`] = image.previewUrl;
        next[markdownUrl] = image.previewUrl;
        try {
          next[decodeURI(markdownUrl)] = image.previewUrl;
        } catch {
          // The backend emits encoded URLs; keep the raw key if an old record is malformed.
        }
      }
      if (session?.projectId === project.projectId) previewAssets = next;
    } catch {
      previewAssets = {};
    }
  }

  function requestArticle(article: ArticleSummary) {
    if (article.articleId === activeArticleId) return;
    if (editorState.dirty) {
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
    if (action === "cancel") {
      pendingArticle = null;
      showSwitchGuard = false;
      return;
    }
    try {
      if (action === "save") await saveAndRefresh();
      else store.discard();
      const next = pendingArticle;
      pendingArticle = null;
      showSwitchGuard = false;
      if (next) await openArticle(next);
    } catch (error) {
      onNotice(normalizeError(error).message);
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
    await store.save();
    if (!session) return;
    const next = await platform.listArticles(session.projectId, session.generation);
    articles = next;
    onArticlesChange(next);
  }

  function openCreateDialog() {
    if (!session) return;
    createTitle = "";
    createFileName = "";
    createKind = "post";
    createDate = new Date().toISOString().slice(0, 16);
    createTags = "";
    createCategories = "";
    createError = "";
    showCreate = true;
  }

  $: if (createTitle && !createFileName) {
    createFileName = createTitle.trim().replace(/[<>:"/\\|?*]/g, "-");
  }

  async function createArticle() {
    if (!session || !createTitle.trim() || !createFileName.trim()) {
      createError = "请填写标题和文件名。";
      return;
    }
    creating = true;
    createError = "";
    try {
      if (editorState.dirty) await saveAndRefresh();
      const summary = await platform.createArticle({
        projectId: session.projectId,
        sessionGeneration: session.generation,
        title: createTitle.trim(),
        fileName: createFileName.trim(),
        kind: createKind,
        date: createDate.replace("T", " "),
        tags: splitLabels(createTags),
        categories: splitLabels(createCategories)
      });
      const next = [summary, ...articles];
      articles = next;
      onArticlesChange(next);
      showCreate = false;
      await openArticle(summary);
    } catch (error) {
      createError = normalizeError(error).message;
    } finally {
      creating = false;
    }
  }

  function splitLabels(value: string) {
    return [...new Set(value.split(/[,，\n]+/).map((item) => item.trim()).filter(Boolean))];
  }

  async function handleImageFiles(files: File[]) {
    if (!session || !files.length) return;
    const ordered = [];
    for (const file of files) {
      ordered.push({
        name: file.name || `image-${Date.now()}.png`,
        mime: file.type,
        bytes: Array.from(new Uint8Array(await file.arrayBuffer()))
      });
    }
    try {
      const results = await platform.importEditorImages(
        session.projectId,
        session.generation,
        config.imageBed.defaultProvider,
        ordered
      );
      const successes = results.filter((result) => result.markdown);
      if (config.imageBed.autoInsertMarkdown) {
        for (const result of successes) store.insertMarkdown(result.markdown!);
      }
      const failed = results.filter((result) => result.error);
      if (failed.length) onNotice(`${successes.length} 张图片已处理，${failed.length} 张失败：${failed[0].error?.message}`);
      else onNotice(`${successes.length} 张图片已处理${config.imageBed.autoInsertMarkdown ? "并插入文章" : ""}。`);
      if (config.imageBed.defaultProvider === "local") void loadPreviewAssets(session);
      setTimeout(() => void refreshRemotePreviewImages(true), 0);
    } catch (error) {
      onNotice(normalizeError(error).message);
    }
  }

  function togglePreview() {
    const next = {
      ...config,
      layout: { ...config.layout, previewVisible: !config.layout.previewVisible }
    };
    onConfigChange(next);
  }

  function handleWindowFocus() {
    if (editorState.snapshot) void refreshRemotePreviewImages(true);
  }

  async function refreshRemotePreviewImages(force = false) {
    if (!session || !editorState.snapshot) return;
    const urls = extractRemoteImageUrls(editorState.content, previewAssets);
    const validationKey = `${activeArticleId ?? ""}\u0000${urls.join("\u0000")}`;
    if (!force && validationKey !== remoteImageKey) return;
    const sequence = ++imageValidationSequence;
    const expectedArticleId = activeArticleId;
    const expectedProjectId = session.projectId;
    const expectedGeneration = session.generation;
    remotePreviewPending = urls.length > 0;
    remotePreviewAssets = Object.fromEntries(urls.map((url) => [url, null]));
    if (!urls.length) return;
    try {
      const results = await platform.resolveRemotePreviewImages({
        projectId: expectedProjectId,
        sessionGeneration: expectedGeneration,
        urls
      });
      if (
        sequence !== imageValidationSequence
        || activeArticleId !== expectedArticleId
        || session?.projectId !== expectedProjectId
        || session.generation !== expectedGeneration
      ) return;
      remotePreviewAssets = Object.fromEntries(
        results.map((result) => [
          result.originalUrl,
          result.state === "ready" ? result.previewUrl ?? null : null
        ])
      );
    } catch {
      if (sequence === imageValidationSequence) remotePreviewAssets = Object.fromEntries(urls.map((url) => [url, null]));
    } finally {
      if (sequence === imageValidationSequence) remotePreviewPending = false;
    }
  }

  function handlePreviewImageError(event: Event) {
    if (event.target instanceof HTMLImageElement) void refreshRemotePreviewImages(true);
  }

  async function runAdvanced(kind: TaskType) {
    if (!session) return;
    advancedMenuOpen = false;
    try {
      await platform.startTask(session.projectId, kind);
      onNotice(`${kind === "gitStatus" ? "Git 检查" : "任务"}已在后台开始。`);
    } catch (error) {
      onNotice(normalizeError(error).message);
    }
  }

  function recordEditorScroll(value: number) {
    editorScrollTop = value;
    if (activeArticleId) editorScrollByArticle.set(activeArticleId, value);
  }

  function recordPreviewScroll() {
    if (activeArticleId && markdownPreview) {
      previewScrollByArticle.set(activeArticleId, markdownPreview.scrollTop);
    }
  }

  async function refreshExpiredAssets() {
    if (!session || assetRefreshAttempted) return;
    assetRefreshAttempted = true;
    try {
      const next = await platform.listArticles(session.projectId, session.generation);
      articles = next;
      onArticlesChange(next);
      await loadPreviewAssets(session);
    } catch {
      // A broken thumbnail remains a quiet placeholder; editing is unaffected.
    }
  }

  function startResize(event: PointerEvent, target: "articles" | "preview") {
    const startX = event.clientX;
    const startWidth = target === "articles" ? articleWidth : previewWidth;
    const move = (moveEvent: PointerEvent) => {
      const delta = moveEvent.clientX - startX;
      if (target === "articles") articleWidth = clamp(startWidth + delta, 220, 420);
      else previewWidth = clamp(startWidth - delta, 280, 720);
    };
    const up = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      onConfigChange({
        ...config,
        layout: { ...config.layout, articleListWidth: articleWidth, previewWidth }
      });
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up, { once: true });
  }

  function handlePreviewInteraction(event: MouseEvent | KeyboardEvent) {
    if (event instanceof KeyboardEvent && event.key !== "Enter" && event.key !== " ") return;
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
</script>

<div class="editor-page">
  <header class="editor-toolbar">
    <div class="project-switcher-wrap">
      <button class="project-switcher" type="button" aria-expanded={projectMenuOpen} on:click={() => { projectMenuOpen = !projectMenuOpen; advancedMenuOpen = false; }}>
        <FolderOpen size={17} />
        <span>{session?.name ?? "打开博客"}</span>
        <ChevronDown size={14} />
      </button>
      {#if projectMenuOpen}
        <div class="project-menu quiet-menu">
          {#if session}<div class="project-menu-current"><strong>{session.name}</strong><span>{session.displayPath}</span></div>{/if}
          {#each recentProjects.slice(0, 10) as recent (recent.recentId)}
            <button type="button" disabled={!recent.available} on:click={() => { projectMenuOpen = false; onOpenRecentProject(recent.recentId); }}><span>{recent.name}</span><small>{recent.available ? recent.displayPath : "位置不可用"}</small></button>
          {/each}
          <button class="project-menu-open" type="button" on:click={() => { projectMenuOpen = false; onOpenProject(); }}><FolderOpen size={15} /><span>打开其他博客</span></button>
        </div>
      {/if}
    </div>
    <div class="toolbar-spacer"></div>
    {#if session}
      <button class="button quiet" type="button" on:click={() => void onPreview(true)}><Server size={16} />浏览器预览</button>
      <button class="button quiet" type="button" on:click={openCreateDialog}><FilePlus2 size={16} />新建</button>
      <button class="icon-button" type="button" disabled={!editorState.snapshot} title="选择图片并插入" aria-label="选择图片并插入" on:click={() => imageInput?.click()}><ImagePlus size={17} /></button>
      <button class="button quiet" type="button" disabled={!editorState.dirty || editorState.saving} on:click={saveCurrent}><Save size={16} />{editorState.saving ? "保存中" : "保存"}</button>
      <button class="icon-button" type="button" title={config.layout.previewVisible ? "隐藏预览" : "显示预览"} aria-label={config.layout.previewVisible ? "隐藏预览" : "显示预览"} on:click={togglePreview}>{#if config.layout.previewVisible}<EyeOff size={16} />{:else}<Eye size={16} />{/if}</button>
      <div class="advanced-menu-wrap">
        <button class="icon-button" type="button" title="高级操作" aria-label="高级操作" aria-expanded={advancedMenuOpen} on:click={() => { advancedMenuOpen = !advancedMenuOpen; projectMenuOpen = false; }}><MoreHorizontal size={18} /></button>
        {#if advancedMenuOpen}
          <div class="advanced-menu quiet-menu">
            <button type="button" on:click={() => void runAdvanced("clean")}>清理缓存</button>
            <button type="button" on:click={() => void runAdvanced("generate")}>生成站点</button>
            <button type="button" on:click={() => void runAdvanced("deploy")}>单独部署</button>
            <button type="button" on:click={() => void runAdvanced("gitStatus")}>检查 Git 状态</button>
            <div class="menu-separator"></div>
            <button type="button" on:click={() => { advancedMenuOpen = false; onTogglePreviewServer(); }}>{previewServer?.state === "running" ? "停止本地预览" : "启动本地预览"}</button>
            <button type="button" on:click={() => { advancedMenuOpen = false; onOpenPreviewHome(); }}>打开博客首页</button>
            <button type="button" on:click={() => { advancedMenuOpen = false; onOpenSettings("maintenance"); }}>诊断与日志</button>
          </div>
        {/if}
      </div>
      <button class="button primary" type="button" disabled={taskBusy} title="发布（Ctrl+Shift+P）" on:click={onPublish}><Rocket size={16} />{taskBusy ? "处理中" : "发布"}</button>
    {/if}
  </header>
  <input hidden bind:this={imageInput} type="file" accept="image/png,image/jpeg,image/gif,image/webp" multiple on:change={(event) => { void handleImageFiles(Array.from(event.currentTarget.files ?? [])); event.currentTarget.value = ""; }} />

  {#if !session}
    <div class="editor-welcome">
      <EmptyState title="还没有打开项目" description="选择包含 _config.yml、package.json 与 source/_posts 的 Hexo 根目录。">
        <button class="button primary" type="button" on:click={onOpenProject}><FolderOpen size={16} />选择项目文件夹</button>
      </EmptyState>
      {#if recentProjects.length}
        <section class="welcome-recents" aria-label="最近项目"><h2>最近项目</h2>{#each recentProjects.slice(0, 5) as recent (recent.recentId)}<button type="button" disabled={!recent.available} on:click={() => onOpenRecentProject(recent.recentId)}><span><strong>{recent.name}</strong><small>{recent.displayPath}</small></span><small>{recent.available ? "打开" : "位置不可用"}</small></button>{/each}</section>
      {/if}
    </div>
  {:else}
    <div class="editor-grid-wrap" style={`--article-width:${articleWidth}px; --preview-width:${previewWidth}px`}>
      <div class="editor-grid">
        <aside class="article-pane" aria-label="文章列表">
          <div class="pane-toolbar">
            <Search size={15} aria-hidden="true" />
            <input class="input" bind:value={query} aria-label="搜索文章" placeholder="搜索标题或路径" />
          </div>
          <div class="filter-row" aria-label="文章筛选">
            <button class:active={filter === "all"} class="filter-chip" type="button" on:click={() => (filter = "all")}>全部</button>
            <button class:active={filter === "post"} class="filter-chip" type="button" on:click={() => (filter = "post")}>文章</button>
            <button class:active={filter === "draft"} class="filter-chip" type="button" on:click={() => (filter = "draft")}>草稿</button>
            <span class="filter-spacer"></span>
            <button class:active={Boolean(category || tag || sortMode !== "modifiedDesc")} class="filter-menu-button" type="button" aria-label="筛选与排序" aria-expanded={filterMenuOpen} on:click={() => (filterMenuOpen = !filterMenuOpen)}><SlidersHorizontal size={15} /></button>
          </div>
          {#if filterMenuOpen}
            <div class="article-filter-popover">
              <label><span>分类</span><select class="select" bind:value={category}><option value="">全部分类</option>{#each availableCategories as item}<option value={item}>{item}</option>{/each}</select></label>
              <label><span>标签</span><select class="select" bind:value={tag}><option value="">全部标签</option>{#each availableTags as item}<option value={item}>{item}</option>{/each}</select></label>
              <label><span>排序</span><select class="select" bind:value={sortMode}><option value="modifiedDesc">最近修改</option><option value="createdDesc">创建时间</option><option value="dateDesc">文章日期</option><option value="titleAsc">标题 A–Z</option></select></label>
              <div class="filter-popover-actions"><button class="button" type="button" on:click={() => { category = ""; tag = ""; sortMode = "modifiedDesc"; }}>重置</button><button class="button primary" type="button" on:click={() => (filterMenuOpen = false)}>完成</button></div>
            </div>
          {/if}
          <div class="article-list">
            {#if !filteredArticles.length}
              <EmptyState title="没有匹配的文章" description="调整搜索或筛选条件，也可以新建一篇文章。" icon={Search} />
            {:else}
              {#each filteredArticles as article (article.articleId)}
                <button
                  class:active={activeArticleId === article.articleId}
                  class="article-item"
                  type="button"
                  data-article-id={article.articleId}
                  on:click={() => requestArticle(article)}
                  on:keydown={(event) => handleArticleKeydown(event, article)}
                >
                  {#if config.articleList.showCover}
                    {#if article.cover.previewUrl}<img class="article-cover" src={article.cover.previewUrl} alt={article.cover.alt} on:error={refreshExpiredAssets} />{:else}<span class="article-cover placeholder" aria-hidden="true">{article.title.slice(0, 1)}</span>{/if}
                  {/if}
                  <span class="article-copy"><span class="article-title">{article.title}</span><span class="article-meta">{article.kind === "draft" ? "草稿" : "文章"} · {new Date(article.modifiedAt).toLocaleDateString()}</span></span>
                  {#if activeArticleId === article.articleId && editorState.dirty}<span class="dirty-dot" title="未保存"></span>{/if}
                </button>
              {/each}
            {/if}
          </div>
        </aside>
        <main class="writing-pane">
          {#if loading}
            <LoadingState label="正在读取文章" />
          {:else if loadError}
            <ErrorState message={loadError}><button class="button" type="button" on:click={() => activeArticleId && openArticle(articles.find((item) => item.articleId === activeArticleId)!)}>重试</button></ErrorState>
          {:else if editorState.snapshot}
            <MarkdownEditor
              content={editorState.content}
              fontSize={config.editor.fontSize}
              lineHeight={config.editor.lineHeight}
              showLineNumbers={config.editor.showLineNumbers}
              lineWrapping={config.editor.lineWrapping}
              highlightLine={config.editor.highlightActiveLine}
              tabSize={config.editor.tabSize}
              selectionFrom={editorState.selection.from}
              selectionTo={editorState.selection.to}
              scrollTop={editorScrollTop}
              onChange={(content) => store.update(content)}
              onSelectionChange={(from, to) => store.setSelection(from, to)}
              onScroll={recordEditorScroll}
              onImageFiles={(files) => void handleImageFiles(files)}
              onSave={saveCurrent}
              onNewArticle={openCreateDialog}
            />
          {:else}
            <EmptyState title="选择一篇文章" description="文章内容会在这里打开，右侧同步显示安全预览。" />
          {/if}
        </main>
        {#if config.layout.previewVisible}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <section class="preview-pane" aria-label="文章预览">
            <div class="preview-mode-bar">
              <strong>即时预览</strong>
              <span>{remotePreviewPending ? "正在验证远程图片" : "HTML 已安全渲染"}</span>
              <button class="icon-button small" type="button" title="重新验证远程图片" aria-label="刷新图片" disabled={remotePreviewPending} on:click={() => void refreshRemotePreviewImages(true)}><RefreshCw size={14} /></button>
            </div>
            {#if !editorState.snapshot}
              <EmptyState title="暂无预览" description="打开文章后显示渲染结果。" />
            {:else}
              <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
              <article class="markdown-preview" bind:this={markdownPreview} on:scroll={recordPreviewScroll} on:click={handlePreviewInteraction} on:keydown={handlePreviewInteraction} on:error|capture={handlePreviewImageError}>{@html previewHtml}</article>
            {/if}
          </section>
        {/if}
      </div>
      <div class="resize-handle" style={`left:${articleWidth}px`} role="separator" aria-label="调整文章列表宽度" on:pointerdown={(event) => startResize(event, "articles")}></div>
      {#if config.layout.previewVisible}
        <div class="resize-handle" style={`right:${previewWidth}px`} role="separator" aria-label="调整预览宽度" on:pointerdown={(event) => startResize(event, "preview")}></div>
      {/if}
    </div>
  {/if}

  <footer class="editor-status">
    <span>{editorState.error ? "保存失败" : editorState.saving ? "正在保存" : editorState.dirty ? "有未保存更改" : editorState.snapshot ? "已保存" : "就绪"}</span>
    <span>{wordCount} 字</span>
    <span class="status-spacer"></span>
    {#if editorState.savedAt}<span>最后保存 {new Date(editorState.savedAt).toLocaleTimeString()}</span>{/if}
    {#if session}<span>预览 {previewStateLabel(previewServer?.state)}</span>{/if}
    {#if activeArticleId}<span>revision {editorState.revision}</span>{/if}
  </footer>
</div>

{#if showSwitchGuard}
  <ModalDialog title="保存当前文章？" description="切换文章前需要处理未保存的内容。" onClose={() => resolveSwitch("cancel")}>
    <svelte:fragment slot="actions">
      <button class="button" type="button" on:click={() => resolveSwitch("cancel")}>取消</button>
      <button class="button danger" type="button" on:click={() => resolveSwitch("discard")}>放弃更改</button>
      <button class="button primary" type="button" data-autofocus on:click={() => resolveSwitch("save")}>保存并继续</button>
    </svelte:fragment>
  </ModalDialog>
{/if}

{#if showCreate}
  <ModalDialog title="新建文章" description="中文文件名会被保留，只过滤 Windows 不允许的字符。" onClose={() => (showCreate = false)}>
    <div class="content-stack">
      <label class="field"><span>标题</span><input class="input" bind:value={createTitle} data-autofocus placeholder="例如：我的第一篇文章" /></label>
      <label class="field"><span>文件名</span><input class="input" bind:value={createFileName} placeholder="支持中文，无需手动填写 .md" /></label>
      <label class="field"><span>类型</span><select class="select" bind:value={createKind}><option value="post">文章</option><option value="draft">草稿</option></select></label>
      <label class="field"><span>日期</span><input class="input" type="datetime-local" bind:value={createDate} /></label>
      <label class="field"><span>标签</span><input class="input" bind:value={createTags} placeholder="中文、逗号或回车分隔" /></label>
      <label class="field"><span>分类</span><input class="input" bind:value={createCategories} placeholder="中文、逗号或回车分隔" /></label>
      {#if createError}<div class="badge warning" role="alert">{createError}</div>{/if}
    </div>
    <svelte:fragment slot="actions">
      <button class="button" type="button" on:click={() => (showCreate = false)}>取消</button>
      <button class="button primary" type="button" disabled={creating} on:click={createArticle}>{creating ? "创建中" : "创建并打开"}</button>
    </svelte:fragment>
  </ModalDialog>
{/if}
