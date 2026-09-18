<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { onDestroy, onMount, tick } from "svelte";
  import { mergeSavedDraft, validateSettings } from "./settingsDraft";
  import { KeyRound, RotateCcw, Trash2 } from "@lucide/svelte";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import SettingsHeader from "./SettingsHeader.svelte";
  import SettingsNavigation from "./SettingsNavigation.svelte";
  import SyncProviderPicker from "./SyncProviderPicker.svelte";
  import CloudflareImageBedSettings from "./CloudflareImageBedSettings.svelte";
  import { syncStatusLabel } from "$features/editor/syncStatusLabel";
  import { defaultConfig } from "$shared/types/app";
  import { normalizeError, platform } from "$platform/tauri";
  import { shortcutLabel } from "$platform/os";
  import { setLanguage, translate } from "$shared/i18n";
  import type {
    AppConfigV3,
    ContentSyncConflict,
    ContentSyncPreflight,
    ContentSyncProvider,
    CredentialStatus,
    RecentProjectView,
    SettingsSectionId,
    ThemeMode,
    WebDavContentSyncPreflight
  } from "$shared/types/app";

  export let config: AppConfigV3;
  export let session: import("$shared/types/app").ProjectSessionView | null = null;
  export let initialSection: SettingsSectionId | null = null;
  export let recentProjects: RecentProjectView[] = [];
  export let onSaveConfig: (config: AppConfigV3) => Promise<AppConfigV3> = async (value) => value;
  export let onThemePreview: (mode: ThemeMode) => void = () => {};
  export let onRemoveRecentProject: (recentId: string) => Promise<void> = async () => {};
  export let onClearRecentProjects: () => Promise<void> = async () => {};
  export let onBeforeSync: () => Promise<boolean> = async () => true;
  export let onPublish: () => Promise<void> = async () => {};
  export let onOpenUpdates: () => void = () => {};
  export let taskBusy = false;
  export let onNotice: (message: string, severity?: "info" | "error") => void = () => {};

  const sectionStorageKey = "hexo-lite-editor:settings-active-section";
  const sections: Array<{ id: SettingsSectionId; title: string; description: string }> = [
    { id: "general", title: "常规", description: "语言、启动和保存" },
    { id: "editing", title: "编辑器", description: "外观和写作习惯" },
    { id: "images", title: "图片", description: "保存位置和图床连接" },
    { id: "hexoPublish", title: "预览与发布", description: "预览网站、发布更新" },
    { id: "sync", title: "文件同步", description: "跨设备同步站点源码" },
    { id: "maintenance", title: "更新与恢复", description: "应用更新和恢复默认" }
  ];

  let saved = structuredClone(config);
  let draft = structuredClone(config);
  let activeSection: SettingsSectionId = "general";
  let dirty = false;
  let saving = false;
  let pendingSave: Promise<void> | null = null;
  let persistTimer: ReturnType<typeof setTimeout> | undefined;
  let saveError = "";
  let pageElement: HTMLDivElement;
  let layoutObserver: ResizeObserver | undefined;
  let disposed = false;
  let credentialRequest = 0;
  let syncRevision = 0;
  let syncLoaded = false;
  let syncLoading = false;
  let unlistenSync: (() => void) | null = null;
  let unlistenSyncPhase: (() => void) | null = null;
  let syncProgress: import("$shared/types/app").ContentSyncEvent | null = null;
  let syncStopping = false;
  let backgroundSyncBusy = false;
  let syncStartedAt = 0;
  let syncElapsed = 0;
  let syncTimer: ReturnType<typeof setInterval> | null = null;
  let syncError = "";
  let credential: CredentialStatus = { configured: false };
  let legacyCredentialAvailable = false;
  let credentialBusy = false;
  let tokenStatusMessage = "";
  let showAcquireToken = false;
  let adminUsername = "";
  let adminPassword = "";
  let showReset = false;
  let showClearRecent = false;
  let pendingSyncOverwrite: "overwriteLocal" | "overwriteRemote" | null = null;
  let scopeAcknowledged = false;
  let syncSummary: import("$shared/types/app").ContentSyncSummary | null = null;
  let summaryBusy = false;
  let summaryError = "";
  let summaryRequest = 0;
  let syncCandidate: import("$shared/types/app").ContentSyncCandidate | null = null;
  let syncCandidates: import("$shared/types/app").ContentSyncCandidate[] = [];
  let syncPreflight: ContentSyncPreflight | null = null;
  let syncConflicts: ContentSyncConflict[] = [];
  let conflictChoices: Record<string, "local" | "remote"> = {};
  let syncStatus: import("$shared/types/app").ContentSyncView = { enabled: false, status: "off", provider: "github", conflicts: [] };
  let syncBusy = false;
  let syncProvider: ContentSyncProvider = "github";
  let syncBranch = "hexo-lite-content";
  let publicAcknowledged = false;
  let webDavEndpoint = "";
  let webDavRemoteDir = "hexo-lite-content";
  let webDavUsername = "";
  let webDavPassword = "";
  let webDavCredential: CredentialStatus = { configured: false };
  let webDavPreflight: WebDavContentSyncPreflight | null = null;
  let webDavTestedAt = "";
  let webDavTestedEndpoint = "";
  let webDavTestedRemoteDir = "";
  let webDavConnectionError = "";
  let webDavConnectionOpen = false;
  let switchingProvider: ContentSyncProvider | null = null;

  $: dirty = JSON.stringify(draft) !== JSON.stringify(saved);
  $: currentSection = sections.find((section) => section.id === activeSection) ?? sections[0];
  $: webDavConnectionDirty = syncStatus.enabled && syncStatus.provider === "webdav"
    && (webDavEndpoint.trim().replace(/\/$/, "") !== (syncStatus.endpoint ?? "").replace(/\/$/, "")
      || webDavRemoteDir.trim().replace(/^\/+|\/+$/g, "") !== (syncStatus.remoteDir ?? ""));
  $: webDavTestMatches = Boolean(webDavPreflight)
    && webDavEndpoint.trim().replace(/\/$/, "") === webDavTestedEndpoint
    && webDavRemoteDir.trim().replace(/^\/+|\/+$/g, "") === webDavTestedRemoteDir;
  $: if (syncStatus.status === "authRequired" && syncStatus.provider === "webdav") webDavConnectionOpen = true;
  $: setupProvider = switchingProvider ?? syncProvider;
  $: if (syncStatus.enabled && switchingProvider === syncStatus.provider) switchingProvider = null;
  $: dirtySections = {
    general: JSON.stringify(draft.general) !== JSON.stringify(saved.general),
    editing: JSON.stringify([draft.appearance, draft.editor, draft.articleList]) !== JSON.stringify([saved.appearance, saved.editor, saved.articleList]),
    images: JSON.stringify(draft.imageBed) !== JSON.stringify(saved.imageBed),
    hexoPublish: JSON.stringify([draft.hexo, draft.publish]) !== JSON.stringify([saved.hexo, saved.publish]),
    sync: false,
    maintenance: JSON.stringify([draft.diagnostics, draft.update]) !== JSON.stringify([saved.diagnostics, saved.update])
  } satisfies Record<SettingsSectionId, boolean>;

  onMount(() => {
    const header = pageElement.querySelector<HTMLElement>(".settings-sticky-header");
    const nav = pageElement.querySelector<HTMLElement>(".settings-nav");
    const measureLayout = () => {
      if (header) pageElement.style.setProperty("--settings-header-height", `${header.getBoundingClientRect().height}px`);
      if (nav) pageElement.style.setProperty("--settings-nav-height", `${nav.getBoundingClientRect().height}px`);
    };
    if (typeof ResizeObserver !== "undefined") {
      layoutObserver = new ResizeObserver(measureLayout);
      if (header) layoutObserver.observe(header);
      if (nav) layoutObserver.observe(nav);
      measureLayout();
    }
    const stored = localStorage.getItem(sectionStorageKey) as SettingsSectionId | null;
    activeSection = initialSection ?? (sections.some((section) => section.id === stored) ? stored! : "general");
    void refreshCredential();
    void platform.onContentSyncStatus((status) => {
      if (!session || disposed || status.projectId !== session.projectId || status.sessionGeneration !== session.generation) return;
      syncRevision += 1;
      syncStatus = status;
      if (!syncBusy && activeSection === "sync") {
        void refreshSyncConflicts().catch((error) => { if (!disposed) syncError = normalizeError(error).message; });
        void refreshSyncSummary();
      }
    }).then((unlisten) => { if (disposed) unlisten(); else unlistenSync = unlisten; })
      .catch((error) => { if (!disposed) onNotice(normalizeError(error).message, "error"); });
    void platform.onContentSyncPhase((event) => {
      if (!session || disposed || event.projectId !== session.projectId || event.sessionGeneration !== session.generation) return;
      if (["completed", "operationFinished", "failed", "attention", "waiting"].includes(event.phase)) {
        if (backgroundSyncBusy) {
          backgroundSyncBusy = false; endSync();
          if (syncStatus.status === "conflict") void refreshSyncConflicts().catch((error) => { if (!disposed) syncError = normalizeError(error).message; });
          if (activeSection === "sync") void refreshSyncSummary();
        }
        syncProgress = null; return;
      }
      if (!syncBusy) { beginSync(); backgroundSyncBusy = true; }
      syncProgress = event;
    }).then((unlisten) => {
      if (disposed) unlisten();
      else {
        unlistenSyncPhase = unlisten;
        if (activeSection === "sync" && !syncLoaded) void refreshSync();
      }
    })
      .catch((error) => { if (!disposed) syncError = normalizeError(error).message; });
    syncTimer = setInterval(() => { if (syncBusy) syncElapsed = Math.floor((Date.now() - syncStartedAt) / 1000); }, 1000);
  });

  onDestroy(() => {
    if (persistTimer) { clearTimeout(persistTimer); persistTimer = undefined; }
    void flushDraft();
    disposed = true;
    layoutObserver?.disconnect();
    unlistenSync?.();
    unlistenSyncPhase?.();
    if (syncTimer) clearInterval(syncTimer);
  });

  function selectSection(section: SettingsSectionId) {
    activeSection = section;
    localStorage.setItem(sectionStorageKey, section);
    if (section === "sync" && !syncLoaded && !syncLoading) void refreshSync();
    void tick().then(() => {
      if (disposed || !pageElement) return;
      pageElement.scrollTop = 0;
      const nav = pageElement.querySelector<HTMLElement>(".settings-nav");
      const button = nav?.querySelector<HTMLButtonElement>(`[data-settings-section="${section}"]`);
      if (nav && button && nav.scrollWidth > nav.clientWidth) {
        const left = button.getBoundingClientRect().left - nav.getBoundingClientRect().left + nav.scrollLeft;
        nav.scrollLeft = Math.max(0, Math.min(nav.scrollLeft, left), left + button.offsetWidth - nav.clientWidth);
      }
    });
  }

  function sameProject(identity: { projectId: string }) {
    return !disposed && session?.projectId === identity.projectId;
  }
  function sameSession(identity: { projectId: string; generation: number }) {
    return sameProject(identity) && session?.generation === identity.generation;
  }

  function beginSync(message = "正在准备同步…") {
    syncBusy = true;
    syncStopping = false;
    syncError = "";
    syncStartedAt = Date.now();
    syncElapsed = 0;
    syncProgress = { phase: "preparing", status: "checking", message };
  }

  function endSync() { syncBusy = false; syncStopping = false; syncProgress = null; }

  async function stopSync() {
    if (!session || !syncBusy || syncStopping) return;
    syncStopping = true;
    try {
      await platform.cancelContentSync(session.projectId, session.generation);
    } catch (error) { syncError = normalizeError(error).message; syncStopping = false; }
  }

  async function refreshSync() {
    if (!session) {
      syncCandidate = null;
      syncStatus = { enabled: false, status: "off", provider: "github", conflicts: [] };
      return;
    }
    if (syncLoading) return;
    syncLoading = true;
    const identity = { ...session };
    const revision = syncRevision;
    try {
      const [detection, status, activeProgress] = await Promise.all([
        platform.detectContentSync(identity.projectId, identity.generation),
        platform.getContentSyncStatus(identity.projectId, identity.generation),
        platform.getContentSyncProgress(identity.projectId)
      ]);
      if (!sameSession(identity)) return;
      syncLoaded = true;
      if (activeProgress && activeProgress.sessionGeneration === identity.generation && !syncBusy) { beginSync(); backgroundSyncBusy = true; syncProgress = activeProgress; }
      syncCandidates = detection.candidates;
      if (revision === syncRevision) syncStatus = status;
      syncProvider = syncStatus.enabled ? syncStatus.provider : syncProvider;
      syncCandidate = syncStatus.repository
        ? detection.candidates.find((item) => item.repository === syncStatus.repository) ?? null
        : detection.requiresSelection
          ? null
          : detection.candidates[0] ?? null;
      syncBranch = syncStatus.branch || "hexo-lite-content";
      webDavEndpoint = syncStatus.endpoint || webDavEndpoint;
      webDavRemoteDir = syncStatus.remoteDir || webDavRemoteDir;
      if (syncStatus.provider === "webdav" && webDavEndpoint) void refreshWebDavCredential();
      void refreshSyncConflicts().catch((error) => { if (!disposed) syncError = normalizeError(error).message; });
      void refreshSyncSummary();
    } catch (error) {
      onNotice(normalizeError(error).message, "error");
    } finally {
      syncLoading = false;
    }
  }

  function chooseSyncProvider(provider: ContentSyncProvider) {
    syncProvider = provider;
    syncPreflight = null;
    webDavPreflight = null;
    publicAcknowledged = false;
    syncError = "";
    webDavConnectionError = "";
  }

  function chooseConnectionProvider(provider: ContentSyncProvider) {
    if (!syncStatus.enabled || syncBusy) return;
    syncProvider = provider;
    if (provider === syncStatus.provider) { switchingProvider = null; return; }
    switchingProvider = provider;
    syncPreflight = null;
    webDavPreflight = null;
    publicAcknowledged = false;
    syncError = "";
    webDavConnectionError = "";
    webDavConnectionOpen = true;
  }

  async function configureSync() {
    if (!session || syncBusy) return;
    if (syncProvider === "github" && !syncCandidate) return;
    const identity = { ...session };
    if (!await onBeforeSync() || !sameSession(identity) || syncBusy) return;
    beginSync(syncProvider === "github" ? "正在连接 GitHub，准备首次合并。" : "正在连接 WebDAV，准备首次合并。");
    await tick();
    try {
      let result: import("$shared/types/app").ContentSyncView;
      if (syncProvider === "github" && syncCandidate) {
        result = await platform.enableContentSync({
          projectId: session.projectId,
          sessionGeneration: session.generation,
          repository: syncCandidate.repository,
          branch: syncBranch,
          confirmPublic: syncCandidate.visibility !== "public" || publicAcknowledged
        });
      } else {
        result = await platform.enableWebDavContentSync({
          projectId: session.projectId,
          sessionGeneration: session.generation,
          endpoint: webDavEndpoint,
          remoteDir: webDavRemoteDir,
        });
      }
      if (!sameProject(identity)) return;
      syncStatus = result;
      syncProvider = result.provider;
      switchingProvider = null;
      await refreshSyncConflicts();
      if (syncStatus.enabled) webDavConnectionOpen = false;
      onNotice(syncStatus.message ? $ui(syncStatus.message) : $ui("同步设置已更新。"));
    } catch (error) {
      syncError = normalizeError(error).message; onNotice(syncError, "error");
    } finally {
      endSync();
      void refreshSyncSummary();
    }
  }

  async function preflightSync() {
    if (!session || syncBusy || (syncProvider === "github" && !syncCandidate)) return;
    const identity = { ...session };
    const branch = syncBranch;
    const repository = syncCandidate?.repository;
    beginSync("正在检查 GitHub 连接和两端差异。");
    await tick();
    try {
      if (syncProvider === "github" && syncCandidate) {
        const result = await platform.preflightContentSync(identity.projectId, identity.generation, syncCandidate.repository, branch);
        if (!sameSession(identity) || syncBranch !== branch || syncCandidate?.repository !== repository) return;
        syncPreflight = result;
        syncCandidate = syncPreflight.candidate;
      }
    } catch (error) {
      syncError = normalizeError(error).message; onNotice(syncError, "error");
    } finally {
      endSync();
    }
  }

  async function refreshWebDavCredential() {
    if (!webDavEndpoint.trim()) {
      webDavCredential = { configured: false };
      return;
    }
    const endpoint = webDavEndpoint;
    try {
      const result = await platform.webDavCredentialStatus(endpoint);
      if (disposed || webDavEndpoint !== endpoint) return;
      webDavCredential = result;
      if (!webDavUsername && webDavCredential.username) webDavUsername = webDavCredential.username;
    }
    catch { webDavCredential = { configured: false }; }
  }

  async function testWebDavConnection() {
    if (!session || syncBusy || !webDavEndpoint.trim() || !webDavRemoteDir.trim() || !webDavUsername.trim()) return;
    const identity = { ...session };
    const submitted = { endpoint: webDavEndpoint, remoteDir: webDavRemoteDir, username: webDavUsername, password: webDavPassword };
    beginSync("正在测试 WebDAV 连接、读取云端清单。");
    await tick();
    webDavConnectionError = "";
    try {
      const result = await platform.testWebDavContentSync({
        projectId: session.projectId,
        sessionGeneration: session.generation,
        ...submitted, password: submitted.password || undefined
      });
      if (!sameSession(identity)) return;
      if (webDavEndpoint !== submitted.endpoint || webDavRemoteDir !== submitted.remoteDir || webDavUsername !== submitted.username || webDavPassword !== submitted.password) {
        webDavPreflight = null; webDavTestedAt = "";
        webDavConnectionError = $ui("测试时连接信息变了，用当前输入重新测试。");
        return;
      }
      webDavEndpoint = result.preflight.endpoint;
      webDavRemoteDir = result.preflight.remoteDir;
      webDavPreflight = result.preflight;
      webDavUsername = result.username;
      webDavPassword = "";
      webDavTestedAt = result.testedAt;
      webDavTestedEndpoint = result.preflight.endpoint;
      webDavTestedRemoteDir = result.preflight.remoteDir;
      webDavCredential = { configured: true, username: result.username };
      syncStatus = result.sync;
      onNotice($ui("WebDAV 连接、读写权限和预检都通过了。"));
    } catch (error) {
      webDavConnectionError = normalizeError(error).message;
      webDavPreflight = null;
      webDavTestedAt = "";
      onNotice(webDavConnectionError, "error");
    } finally {
      endSync();
    }
  }

  async function applyWebDavConnection() {
    if (!session || syncBusy || !webDavTestMatches) return;
    const identity = { ...session };
    beginSync("正在应用 WebDAV 连接设置。");
    await tick();
    webDavConnectionError = "";
    try {
      const result = await platform.updateWebDavContentSync({
        projectId: session.projectId,
        sessionGeneration: session.generation,
        endpoint: webDavEndpoint,
        remoteDir: webDavRemoteDir
      });
      if (!sameSession(identity)) return;
      syncStatus = result;
      onNotice(syncStatus.message ? $ui(syncStatus.message) : $ui("WebDAV 连接设置已应用。"));
    } catch (error) {
      webDavConnectionError = normalizeError(error).message;
      onNotice(webDavConnectionError, "error");
    } finally {
      endSync();
    }
  }

  async function deleteWebDavCredential() {
    if (syncBusy || !webDavEndpoint.trim()) return;
    beginSync();
    try {
      webDavCredential = await platform.webDavCredentialDelete(webDavEndpoint);
      webDavPreflight = null;
      webDavTestedAt = "";
      webDavConnectionError = "";
      onNotice($ui("WebDAV 凭据已删除。"));
    } catch (error) {
      syncError = normalizeError(error).message; onNotice(syncError, "error");
    } finally {
      endSync();
    }
  }

  async function refreshSyncConflicts() {
    if (!session || syncStatus.status !== "conflict") {
      syncConflicts = [];
      conflictChoices = {};
      return;
    }
    const identity = { ...session };
    const conflicts = await platform.getContentSyncConflicts(identity.projectId, identity.generation);
    if (!sameSession(identity)) return;
    syncConflicts = conflicts;
    conflictChoices = {};
  }

  async function submitConflictChoices() {
    if (!session || syncBusy || syncConflicts.some((item) => !conflictChoices[item.path])) return;
    const identity = { ...session };
    if (!await onBeforeSync() || !sameSession(identity) || syncBusy) return;
    beginSync();
    syncError = "";
    try {
      const result = await platform.resolveContentSyncConflicts(identity.projectId, identity.generation, conflictChoices);
      if (!sameProject(identity)) return;
      syncStatus = result;
      syncConflicts = [];
      conflictChoices = {};
      onNotice(syncStatus.message ? $ui(syncStatus.message) : $ui("冲突已解决。"));
      void refreshSyncSummary();
    } catch (error) {
      syncError = normalizeError(error).message;
      onNotice(syncError, "error");
    } finally {
      endSync();
    }
  }

  async function runSync(direction: "auto" | "local" | "remote" | "overwriteLocal" | "overwriteRemote" = "auto", confirmScope = false) {
    if (!session || syncBusy) return;
    const identity = { ...session };
    if (!await onBeforeSync() || !sameSession(identity) || syncBusy) return;
    beginSync("正在检查本机和云端的变化，先别关这个页面。");
    await tick();
    syncError = "";
    try {
      const result = await platform.runContentSync(identity.projectId, identity.generation, direction, confirmScope);
      if (!sameProject(identity)) return;
      syncStatus = result;
      await refreshSyncConflicts();
      onNotice(syncStatus.message ? $ui(syncStatus.message) : $ui("检查完成。"));
    } catch (error) {
      syncError = normalizeError(error).message;
      onNotice(syncError, "error");
    } finally {
      endSync();
      void refreshSyncSummary();
    }
  }

  async function refreshSyncSummary() {
    if (!session) { syncSummary = null; return; }
    const identity = { ...session };
    const request = ++summaryRequest;
    summaryBusy = true;
    summaryError = "";
    try {
      const result = await platform.getContentSyncSummary(identity.projectId, identity.generation);
      if (sameSession(identity) && request === summaryRequest) syncSummary = result;
    } catch (error) {
      if (sameSession(identity) && request === summaryRequest) summaryError = normalizeError(error).message;
    } finally {
      if (request === summaryRequest) summaryBusy = false;
    }
  }

  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }

  const scopeLabels = { articles: "文章与草稿", site: "配置与页面", themes: "主题与模块", assets: "图片与资源" };

  async function publishSite() {
    if (syncBusy || taskBusy) return;
    try {
      await flushDraft();
      await onPublish();
    } catch (error) { onNotice(normalizeError(error).message, "error"); }
  }

  function chooseAllConflicts(choice: "local" | "remote") {
    conflictChoices = Object.fromEntries(syncConflicts.map((item) => [item.path, choice]));
  }

  async function confirmSyncOverwrite() {
    const direction = pendingSyncOverwrite;
    pendingSyncOverwrite = null;
    if (direction) await runSync(direction);
  }

  async function disableSync() {
    if (!session || syncBusy) return;
    beginSync();
    try {
      syncStatus = await platform.disableContentSync(session.projectId, session.generation);
      switchingProvider = null;
      onNotice($ui("同步已关闭，本地文章不会被删。"));
    } catch (error) {
      syncError = normalizeError(error).message; onNotice(syncError, "error");
    } finally {
      endSync();
    }
  }

  async function reconnectSync() {
    if (!session || syncBusy) return;
    beginSync();
    try {
      syncStatus = await platform.reconnectContentSync(session.projectId, session.generation);
      onNotice(syncStatus.message ? $ui(syncStatus.message) : $ui("Git 认证检查完成。"));
    } catch (error) {
      syncError = normalizeError(error).message; onNotice(syncError, "error");
    } finally {
      endSync();
    }
  }

  function handleNavKeydown(event: KeyboardEvent, index: number) {
    if (!["ArrowDown", "ArrowUp", "ArrowRight", "ArrowLeft", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const nextIndex = event.key === "Home"
      ? 0
      : event.key === "End"
        ? sections.length - 1
        : (index + (event.key === "ArrowDown" || event.key === "ArrowRight" ? 1 : -1) + sections.length) % sections.length;
    selectSection(sections[nextIndex].id);
    requestAnimationFrame(() => pageElement.querySelector<HTMLButtonElement>(`[data-settings-section="${sections[nextIndex].id}"]`)?.focus({ preventScroll: true }));
  }

  function change(next: AppConfigV3) {
    const previousLanguage = draft.general.language;
    draft = next;
    onThemePreview(next.appearance.themeMode);
    saveError = "";
    if (next.general.language !== previousLanguage) setLanguage(next.general.language);
    schedulePersist();
  }

  function schedulePersist() {
    if (persistTimer) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => { persistTimer = undefined; void flushDraft().catch(() => null); }, 350);
  }

  async function flushDraft() {
    if (persistTimer) { clearTimeout(persistTimer); persistTimer = undefined; }
    if (JSON.stringify(draft) === JSON.stringify(saved)) return;
    await persistConfig(draft);
  }

  async function persistConfig(nextConfig: AppConfigV3, message = "") {
    if (pendingSave) await pendingSave;
    const submitted = structuredClone(nextConfig);
    const validation = validateSettings(submitted);
    if (validation) {
      saveError = $ui(validation.message);
      draft = structuredClone(saved);
      onThemePreview(draft.appearance.themeMode);
      setLanguage(draft.general.language);
      selectSection(validation.section);
      await tick();
      const field = pageElement?.querySelector<HTMLInputElement>(`[data-config-field="${validation.field}"]`);
      field?.focus();
      throw new Error(saveError);
    }
    saving = true;
    saveError = "";
    const operation = (async () => {
      try {
        const next = await onSaveConfig(submitted);
        saved = structuredClone(next);
        draft = mergeSavedDraft(submitted, draft, next);
        if (!disposed) {
          onThemePreview(draft.appearance.themeMode);
          setLanguage(draft.general.language);
          if (message) onNotice($ui(message));
        }
      } catch (error) {
        saveError = normalizeError(error).message;
        if (!disposed) onNotice(saveError, "error");
        throw error;
      } finally { saving = false; }
    })();
    pendingSave = operation;
    try { await operation; }
    finally { if (pendingSave === operation) pendingSave = null; }
  }

  async function persistImageBed(patch: Partial<AppConfigV3["imageBed"]>, message: string) {
    await flushDraft();
    draft = { ...draft, imageBed: { ...draft.imageBed, ...patch } };
    await persistConfig({ ...saved, imageBed: { ...saved.imageBed, ...patch } }, message);
  }

  async function restoreDefaults() {
    showReset = false;
    if (persistTimer) { clearTimeout(persistTimer); persistTimer = undefined; }
    try {
      await persistConfig(structuredClone(defaultConfig), "已恢复默认设置。");
    } catch (error) { saveError = normalizeError(error).message; }
  }

  async function refreshCredential() {
    const request = ++credentialRequest;
    try {
      const result = await Promise.all([
        platform.credentialStatus(
          draft.imageBed.cloudflareConnectionId,
          draft.imageBed.cloudflareApiUrl
        ),
        platform.credentialLegacyAvailable()
      ]);
      if (!disposed && request === credentialRequest) [credential, legacyCredentialAvailable] = result;
    }
    catch { if (request === credentialRequest) credential = { configured: false }; }
  }

  function updateImageBed(imageBed: AppConfigV3["imageBed"]) {
    change({ ...draft, imageBed });
    void refreshCredential();
  }

  async function migrateLegacyCredential() {
    if (!draft.imageBed.cloudflareApiUrl.trim()) {
      onNotice($ui("先填 Cloudflare-ImgBed 服务地址。"));
      return;
    }
    credentialBusy = true;
    try {
      credential = await platform.credentialMigrate(
        draft.imageBed.cloudflareConnectionId,
        draft.imageBed.cloudflareApiUrl
      );
      legacyCredentialAvailable = false;
      tokenStatusMessage = $ui("旧 Token 已绑到当前连接，临时凭据已清理。");
    } catch (error) {
      tokenStatusMessage = normalizeError(error).message;
    } finally {
      credentialBusy = false;
    }
  }

  async function prepareAcquireToken() {
    if (credentialBusy) return;
    if (!draft.imageBed.cloudflareApiUrl.trim()) return onNotice($ui("先填 Cloudflare-ImgBed 服务地址。"));
    credentialBusy = true;
    try {
      await persistImageBed({
        cloudflareApiUrl: draft.imageBed.cloudflareApiUrl,
        cloudflareConnectionId: draft.imageBed.cloudflareConnectionId
      }, "图床地址已保存，其他设置不受影响。");
      showAcquireToken = true;
      tokenStatusMessage = "";
    } catch (error) { onNotice(normalizeError(error).message, "error"); }
    finally { credentialBusy = false; }
  }

  function closeAcquireToken() {
    if (credentialBusy) return;
    showAcquireToken = false;
    adminUsername = "";
    adminPassword = "";
  }

  async function acquireToken() {
    if (credentialBusy) return;
    credentialBusy = true;
    tokenStatusMessage = $ui("正在获取 Token…");
    try {
      const result = await platform.acquireCloudflareImgbedToken(
        draft.imageBed.cloudflareConnectionId,
        {
        baseUrl: draft.imageBed.cloudflareApiUrl,
        adminUsername: adminUsername.trim() || undefined,
        adminPassword: adminPassword || undefined,
        tokenName: draft.imageBed.cloudflareName.trim() || "Hexo Lite Editor",
        owner: "Hexo Lite Editor",
        permissions: ["upload", "list", "delete"],
        expiresAt: null,
        autoDelete: false
      });
      credential = { configured: result.configured };
      await persistImageBed({ cloudflareTokenId: result.tokenId }, "Token 已创建，存进了系统凭据库。");
      tokenStatusMessage = $ui("Token 已配置");
      showAcquireToken = false;
      adminUsername = "";
    } catch (error) { tokenStatusMessage = normalizeError(error).message; }
    finally { adminPassword = ""; credentialBusy = false; }
  }

  async function testCredential() {
    if (credentialBusy) return;
    credentialBusy = true;
    tokenStatusMessage = $ui("正在测试连接…");
    try {
      tokenStatusMessage = (
        await platform.testCloudflareImgbedToken(
          draft.imageBed.cloudflareConnectionId,
          draft.imageBed.cloudflareApiUrl
        )
      ).message;
    }
    catch (error) { tokenStatusMessage = normalizeError(error).message; }
    finally { credentialBusy = false; }
  }

  async function deleteCredential() {
    credentialBusy = true;
    try {
      credential = await platform.credentialDelete(draft.imageBed.cloudflareConnectionId);
      await persistImageBed({ cloudflareTokenId: undefined }, "Token 已从系统凭据库删除。");
      tokenStatusMessage = $ui("本地 Token 已删除");
    } catch (error) { onNotice(normalizeError(error).message, "error"); }
    finally { credentialBusy = false; }
  }

  async function removeRecent(recentId: string) {
    try { await onRemoveRecentProject(recentId); }
    catch (error) { onNotice(normalizeError(error).message, "error"); }
  }

  async function clearRecent() {
    try { await onClearRecentProjects(); showClearRecent = false; }
    catch (error) { onNotice(normalizeError(error).message, "error"); }
  }

</script>

<div class="workspace-page settings-page" bind:this={pageElement}>
  <SettingsHeader {dirty} {saving} />
  {#if saveError}<p class="sync-error" role="alert">{saveError}</p>{/if}

  <div class="settings-layout">
    <SettingsNavigation {sections} {activeSection} {dirtySections} onSelect={selectSection} onKeydown={handleNavKeydown} />

    <section class="panel settings-content-panel" aria-labelledby={`settings-title-${activeSection}`}>
      <header class="settings-content-heading">
        <h2 id={`settings-title-${activeSection}`}>{$ui(currentSection.title)}</h2>
        <span>{$ui(currentSection.description)}</span>
      </header>

      {#if activeSection === "general"}
        <div class="settings-block">

          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-1">{$translate("settings.languageTitle")}</label></div><select id="setting-field-1" class="select compact-control" aria-label={$translate("settings.languageTitle")} value={draft.general.language} on:change={(event) => change({ ...draft, general: { ...draft.general, language: event.currentTarget.value as AppConfigV3["general"]["language"] } })}><option value="system">{$translate("settings.languageSystem")}</option><option value="zh-CN">{$translate("settings.languageChinese")}</option><option value="en-US">{$translate("settings.languageEnglish")}</option></select></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("启动")}</h3></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-2">{$ui("启动时打开最近项目")}</label><span id="setting-field-2-hint">{$ui("继续上次的写作。")}</span></div><label class="switch"><input id="setting-field-2" aria-describedby="setting-field-2-hint" type="checkbox" checked={draft.general.openRecentProjectOnStart} on:change={(event) => change({ ...draft, general: { ...draft.general, openRecentProjectOnStart: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("保存与备份")}</h3></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-3">{$ui("自动保存")}</label><span id="setting-field-3-hint">{$ui("停止输入后保存文章。")}</span></div><label class="switch"><input id="setting-field-3" aria-describedby="setting-field-3-hint" type="checkbox" checked={draft.general.autoSave} on:change={(event) => change({ ...draft, general: { ...draft.general, autoSave: event.currentTarget.checked } })} /><span></span></label></div>
          <div class:disabled={!draft.general.autoSave} class="setting-row setting-row-dependent"><div class="setting-copy"><label class="setting-title" for="setting-field-4">{$ui("自动保存延迟")}</label><span id="setting-field-4-hint">{$ui("毫秒，500–30000。")}</span></div><input data-config-field="autoSaveDelayMs" id="setting-field-4" aria-describedby="setting-field-4-hint" class="input compact-control" disabled={!draft.general.autoSave} type="number" min="500" max="30000" step="100" value={draft.general.autoSaveDelayMs} on:input={(event) => change({ ...draft, general: { ...draft.general, autoSaveDelayMs: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-5">{$ui("保存前创建备份")}</label><span id="setting-field-5-hint">{$ui("保留修改前的文件版本。")}</span></div><label class="switch"><input id="setting-field-5" aria-describedby="setting-field-5-hint" type="checkbox" checked={draft.general.backupBeforeSave} on:change={(event) => change({ ...draft, general: { ...draft.general, backupBeforeSave: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <details class="settings-block settings-disclosure">
          <summary>{$ui("最近项目")} <span>{recentProjects.length} / 10</span></summary>
          {#if recentProjects.length}
            <div class="recent-project-list">{#each recentProjects as recent (recent.recentId)}<div class="recent-project-row"><div><strong>{recent.name}</strong><span>{recent.displayPath}</span></div><span class:warning={!recent.available} class="recent-availability">{recent.available ? $ui("可用") : $ui("不可用")}</span><button class="icon-button" type="button" title={$ui("移除记录")} aria-label={$ui("移除 {p0}", { p0: recent.name })} on:click={() => removeRecent(recent.recentId)}><Trash2 size={15} /></button></div>{/each}</div>
            <button class="button danger" type="button" on:click={() => (showClearRecent = true)}>{$ui("清空记录")}</button>
          {:else}<p class="muted-line">{$ui("尚无最近项目。")}</p>{/if}
        </details>
      {:else if activeSection === "editing"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("外观")}</h3><p>{$ui("改主题立刻能看到效果。")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-6">{$ui("主题模式")}</label><span id="setting-field-6-hint">{$ui("浅色、深色或跟随系统。")}</span></div><select id="setting-field-6" aria-describedby="setting-field-6-hint" class="select compact-control" value={draft.appearance.themeMode} on:change={(event) => change({ ...draft, appearance: { themeMode: event.currentTarget.value as ThemeMode } })}><option value="system">{$ui("跟随系统")}</option><option value="light">{$ui("浅色")}</option><option value="dark">{$ui("深色")}</option></select></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("正文排版")}</h3></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-7">{$ui("字号")}</label><span id="setting-field-7-hint">12–28 px。</span></div><input data-config-field="fontSize" id="setting-field-7" aria-describedby="setting-field-7-hint" class="input compact-control" type="number" min="12" max="28" value={draft.editor.fontSize} on:input={(event) => change({ ...draft, editor: { ...draft.editor, fontSize: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-8">{$ui("行高")}</label><span id="setting-field-8-hint">1.2–2.2。</span></div><input data-config-field="lineHeight" id="setting-field-8" aria-describedby="setting-field-8-hint" class="input compact-control" type="number" min="1.2" max="2.2" step="0.05" value={draft.editor.lineHeight} on:input={(event) => change({ ...draft, editor: { ...draft.editor, lineHeight: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-9">{$ui("Tab 宽度")}</label><span id="setting-field-9-hint">{$ui("使用 2、4 或 8 个空格。")}</span></div><select id="setting-field-9" aria-describedby="setting-field-9-hint" class="select compact-control" value={draft.editor.tabSize} on:change={(event) => change({ ...draft, editor: { ...draft.editor, tabSize: Number(event.currentTarget.value) } })}><option value="2">2</option><option value="4">4</option><option value="8">8</option></select></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("编辑辅助")}</h3></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-10">{$ui("显示行号")}</label><span id="setting-field-10-hint">{$ui("在正文左侧显示行号栏。")}</span></div><label class="switch"><input id="setting-field-10" aria-describedby="setting-field-10-hint" type="checkbox" checked={draft.editor.showLineNumbers} on:change={(event) => change({ ...draft, editor: { ...draft.editor, showLineNumbers: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-11">{$ui("自动换行")}</label><span id="setting-field-11-hint">{$ui("长行按编辑区宽度折行。")}</span></div><label class="switch"><input id="setting-field-11" aria-describedby="setting-field-11-hint" type="checkbox" checked={draft.editor.lineWrapping} on:change={(event) => change({ ...draft, editor: { ...draft.editor, lineWrapping: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-12">{$ui("突出当前行")}</label><span id="setting-field-12-hint">{$ui("高亮光标所在的那一行。")}</span></div><label class="switch"><input id="setting-field-12" aria-describedby="setting-field-12-hint" type="checkbox" checked={draft.editor.highlightActiveLine} on:change={(event) => change({ ...draft, editor: { ...draft.editor, highlightActiveLine: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-13">{$ui("文章列表封面")}</label><span id="setting-field-13-hint">{$ui("在文章标题左侧显示缩略图。")}</span></div><label class="switch"><input id="setting-field-13" aria-describedby="setting-field-13-hint" type="checkbox" checked={draft.articleList.showCover} on:change={(event) => change({ ...draft, articleList: { showCover: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
      {:else if activeSection === "images"}
        <div class="settings-block">

          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-14">{$ui("图片保存到")}</label><span id="setting-field-14-hint">{$ui("适用于粘贴、拖入和导入图片。")}</span></div><select id="setting-field-14" aria-describedby="setting-field-14-hint" class="select compact-control" value={draft.imageBed.defaultProvider} on:change={(event) => change({ ...draft, imageBed: { ...draft.imageBed, defaultProvider: event.currentTarget.value as AppConfigV3["imageBed"]["defaultProvider"] } })}><option value="local">{$ui("本地图片")}</option><option value="cloudflare-imgbed">Cloudflare-ImgBed</option>{#if draft.imageBed.defaultProvider.startsWith("plugin:")}<option value={draft.imageBed.defaultProvider}>{$ui("插件图床")}</option>{/if}</select></div>
        </div>
        {#if draft.imageBed.defaultProvider !== "local"}
          <div class="settings-block provider-block">
            <div class="settings-block-heading"><h3>{draft.imageBed.defaultProvider === "cloudflare-imgbed" ? $ui("Cloudflare 连接") : $ui("插件图床")}</h3><p>{draft.imageBed.defaultProvider === "cloudflare-imgbed" ? $ui("Token 改动即时生效。") : $ui("连接在插件页管理。")}</p></div>
            {#if draft.imageBed.defaultProvider === "cloudflare-imgbed"}
            <CloudflareImageBedSettings settings={draft.imageBed} {credential} {legacyCredentialAvailable} busy={credentialBusy} statusMessage={tokenStatusMessage} onChange={updateImageBed} onAcquireToken={prepareAcquireToken} onMigrateLegacyToken={migrateLegacyCredential} onTestConnection={testCredential} onDeleteToken={deleteCredential} />
            {:else}
              <p class="muted-line">{$ui("粘贴或导入图片时使用当前插件。")}</p>
            {/if}
          </div>
        {/if}
      {:else if activeSection === "hexoPublish"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("网站预览")}</h3><p>{$ui("在浏览器里看真正的网站。")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-15">{$ui("预览端口")}</label><span id="setting-field-15-hint">{$ui("默认使用 4000。")}</span></div><input data-config-field="previewPort" id="setting-field-15" aria-describedby="setting-field-15-hint" class="input compact-control" type="number" min="300" max="65535" value={draft.hexo.previewPort} on:input={(event) => change({ ...draft, hexo: { ...draft.hexo, previewPort: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-16">{$ui("打开项目后自动启动预览")}</label><span id="setting-field-16-hint">{$ui("后台启动，不打开浏览器。")}</span></div><label class="switch"><input id="setting-field-16" aria-describedby="setting-field-16-hint" type="checkbox" checked={draft.hexo.autoStartPreview} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, autoStartPreview: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-17">{$ui("预览草稿")}</label><span id="setting-field-17-hint">{$ui("在本机预览未发布的文章。")}</span></div><label class="switch"><input id="setting-field-17" aria-describedby="setting-field-17-hint" type="checkbox" checked={draft.hexo.previewDrafts} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, previewDrafts: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("发布网站")}</h3><p>{$ui("发布快捷键为")} {shortcutLabel("⇧P")}。</p></div>
          <p class="muted-line publish-sequence">{$ui("保存后依次清理、生成、部署")}</p>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-18">{$ui("部署后 Git Push")}</label><span id="setting-field-18-hint">{$ui("部署成功后推送当前 Git 分支。")}</span></div><label class="switch"><input id="setting-field-18" aria-describedby="setting-field-18-hint" type="checkbox" checked={draft.publish.gitPushAfterDeploy} on:change={(event) => change({ ...draft, publish: { ...draft.publish, gitPushAfterDeploy: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
      {:else if activeSection === "sync"}
        {#if !session}
          <div class="settings-block"><p class="muted-line">{$ui("请先打开一个 Hexo 项目。")}</p></div>
        {:else}
          {#if !syncStatus.enabled}
          <div class="settings-block sync-plan-block">
            <div class="settings-block-heading"><h3>{$ui("同步规划")}</h3><p>{$ui("先选通道和范围，再连接；之后每次保存会自动排队同步。")}</p></div>
            <ol class="sync-plan">
              <li><span>1</span><div><strong>{$ui("选择通道")}</strong><p>{$ui("GitHub 用一个独立分支；WebDAV 用你指定的目录。")}</p></div></li>
              <li><span>2</span><div><strong>{$ui("检查连接与差异")}</strong><p>{$ui("预检只读本机和云端的清单，不会动文件。")}</p></div></li>
              <li><span>3</span><div><strong>{$ui("合并并持续同步")}</strong><p>{$ui("首次合并保留两边独有的文件，冲突逐个让你选。保存后约 30 秒自动上传。")}</p></div></li>
            </ol>
          </div>
          {/if}
          {#if syncLoading && !syncLoaded}
            <div class="settings-block sync-loading" role="status">{$ui("正在读取同步设置…")}</div>
          {:else}
          {#if !syncStatus.enabled}
            <div class="settings-block sync-provider-block">
              <SyncProviderPicker value={syncProvider} disabled={syncBusy} on:change={(event) => chooseSyncProvider(event.detail)} />
            </div>
          {/if}
          {#if syncStatus.enabled || syncBusy || syncError}
          <div class="settings-block sync-state-block">
            <div class="sync-state-heading">
              <div><h3>{session.name}</h3><span class={`sync-status ${syncStatus.status}`}>{$ui(syncStatusLabel(syncStatus))}</span>{#if syncStatus.enabled}<span class="sync-provider">{syncStatus.provider === "webdav" ? "WebDAV" : "GitHub"}</span>{/if}</div>
              {#if syncStatus.lastSyncedAt}<span class="muted-line">{$ui("上次同步：")}{new Date(syncStatus.lastSyncedAt).toLocaleString()}</span>{/if}
            </div>
            {#if syncError}<p class="sync-error" role="alert">{syncError}</p>{/if}
            {#if syncBusy}
              <div class="sync-progress" role="status" aria-live="polite">
                <strong>{syncStopping ? $ui("正在停止同步…") : syncProgress?.message ? $ui(syncProgress.message) : $ui("正在准备同步…")}</strong>
                {#if syncProgress?.totalFiles != null && syncProgress.totalFiles > 0}
                  <progress max={syncProgress.totalFiles} value={syncProgress.completedFiles ?? 0} aria-label={$ui("同步文件进度")}></progress>
                  <span>{Math.min(100, Math.round((syncProgress.completedFiles ?? 0) / syncProgress.totalFiles * 100))}% · {$ui("{p0} / {p1} 个文件", { p0: syncProgress.completedFiles ?? 0, p1: syncProgress.totalFiles })}</span>
                {/if}
                <span>{$ui("已用时 {p0} 秒", { p0: syncElapsed })}</span>
                <button class="button" type="button" disabled={syncStopping} on:click={stopSync}>{syncStopping ? $ui("等待当前请求结束") : $ui("停止同步")}</button>
              </div>
            {/if}
            {#if syncStatus.enabled}
              <fieldset class="sync-decisions" disabled={syncBusy || webDavConnectionDirty}>
                {#if syncStatus.requiresScopeConfirmation}
                  <label class="sync-warning"><input type="checkbox" bind:checked={scopeAcknowledged} /><span>{$ui("我同意把草稿、配置和主题传到公开仓库。")}</span></label>
                  <button class="button primary" type="button" disabled={syncBusy || !scopeAcknowledged} on:click={() => runSync("auto", true)}>{$ui("确认范围并合并同步")}</button>
                {:else if syncStatus.status === "conflict"}
                  <p class="muted-line">{$ui("本地和云端改了同一个文件，逐个选一下。")}</p>
                  <details class="settings-disclosure"><summary>{$ui("批量选择")}</summary><div class="button-row"><button class="button" type="button" on:click={() => chooseAllConflicts("local")}>{$ui("全部用本地")}</button><button class="button" type="button" on:click={() => chooseAllConflicts("remote")}>{$ui("全部用云端")}</button></div></details>
                <div class="sync-conflict-list">
                  {#each syncConflicts as conflict}
                    <article class="sync-conflict-card" aria-labelledby={`conflict-${conflict.path}`}>
                      <strong id={`conflict-${conflict.path}`}>{conflict.path}</strong>
                      <span>{conflict.kind === "markdown" ? $ui("Markdown 文本") : $ui("二进制 · 本地 {p0} B / 云端 {p1} B", { p0: conflict.localSize ?? 0, p1: conflict.remoteSize ?? 0 })}</span>
                      {#if conflict.kind === "binary"}<code class="sync-conflict-hashes">{$ui("本地")} {conflict.localHash ?? $ui("已删除")} {$ui("· 云端")} {conflict.remoteHash ?? $ui("已删除")}</code>{/if}
                      {#if conflict.kind === "markdown"}<details><summary>{$ui("查看两端内容")}</summary><div class="sync-diff"><pre>{conflict.localText ?? $ui("（本地已删除）")}</pre><pre>{conflict.remoteText ?? $ui("（云端已删除）")}</pre></div></details>{/if}
                      <div class="button-row"><label><input type="radio" name={`sync-${conflict.path}`} value="local" checked={conflictChoices[conflict.path] === "local"} on:change={() => (conflictChoices = { ...conflictChoices, [conflict.path]: "local" })} /> {$ui("本地")}</label><label><input type="radio" name={`sync-${conflict.path}`} value="remote" checked={conflictChoices[conflict.path] === "remote"} on:change={() => (conflictChoices = { ...conflictChoices, [conflict.path]: "remote" })} /> {$ui("云端")}</label></div>
                    </article>
                  {/each}
                </div>
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy || !syncConflicts.length || syncConflicts.some((item) => !conflictChoices[item.path])} on:click={submitConflictChoices}>{$ui("提交选择")}</button><button class="button" type="button" on:click={() => session && platform.openContentSyncBackups(session.projectId, session.generation)}>{$ui("打开备份目录")}</button></div>

                  <button class="button" type="button" on:click={() => runSync("auto")}>{$ui("重新检查冲突")}</button>
                {:else}
                  {#if ["error", "offline", "authRequired"].includes(syncStatus.status) && syncStatus.message}<p class="sync-warning" role="status">{syncStatus.message}</p>{/if}
                  {#if syncStatus.status === "remoteAhead"}<p class="muted-line">{$ui("双向合并，遇到冲突会让你逐个选。")}</p>{/if}
                  <div class="button-row"><button class="button primary" type="button" disabled={syncBusy} on:click={() => runSync("auto")}>{["error", "offline", "authRequired"].includes(syncStatus.status) ? $ui("重试同步") : syncStatus.status === "remoteAhead" ? $ui("合并云端改动") : !syncStatus.lastSyncedAt ? $ui("合并并开始同步") : $ui("立即同步")}</button>{#if syncStatus.status === "authRequired" && syncProvider === "github"}<button class="button" type="button" disabled={syncBusy} on:click={reconnectSync}>{$ui("重新认证")}</button>{/if}</div>
                {/if}
              </fieldset>
            {:else}
              <p class="muted-line">{$ui("连接 GitHub 或 WebDAV，开始跨设备同步。")}</p>
            {/if}
          </div>
          {/if}
          {#if syncStatus.enabled}
          <div class="settings-block">
            <div class="setting-subsection-heading"><h3>{$ui("本次变化")}</h3><button class="button" type="button" disabled={syncBusy || summaryBusy} on:click={refreshSyncSummary}>{summaryBusy ? $ui("正在扫描…") : $ui("刷新")}</button></div>
            {#if summaryError}<p class="sync-error" role="alert">{summaryError}</p>{/if}
            {#if syncSummary}
              <p class="sync-change-summary" role="status">{$ui("待上传 {p0} 个文件 · {p1}", { p0: syncSummary.pendingFileCount, p1: formatBytes(syncSummary.pendingBytes) })}{#if syncSummary.deletedFileCount} · {$ui("删除 {p0} 个文件", { p0: syncSummary.deletedFileCount })}{/if}</p>
              <details class="settings-disclosure sync-scope">
                <summary>{$ui("同步细节")} <span>{$ui("{p0} 个文件 · {p1}", { p0: syncSummary.fileCount, p1: formatBytes(syncSummary.totalBytes) })}</span></summary>
                <p class="muted-line">{syncSummary.baselineAvailable ? $ui("只传有变化的文件，自动合并其他设备的改动。") : $ui("首次合并保留两边独有的文件，同名冲突逐个让你选。")}</p>
                <dl>{#each syncSummary.categories as category}<div><dt>{$ui(scopeLabels[category.id])}</dt><dd>{$ui("{p0} 个文件 · {p1}", { p0: category.fileCount, p1: formatBytes(category.totalBytes) })}</dd></div>{/each}</dl>
                <p class="muted-line">{$ui("包含重定向和新增模块，不含依赖、生成目录、缓存和敏感文件。")}</p>
              </details>
            {:else if summaryBusy}<p class="muted-line" role="status">{$ui("正在统计站点源码…")}</p>{/if}
            <div class="sync-publish-row"><div class="setting-copy"><strong>{$ui("发布网站")}</strong><span>{$ui("同步只传源码，线上生效还需要发布。")}</span></div><button class="button" type="button" disabled={syncBusy || taskBusy || syncStatus.status === "conflict" || syncStatus.status === "remoteAhead"} on:click={publishSite}>{$ui("发布网站")}</button></div>
          </div>
          {/if}
          <div class="settings-block sync-connection-block">
            {#if syncStatus.enabled}
              <div class="provider-tabs" role="tablist" aria-label={$ui("同步方式")}>
                <button type="button" role="tab" class="provider-tab" aria-selected={setupProvider === "github"} class:active={setupProvider === "github"} disabled={syncBusy} on:click={() => chooseConnectionProvider("github")}>GitHub</button>
                <button type="button" role="tab" class="provider-tab" aria-selected={setupProvider === "webdav"} class:active={setupProvider === "webdav"} disabled={syncBusy} on:click={() => chooseConnectionProvider("webdav")}>WebDAV</button>
              </div>
              {#if switchingProvider}<p class="provider-switch-hint">{$ui("切换后本机改用新通道同步，旧通道上的文件不会被删除。")}</p>{/if}
            {:else}
              <div class="settings-block-heading">
                <h3>{syncProvider === "github" ? $ui("设置 GitHub") : $ui("设置 WebDAV")}</h3>
                <p>{$ui("先完成检查再启用，不会直接覆盖任何一边。")}</p>
              </div>
            {/if}
            {#if syncStatus.enabled && !switchingProvider && setupProvider === "github"}
              <details class="settings-disclosure"><summary>{$ui("连接详情")} <span>GitHub</span></summary><p class="sync-location">{syncStatus.repository} · {syncStatus.branch}</p><p class="muted-line">{$ui("保存后会自动同步，也可手动立即同步。")}</p></details>
            {:else if setupProvider === "github"}
              {#if !syncCandidates.length}
                <p class="muted-line">{syncStatus.enabled ? $ui("项目里没有可同步的 GitHub 仓库。") : $ui("没找到 GitHub Pages 或 deploy 仓库，也可以改用 WebDAV。")}</p>
              {:else}
                {#if syncCandidates.length > 1}
                  <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-20">{$ui("目标仓库")}</label><span id="setting-field-20-hint">{$ui("选一个存源码的仓库。")}</span></div><select id="setting-field-20" aria-describedby="setting-field-20-hint" aria-label={$ui("目标仓库")} class="select compact-control" disabled={syncBusy} value={syncCandidate?.repository ?? ""} on:change={(event) => { syncCandidate = syncCandidates.find((item) => item.repository === event.currentTarget.value) ?? null; syncPreflight = null; publicAcknowledged = false; }}><option value="" disabled>{$ui("请选择仓库")}</option>{#each syncCandidates as candidate}<option value={candidate.repository}>{candidate.repository}</option>{/each}</select></div>
                {/if}
                {#if !syncCandidate}
                  <p class="muted-line">{$ui("先选仓库，才能预检和启用同步。")}</p>
                {:else}
                  <div class="sync-summary"><strong>{syncCandidate.repository}</strong><span>{syncCandidate.source} {$ui("· 仓库可见性：")}{syncCandidate.visibility === "public" ? $ui("公开") : syncCandidate.visibility === "private" ? $ui("私有") : $ui("未确认")}</span></div>
                  <details class="settings-disclosure sync-advanced-branch"><summary>{$ui("高级连接选项")}</summary><div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-21">{$ui("项目同步分支")}</label><span id="setting-field-21-hint">{$ui("独立保存源码，不影响网站发布分支。")}</span></div><input id="setting-field-21" aria-describedby="setting-field-21-hint" aria-label={$ui("内容分支")} class="input compact-control" value={syncBranch} disabled={syncBusy} on:input={(event) => { syncBranch = event.currentTarget.value; syncPreflight = null; }} /></div></details>
                  {#if syncCandidate.visibility === "public" || syncCandidate.visibility === "unknown"}<label class="sync-warning"><input type="checkbox" bind:checked={publicAcknowledged} /><span>{$ui("我同意把草稿、配置和主题传到公开仓库。")}</span></label>{/if}
                  {#if syncPreflight}<div class="sync-summary"><strong>{$ui("启用预检")}</strong><span>{$ui("本地 {p0} 个文件 · {p1} MB", { p0: syncPreflight.fileCount, p1: (syncPreflight.totalBytes / 1024 / 1024).toFixed(2) })}</span>{#if syncPreflight.remoteBranchExists && syncPreflight.remoteManifestValid}<span>{$ui("云端 {p0} 个文件 · {p1} MB", { p0: syncPreflight.remoteFileCount, p1: (syncPreflight.remoteTotalBytes / 1024 / 1024).toFixed(2) })}</span><span>{$ui("仅本地 {p0} · 仅云端 {p1} · 内容不同 {p2}", { p0: syncPreflight.localOnlyCount, p1: syncPreflight.remoteOnlyCount, p2: syncPreflight.differentCount })}</span>{:else}<span>{syncPreflight.remoteBranchExists ? $ui("云端分支没有合法清单，不能接管") : $ui("将创建新的孤立分支")}</span>{/if}</div>{/if}
                  <div class="button-row"><button class="button" type="button" disabled={syncBusy} on:click={preflightSync}>{syncBusy ? $ui("正在检查…") : $ui("检查连接与差异")}</button><button class="button primary" type="button" disabled={syncBusy || !syncPreflight || (syncPreflight.remoteBranchExists && !syncPreflight.remoteManifestValid) || ((syncCandidate.visibility === "public" || syncCandidate.visibility === "unknown") && !publicAcknowledged)} on:click={() => configureSync()}>{syncStatus.enabled ? $ui("切换到 GitHub 并合并") : $ui("合并并开始同步")}</button></div>
                {/if}
              {/if}
            {:else}
              <details class="sync-connection-details" open={!syncStatus.enabled || webDavConnectionOpen} on:toggle={(event) => { if (syncStatus.enabled) webDavConnectionOpen = event.currentTarget.open; }}>
                <summary>{$ui("WebDAV 连接设置")}{#if syncStatus.enabled}<span>{$ui("立即生效")}</span>{/if}</summary>
              <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-22">{$ui("服务器地址")}</label><span id="setting-field-22-hint">{$ui("更换地址后需重新测试。")}</span></div><input id="setting-field-22" aria-describedby="setting-field-22-hint" aria-label={$ui("WebDAV 服务器地址")} class="input compact-control" type="url" placeholder="https://dav.example.com/dav" value={webDavEndpoint} disabled={syncBusy} on:input={(event) => { webDavEndpoint = event.currentTarget.value; webDavCredential = { configured: false }; webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} on:blur={refreshWebDavCredential} /></div>
              <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-23">{$ui("云端目录")}</label><span id="setting-field-23-hint">{$ui("用于保存站点源码。")}</span></div><input id="setting-field-23" aria-describedby="setting-field-23-hint" aria-label={$ui("WebDAV 云端目录")} class="input compact-control" value={webDavRemoteDir} disabled={syncBusy} on:input={(event) => { webDavRemoteDir = event.currentTarget.value; webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} /></div>
              <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-24">{$ui("用户名")}</label><span id="setting-field-24-hint">{$ui("WebDAV 登录账号。")}</span></div><input id="setting-field-24" aria-describedby="setting-field-24-hint" aria-label={$ui("WebDAV 用户名")} class="input compact-control" autocomplete="username" bind:value={webDavUsername} disabled={syncBusy} on:input={() => { webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} /></div>
              <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-25">{$ui("密码")}</label><span id="setting-field-25-hint">{webDavCredential.configured ? $ui("留空使用已保存密码。") : $ui("测试成功后保存到系统凭据库。")}</span></div><input id="setting-field-25" aria-describedby="setting-field-25-hint" aria-label={$ui("WebDAV 密码")} class="input compact-control" type="password" autocomplete="current-password" bind:value={webDavPassword} disabled={syncBusy} on:input={() => { webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} /></div>

              {#if webDavConnectionDirty}<p class="sync-warning" role="status">{$ui("连接信息改了，重新测试并应用。")}</p>{/if}
              {#if syncStatus.status === "authRequired"}<p class="sync-warning" role="alert">{$ui("凭据验证不过。改一下用户名或密码，再重新测试。")}</p>{/if}
              {#if webDavConnectionError}<p class="sync-error" role="alert">{webDavConnectionError}</p>{/if}
              <div class="button-row"><button class="button" type="button" disabled={syncBusy || !webDavEndpoint.trim() || !webDavRemoteDir.trim() || !webDavUsername.trim() || (!webDavPassword && !webDavCredential.configured)} on:click={testWebDavConnection}>{syncBusy ? $ui("正在测试…") : $ui("保存并测试连接")}</button>{#if webDavCredential.configured}<button class="button danger" type="button" disabled={syncBusy} on:click={deleteWebDavCredential}>{$ui("删除凭据")}</button>{/if}</div>
              {#if webDavPreflight}<div class="sync-summary"><strong>{$ui("连接测试通过")}</strong><span>{webDavPreflight.endpoint}/{webDavPreflight.remoteDir}</span>{#if webDavTestedAt}<span>{$ui("验证时间：")}{new Date(webDavTestedAt).toLocaleString()}</span>{/if}<span>{$ui("本地 {p0} 个文件 · {p1} MB", { p0: webDavPreflight.fileCount, p1: (webDavPreflight.totalBytes / 1024 / 1024).toFixed(2) })}</span>{#if webDavPreflight.remoteExists && webDavPreflight.remoteManifestValid}<span>{$ui("云端 {p0} 个文件 · {p1} MB", { p0: webDavPreflight.remoteFileCount, p1: (webDavPreflight.remoteTotalBytes / 1024 / 1024).toFixed(2) })}</span><span>{$ui("仅本地 {p0} · 仅云端 {p1} · 内容不同 {p2}", { p0: webDavPreflight.localOnlyCount, p1: webDavPreflight.remoteOnlyCount, p2: webDavPreflight.differentCount })}</span>{:else}<span>{webDavPreflight.remoteExists ? $ui("云端目录没有合法清单，不能接管") : $ui("将初始化新的 WebDAV 云端目录")}</span>{/if}</div>{/if}
              {#if syncStatus.enabled && syncStatus.provider === "webdav"}
                {#if webDavConnectionDirty}<div class="button-row"><button class="button primary" type="button" disabled={syncBusy || !webDavTestMatches || (webDavPreflight?.remoteExists && !webDavPreflight.remoteManifestValid)} on:click={applyWebDavConnection}>{$ui("应用连接设置")}</button></div>{/if}
              {:else}
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy || !webDavTestMatches || !webDavPreflight || (webDavPreflight.remoteExists && !webDavPreflight.remoteManifestValid)} on:click={() => configureSync()}>{syncStatus.enabled ? $ui("切换到 WebDAV 并合并") : $ui("合并并开始同步")}</button></div>
              {/if}
              </details>
            {/if}

          </div>
          {#if syncStatus.enabled}
            <details class="settings-block settings-disclosure sync-danger-zone"><summary>{$ui("高级操作")}</summary><p class="muted-line">{$ui("覆盖会丢掉一边的修改，合并同步不会。")}</p>
              <div class="button-row"><button class="button danger" type="button" disabled={syncBusy || webDavConnectionDirty || syncStatus.requiresScopeConfirmation} on:click={() => (pendingSyncOverwrite = "overwriteLocal")}>{$ui("用云端项目覆盖本机")}</button><button class="button danger" type="button" disabled={syncBusy || webDavConnectionDirty || syncStatus.requiresScopeConfirmation} on:click={() => (pendingSyncOverwrite = "overwriteRemote")}>{$ui("用本机项目覆盖云端")}</button><button class="button" type="button" disabled={syncBusy} on:click={() => session && platform.openContentSyncBackups(session.projectId, session.generation)}>{$ui("打开备份目录")}</button><button class="button danger" type="button" disabled={syncBusy} on:click={disableSync}>{$ui("关闭同步")}</button></div>
            </details>
          {/if}
          {/if}
        {/if}
      {:else}
        <div class="settings-block">
          <div class="setting-subsection-heading"><h3>{$ui("应用更新")}</h3><button class="button" type="button" on:click={onOpenUpdates}>{$ui("查看更新")}</button></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-26">{$ui("启动时检查更新")}</label><span id="setting-field-26-hint">{$ui("每天查一次，不弹窗。")}</span></div><label class="switch"><input id="setting-field-26" aria-describedby="setting-field-26-hint" type="checkbox" checked={draft.update.checkOnStart} on:change={(event) => change({ ...draft, update: { ...draft.update, checkOnStart: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row setting-row-dependent" class:disabled={!draft.update.checkOnStart}><div class="setting-copy"><label class="setting-title" for="setting-auto-download">{$ui("后台下载更新")}</label><span id="setting-auto-download-hint">{$ui("下载完成后由你点安装。")}</span></div><label class="switch"><input id="setting-auto-download" aria-describedby="setting-auto-download-hint" type="checkbox" disabled={!draft.update.checkOnStart} checked={draft.update.autoDownload} on:change={(event) => change({ ...draft, update: { ...draft.update, autoDownload: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <details class="settings-block settings-disclosure">
          <summary>{$ui("恢复设置")}</summary>
          <div class="setting-row"><div class="setting-copy"><strong>{$ui("恢复默认设置")}</strong><span>{$ui("只重置常规设置，Token 保留。")}</span></div><button class="button" type="button" on:click={() => (showReset = true)}><RotateCcw size={14} />{$ui("恢复默认")}</button></div>
        </details>
      {/if}
    </section>
  </div>
</div>

{#if showAcquireToken}
  <ModalDialog title={$ui("获取 Cloudflare-ImgBed Token")} description={$ui("仅用于本次登录和创建 Token，不会写进配置或日志。")} onClose={closeAcquireToken}>
    <div class="modal-form">
      <label><span>{$ui("管理员用户名")}</span><input class="input" data-autofocus autocomplete="username" bind:value={adminUsername} placeholder={$ui("按服务端配置填写，可留空")} /></label>
      <label><span>{$ui("管理员密码")}</span><input class="input" type="password" autocomplete="current-password" bind:value={adminPassword} placeholder={$ui("按服务端配置填写，可留空")} /></label>
      {#if tokenStatusMessage}<p class="modal-status" role="status">{tokenStatusMessage}</p>{/if}
    </div>
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={credentialBusy} on:click={closeAcquireToken}>{$ui("取消")}</button><button class="button primary" type="button" disabled={credentialBusy} on:click={acquireToken}><KeyRound size={14} />{credentialBusy ? $ui("正在获取 Token…") : $ui("获取并保存")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if showReset}
  <ModalDialog title={$ui("恢复默认设置？")} description={$ui("恢复默认会立即生效；凭据库里的 Token 不受影响。")} onClose={() => (showReset = false)}>
    <svelte:fragment slot="actions"><button class="button" type="button" data-autofocus on:click={() => (showReset = false)}>{$ui("取消")}</button><button class="button danger" type="button" on:click={restoreDefaults}>{$ui("恢复默认")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if showClearRecent}
  <ModalDialog title={$ui("清空最近项目？")} description={$ui("只清掉记录，磁盘上的博客文件不动。")} onClose={() => (showClearRecent = false)}>
    <svelte:fragment slot="actions"><button class="button" type="button" data-autofocus on:click={() => (showClearRecent = false)}>{$ui("取消")}</button><button class="button danger" type="button" on:click={clearRecent}>{$ui("清空记录")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if pendingSyncOverwrite}
  <ModalDialog
    title={pendingSyncOverwrite === "overwriteLocal" ? $ui("用云端覆盖本机？") : $ui("用本机项目覆盖云端？")}
    description={pendingSyncOverwrite === "overwriteLocal"
      ? $ui("云端会覆盖同名文件；云端删掉的，本地也会删。会先做本地备份。")
      : $ui("云端会更新到和本机一致。其他设备没上传的改动会被覆盖。")}
    onClose={() => (pendingSyncOverwrite = null)}
  >
    <svelte:fragment slot="actions"><button class="button" type="button" data-autofocus on:click={() => (pendingSyncOverwrite = null)}>{$ui("取消")}</button><button class="button danger" type="button" on:click={confirmSyncOverwrite}>{$ui("确认覆盖")}</button></svelte:fragment>
  </ModalDialog>
{/if}

<style>
  .setting-title { font-weight: 600; cursor: pointer; }
  .sync-decisions { border: 0; margin: 0; padding: 0; min-width: 0; }
  .sync-plan { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 10px; margin: 16px 0 0; padding: 0; list-style: none; }
  .sync-plan li { display: grid; grid-template-columns: auto minmax(0, 1fr); align-items: start; gap: 10px; min-width: 0; padding: 13px; border-radius: 10px; background: var(--bg-control); }
  .sync-plan li > span { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 7px; color: var(--accent); background: var(--accent-soft); font-size: 11px; font-weight: 700; font-variant-numeric: tabular-nums; }
  .sync-plan strong { display: block; margin: 2px 0 4px; font-size: 12px; }
  .sync-plan p { margin: 0; color: var(--text-secondary); font-size: 11px; line-height: 1.55; }
  .sync-loading { color: var(--text-secondary); font-size: 12px; }
  .sync-progress { display: grid; gap: 8px; padding: 14px; border: 1px solid var(--border-subtle); border-radius: 10px; margin-block: 12px; overflow-wrap: anywhere; }
  .sync-progress progress { width: 100%; }
  .sync-progress span { font-size: 12px; color: var(--text-secondary); }
  .sync-progress .button { justify-self: start; }
  .sync-connection-details { margin-block: 12px; }
  .provider-tabs { display: inline-grid; grid-auto-flow: column; gap: 4px; padding: 4px; border-radius: 11px; background: var(--bg-control); }
  .provider-tabs .provider-tab { border: 0; border-radius: 8px; padding: 7px 22px; background: transparent; color: var(--text-secondary); font: inherit; font-size: 13px; font-weight: 650; cursor: pointer; transition: background var(--duration-fast) ease, color var(--duration-fast) ease; }
  .provider-tabs .provider-tab:hover:not(:disabled) { color: var(--text-primary); }
  .provider-tabs .provider-tab.active { background: var(--bg-panel); color: var(--text-primary); box-shadow: var(--shadow-popover); }
  .provider-tabs .provider-tab:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
  .provider-tabs .provider-tab:disabled { cursor: default; opacity: 0.6; }
  .provider-switch-hint { margin: 12px 0 0; color: var(--text-secondary); font-size: 12px; }
  .sync-connection-details > summary { padding-block: 10px; cursor: pointer; font-weight: 600; }
  .sync-connection-details > summary span { margin-left: 12px; font-weight: 400; font-size: 12px; color: var(--text-secondary); }
  .settings-disclosure > summary { cursor: pointer; padding-block: 4px; font-size: 13px; font-weight: 600; }
  .settings-disclosure > summary > span { margin-left: 8px; color: var(--text-secondary); font-size: 12px; font-weight: 400; }
  .settings-disclosure[open] > summary { margin-bottom: 14px; }
  .settings-disclosure > summary:focus-visible, .sync-connection-details > summary:focus-visible { outline: 2px solid var(--accent); outline-offset: 4px; border-radius: 4px; }
  .sync-state-heading { display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 12px; margin-bottom: 18px; }
  .sync-state-heading > div { display: flex; flex-wrap: wrap; align-items: center; gap: 12px; }
  .sync-state-heading h3 { margin: 0; font-size: 16px; }
  .sync-provider { color: var(--text-secondary); font-size: 12px; }
  .sync-change-summary { margin: 14px 0 6px; font-size: 14px; font-variant-numeric: tabular-nums; }
  .sync-scope { margin-top: 20px; }
  .sync-scope dl { margin: 0; }
  .sync-scope dl > div { display: flex; justify-content: space-between; gap: 16px; padding-block: 7px; font-size: 12px; }
  .sync-scope dd { margin: 0; color: var(--text-secondary); font-variant-numeric: tabular-nums; }
  .sync-publish-row { display: flex; align-items: center; justify-content: space-between; gap: 20px; border-top: 1px solid var(--border-subtle); margin-top: 20px; padding-top: 20px; }
  .sync-publish-row .button { flex-shrink: 0; }
  .sync-location { overflow-wrap: anywhere; font-size: 13px; }
  .sync-advanced-branch { padding-block: 10px; }
  .publish-sequence { margin: 12px 0 16px; }
  @media (max-width: 660px) {
    .sync-plan { grid-template-columns: 1fr; }
    .sync-publish-row { align-items: stretch; flex-direction: column; gap: 12px; }
    .sync-scope dl > div { flex-wrap: wrap; gap: 4px 12px; }
    .sync-connection-details > summary span { display: block; margin: 4px 0 0; }
  }
</style>
