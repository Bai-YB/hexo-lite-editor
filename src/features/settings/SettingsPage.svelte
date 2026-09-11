<script lang="ts">
  import { ui } from "$shared/i18n/ui";
  import { onDestroy, onMount, tick } from "svelte";
  import { mergeSavedDraft, validateSettings } from "./settingsDraft";
  import { KeyRound, RotateCcw, Trash2 } from "@lucide/svelte";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import SettingsHeader from "./SettingsHeader.svelte";
  import SettingsNavigation from "./SettingsNavigation.svelte";
  import CloudflareImageBedSettings from "./CloudflareImageBedSettings.svelte";
  import { syncStatusLabel } from "$features/editor/syncStatusLabel";
  import { defaultConfig } from "$shared/types/app";
  import { normalizeError, platform } from "$platform/tauri";
  import { shortcutLabel } from "$platform/os";
  import type { SettingsController } from "./controller";
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
  export let onRegisterSettingsController: (controller: SettingsController | null) => void = () => {};
  export let onRemoveRecentProject: (recentId: string) => Promise<void> = async () => {};
  export let onClearRecentProjects: () => Promise<void> = async () => {};
  export let onBeforeSync: () => Promise<boolean> = async () => true;
  export let onNotice: (message: string) => void = () => {};

  const sectionStorageKey = "hexo-lite-editor:settings-active-section";
  const sections: Array<{ id: SettingsSectionId; title: string; description: string }> = [
    { id: "general", title: "常规", description: "启动、保存与项目记录" },
    { id: "editing", title: "编辑体验", description: "外观、排版与编辑辅助" },
    { id: "images", title: "图片与图床", description: "导入目标与连接状态" },
    { id: "hexoPublish", title: "Hexo 与发布", description: "浏览器预览与发布流水线" },
    { id: "sync", title: "内容同步", description: "GitHub 或 WebDAV" },
    { id: "maintenance", title: "维护", description: "更新与恢复" }
  ];

  let saved = structuredClone(config);
  let draft = structuredClone(config);
  let activeSection: SettingsSectionId = "general";
  let dirty = false;
  let saving = false;
  let pendingSave: Promise<void> | null = null;
  let saveError = "";
  let pageElement: HTMLDivElement;
  let disposed = false;
  let credentialRequest = 0;
  let syncRevision = 0;
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
  let pendingInitialChoice: "local" | "remote" | null = null;
  let pendingSyncOverwrite: "overwriteLocal" | "overwriteRemote" | null = null;
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

  $: dirty = JSON.stringify(draft) !== JSON.stringify(saved);
  $: currentSection = sections.find((section) => section.id === activeSection) ?? sections[0];
  $: webDavConnectionDirty = syncStatus.enabled && syncStatus.provider === "webdav"
    && (webDavEndpoint.trim().replace(/\/$/, "") !== (syncStatus.endpoint ?? "").replace(/\/$/, "")
      || webDavRemoteDir.trim().replace(/^\/+|\/+$/g, "") !== (syncStatus.remoteDir ?? ""));
  $: webDavTestMatches = Boolean(webDavPreflight)
    && webDavEndpoint.trim().replace(/\/$/, "") === webDavTestedEndpoint
    && webDavRemoteDir.trim().replace(/^\/+|\/+$/g, "") === webDavTestedRemoteDir;
  $: if (syncStatus.status === "authRequired" && syncStatus.provider === "webdav") webDavConnectionOpen = true;
  $: dirtySections = {
    general: JSON.stringify(draft.general) !== JSON.stringify(saved.general),
    editing: JSON.stringify([draft.appearance, draft.editor, draft.articleList]) !== JSON.stringify([saved.appearance, saved.editor, saved.articleList]),
    images: JSON.stringify(draft.imageBed) !== JSON.stringify(saved.imageBed),
    hexoPublish: JSON.stringify([draft.hexo, draft.publish]) !== JSON.stringify([saved.hexo, saved.publish]),
    sync: false,
    maintenance: JSON.stringify([draft.diagnostics, draft.update]) !== JSON.stringify([saved.diagnostics, saved.update])
  } satisfies Record<SettingsSectionId, boolean>;

  onMount(() => {
    const stored = localStorage.getItem(sectionStorageKey) as SettingsSectionId | null;
    activeSection = initialSection ?? (sections.some((section) => section.id === stored) ? stored! : "general");
    onRegisterSettingsController({ save: saveDraft, discard, hasDirty: () => dirty || saving });
    void refreshCredential();
    void platform.onContentSyncStatus((status) => {
      if (!session || disposed || status.projectId !== session.projectId || status.sessionGeneration !== session.generation) return;
      syncRevision += 1;
      syncStatus = status;
      if (!syncBusy) void refreshSyncConflicts().catch((error) => { if (!disposed) syncError = normalizeError(error).message; });
    }).then((unlisten) => { if (disposed) unlisten(); else unlistenSync = unlisten; })
      .catch((error) => { if (!disposed) onNotice(normalizeError(error).message); });
    void platform.onContentSyncPhase((event) => {
      if (!session || disposed || event.projectId !== session.projectId || event.sessionGeneration !== session.generation) return;
      if (["completed", "operationFinished", "failed", "attention", "waiting"].includes(event.phase)) {
        if (backgroundSyncBusy) {
          backgroundSyncBusy = false; endSync();
          if (syncStatus.status === "conflict") void refreshSyncConflicts().catch((error) => { if (!disposed) syncError = normalizeError(error).message; });
        }
        syncProgress = null; return;
      }
      if (!syncBusy) { beginSync(); backgroundSyncBusy = true; }
      syncProgress = event;
    }).then((unlisten) => { if (disposed) unlisten(); else { unlistenSyncPhase = unlisten; void refreshSync(); } })
      .catch((error) => { if (!disposed) syncError = normalizeError(error).message; });
    syncTimer = setInterval(() => { if (syncBusy) syncElapsed = Math.floor((Date.now() - syncStartedAt) / 1000); }, 1000);
  });

  onDestroy(() => {
    disposed = true;
    unlistenSync?.();
    unlistenSyncPhase?.();
    if (syncTimer) clearInterval(syncTimer);
    if (dirty) onThemePreview(saved.appearance.themeMode);
    onRegisterSettingsController(null);
  });

  function selectSection(section: SettingsSectionId) {
    activeSection = section;
    localStorage.setItem(sectionStorageKey, section);
  }

  function sameProject(identity: { projectId: string }) {
    return !disposed && session?.projectId === identity.projectId;
  }
  function sameSession(identity: { projectId: string; generation: number }) {
    return sameProject(identity) && session?.generation === identity.generation;
  }

  function beginSync(message = "正在准备同步操作。") {
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
    const identity = { ...session };
    const revision = syncRevision;
    try {
      const detection = await platform.detectContentSync(identity.projectId, identity.generation);
      const status = await platform.getContentSyncStatus(identity.projectId, identity.generation);
      const activeProgress = await platform.getContentSyncProgress(identity.projectId);
      if (!sameSession(identity)) return;
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
      if (webDavEndpoint) await refreshWebDavCredential();
      await refreshSyncConflicts();
    } catch (error) {
      onNotice(normalizeError(error).message);
    }
  }

  async function configureSync(initialChoice?: "local" | "remote") {
    if (!session || syncBusy) return;
    if (syncStatus.enabled && initialChoice) { await runSync(initialChoice); return; }
    if (syncProvider === "github" && !syncCandidate) return;
    const identity = { ...session };
    beginSync();
    try {
      let result: import("$shared/types/app").ContentSyncView;
      if (syncProvider === "github" && syncCandidate) {
        result = await platform.enableContentSync({
          projectId: session.projectId,
          sessionGeneration: session.generation,
          repository: syncCandidate.repository,
          branch: syncBranch,
          initialChoice,
          confirmPublic: syncCandidate.visibility !== "public" || publicAcknowledged
        });
      } else {
        result = await platform.enableWebDavContentSync({
          projectId: session.projectId,
          sessionGeneration: session.generation,
          endpoint: webDavEndpoint,
          remoteDir: webDavRemoteDir,
          initialChoice
        });
      }
      if (!sameProject(identity)) return;
      syncStatus = result;
      if (syncStatus.enabled) webDavConnectionOpen = false;
      onNotice(syncStatus.message || "内容同步设置已更新。");
    } catch (error) {
      syncError = normalizeError(error).message; onNotice(syncError);
    } finally {
      endSync();
    }
  }

  async function preflightSync() {
    if (!session || syncBusy || (syncProvider === "github" && !syncCandidate)) return;
    const identity = { ...session };
    const branch = syncBranch;
    const repository = syncCandidate?.repository;
    beginSync();
    try {
      if (syncProvider === "github" && syncCandidate) {
        const result = await platform.preflightContentSync(identity.projectId, identity.generation, syncCandidate.repository, branch);
        if (!sameSession(identity) || syncBranch !== branch || syncCandidate?.repository !== repository) return;
        syncPreflight = result;
        syncCandidate = syncPreflight.candidate;
      }
    } catch (error) {
      syncError = normalizeError(error).message; onNotice(syncError);
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
    beginSync();
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
        webDavConnectionError = "测试期间连接信息已修改，请重新测试当前输入。";
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
      onNotice("WebDAV 真实连接、读写权限和远端预检均已通过。");
    } catch (error) {
      webDavConnectionError = normalizeError(error).message;
      webDavPreflight = null;
      webDavTestedAt = "";
      onNotice(webDavConnectionError);
    } finally {
      endSync();
    }
  }

  async function applyWebDavConnection() {
    if (!session || syncBusy || !webDavTestMatches) return;
    const identity = { ...session };
    beginSync();
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
      onNotice(syncStatus.message || "WebDAV 连接设置已应用。");
    } catch (error) {
      webDavConnectionError = normalizeError(error).message;
      onNotice(webDavConnectionError);
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
      onNotice("WebDAV 凭据已删除。");
    } catch (error) {
      syncError = normalizeError(error).message; onNotice(syncError);
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
      onNotice(syncStatus.message || "冲突已解决。");
    } catch (error) {
      syncError = normalizeError(error).message;
      onNotice(syncError);
    } finally {
      endSync();
    }
  }

  async function runSync(direction: "auto" | "local" | "remote" | "overwriteLocal" | "overwriteRemote" = "auto") {
    if (!session || syncBusy) return;
    const identity = { ...session };
    if (!await onBeforeSync() || !sameSession(identity) || syncBusy) return;
    beginSync();
    syncError = "";
    try {
      const result = await platform.runContentSync(identity.projectId, identity.generation, direction);
      if (!sameProject(identity)) return;
      syncStatus = result;
      await refreshSyncConflicts();
      onNotice(syncStatus.message || "同步检查完成。");
    } catch (error) {
      syncError = normalizeError(error).message;
      onNotice(syncError);
    } finally {
      endSync();
    }
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
      onNotice("内容同步已关闭，本地文章不会被删除。");
    } catch (error) {
      syncError = normalizeError(error).message; onNotice(syncError);
    } finally {
      endSync();
    }
  }

  async function reconnectSync() {
    if (!session || syncBusy) return;
    beginSync();
    try {
      syncStatus = await platform.reconnectContentSync(session.projectId, session.generation);
      onNotice(syncStatus.message || "系统 Git 认证检查完成。");
    } catch (error) {
      syncError = normalizeError(error).message; onNotice(syncError);
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
    requestAnimationFrame(() => document.querySelector<HTMLButtonElement>(`[data-settings-section="${sections[nextIndex].id}"]`)?.focus());
  }

  function change(next: AppConfigV3) {
    const previousLanguage = draft.general.language;
    draft = next;
    onThemePreview(next.appearance.themeMode);
    saveError = "";
    if (next.general.language !== previousLanguage) setLanguage(next.general.language);
  }

  async function persistConfig(nextConfig: AppConfigV3, message = "设置已保存。") {
    if (pendingSave) await pendingSave;
    const submitted = structuredClone(nextConfig);
    const validation = validateSettings(submitted);
    if (validation) {
      saveError = validation.message;
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
          onNotice(message);
        }
      } catch (error) {
        saveError = normalizeError(error).message;
        if (!disposed) onNotice(saveError);
        throw error;
      } finally { saving = false; }
    })();
    pendingSave = operation;
    try { await operation; }
    finally { if (pendingSave === operation) pendingSave = null; }
  }

  async function saveDraft() {
    if (pendingSave) await pendingSave;
    if (JSON.stringify(draft) !== JSON.stringify(saved)) await persistConfig(draft);
    if (JSON.stringify(draft) !== JSON.stringify(saved)) {
      throw new Error("保存期间又有设置变化，请再次保存后继续。");
    }
  }

  async function saveFromButton() {
    try { await saveDraft(); }
    catch (error) { saveError = normalizeError(error).message; }
  }

  async function persistImageBed(patch: Partial<AppConfigV3["imageBed"]>, message: string) {
    if (pendingSave) await pendingSave;
    draft = { ...draft, imageBed: { ...draft.imageBed, ...patch } };
    await persistConfig({ ...saved, imageBed: { ...saved.imageBed, ...patch } }, message);
  }

  async function discard() {
    if (pendingSave) await pendingSave.catch(() => {});
    saveError = "";
    draft = structuredClone(saved);
    onThemePreview(saved.appearance.themeMode);
    setLanguage(saved.general.language);
  }

  function restoreDefaults() {
    draft = structuredClone(defaultConfig);
    onThemePreview(draft.appearance.themeMode);
    setLanguage(draft.general.language);
    showReset = false;
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
      onNotice("请先填写 Cloudflare-ImgBed 服务地址。");
      return;
    }
    credentialBusy = true;
    try {
      credential = await platform.credentialMigrate(
        draft.imageBed.cloudflareConnectionId,
        draft.imageBed.cloudflareApiUrl
      );
      legacyCredentialAvailable = false;
      tokenStatusMessage = "旧版 Token 已绑定到当前连接；旧凭据已从临时命名空间移除。";
    } catch (error) {
      tokenStatusMessage = normalizeError(error).message;
    } finally {
      credentialBusy = false;
    }
  }

  async function prepareAcquireToken() {
    if (credentialBusy) return;
    if (!draft.imageBed.cloudflareApiUrl.trim()) return onNotice("请先填写 Cloudflare-ImgBed 服务地址。");
    credentialBusy = true;
    try {
      await persistImageBed({
        cloudflareApiUrl: draft.imageBed.cloudflareApiUrl,
        cloudflareConnectionId: draft.imageBed.cloudflareConnectionId
      }, "图床连接地址已保存；其他设置仍保留在草稿中。");
      showAcquireToken = true;
      tokenStatusMessage = "";
    } catch (error) { onNotice(normalizeError(error).message); }
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
    tokenStatusMessage = "正在获取 Token...";
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
      await persistImageBed({ cloudflareTokenId: result.tokenId }, "Token 已创建并保存到系统凭据库。");
      tokenStatusMessage = "Token 已配置";
      showAcquireToken = false;
      adminUsername = "";
    } catch (error) { tokenStatusMessage = normalizeError(error).message; }
    finally { adminPassword = ""; credentialBusy = false; }
  }

  async function testCredential() {
    if (credentialBusy) return;
    credentialBusy = true;
    tokenStatusMessage = "正在测试连接...";
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
      await persistImageBed({ cloudflareTokenId: undefined }, "Cloudflare Token 已从系统凭据库删除。");
      tokenStatusMessage = "本地 Token 已删除";
    } catch (error) { onNotice(normalizeError(error).message); }
    finally { credentialBusy = false; }
  }

  async function removeRecent(recentId: string) {
    try { await onRemoveRecentProject(recentId); }
    catch (error) { onNotice(normalizeError(error).message); }
  }

  async function clearRecent() {
    try { await onClearRecentProjects(); showClearRecent = false; }
    catch (error) { onNotice(normalizeError(error).message); }
  }

</script>

<div class="workspace-page settings-page" bind:this={pageElement}>
  <SettingsHeader {dirty} {saving} onDiscard={discard} onSave={saveFromButton} />
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
          <div class="settings-block-heading"><h3>{$translate("settings.languageTitle")}</h3><p>{$translate("settings.languageDescription")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-1">{$translate("settings.languageTitle")}</label><span id="setting-field-1-hint">{$translate("settings.languageDescription")}</span></div><select id="setting-field-1" aria-describedby="setting-field-1-hint" class="select compact-control" aria-label={$translate("settings.languageTitle")} value={draft.general.language} on:change={(event) => change({ ...draft, general: { ...draft.general, language: event.currentTarget.value as AppConfigV3["general"]["language"] } })}><option value="system">{$translate("settings.languageSystem")}</option><option value="zh-CN">{$translate("settings.languageChinese")}</option><option value="en-US">{$translate("settings.languageEnglish")}</option></select></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("启动")}</h3><p>{$ui("控制应用进入工作区时的恢复行为。")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-2">{$ui("启动时打开最近项目")}</label><span id="setting-field-2-hint">{$ui("只恢复上次经过验证的 Hexo 项目。")}</span></div><label class="switch"><input id="setting-field-2" aria-describedby="setting-field-2-hint" type="checkbox" checked={draft.general.openRecentProjectOnStart} on:change={(event) => change({ ...draft, general: { ...draft.general, openRecentProjectOnStart: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("保存与备份")}</h3><p>{$ui("减少输入中断，同时保留明确的安全边界。")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-3">{$ui("自动保存")}</label><span id="setting-field-3-hint">{$ui("按文章、revision 和项目会话串行保存。")}</span></div><label class="switch"><input id="setting-field-3" aria-describedby="setting-field-3-hint" type="checkbox" checked={draft.general.autoSave} on:change={(event) => change({ ...draft, general: { ...draft.general, autoSave: event.currentTarget.checked } })} /><span></span></label></div>
          <div class:disabled={!draft.general.autoSave} class="setting-row setting-row-dependent"><div class="setting-copy"><label class="setting-title" for="setting-field-4">{$ui("自动保存延迟")}</label><span id="setting-field-4-hint">{$ui("停止输入后等待的毫秒数。")}</span></div><input data-config-field="autoSaveDelayMs" id="setting-field-4" aria-describedby="setting-field-4-hint" class="input compact-control" disabled={!draft.general.autoSave} type="number" min="500" max="30000" step="100" value={draft.general.autoSaveDelayMs} on:change={(event) => change({ ...draft, general: { ...draft.general, autoSaveDelayMs: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-5">{$ui("保存前创建备份")}</label><span id="setting-field-5-hint">{$ui("在 .hlex-backups 中保留上一版本。")}</span></div><label class="switch"><input id="setting-field-5" aria-describedby="setting-field-5-hint" type="checkbox" checked={draft.general.backupBeforeSave} on:change={(event) => change({ ...draft, general: { ...draft.general, backupBeforeSave: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <div class="settings-block">
          <div class="setting-subsection-heading"><div><h3>{$ui("最近项目")}</h3><span>{$ui("最多保留 10 个项目，路径仅由后端管理。")}</span></div>{#if recentProjects.length}<button class="button danger" type="button" on:click={() => (showClearRecent = true)}>{$ui("清空")}</button>{/if}</div>
          {#if recentProjects.length}<div class="recent-project-list">{#each recentProjects as recent (recent.recentId)}<div class="recent-project-row"><div><strong>{recent.name}</strong><span>{recent.displayPath}</span></div><span class:warning={!recent.available} class="recent-availability">{recent.available ? $ui("可用") : $ui("不可用")}</span><button class="icon-button" type="button" title={$ui("移除记录")} aria-label={$ui("移除 {p0}", { p0: recent.name })} on:click={() => removeRecent(recent.recentId)}><Trash2 size={15} /></button></div>{/each}</div>{:else}<p class="muted-line">{$ui("尚无最近项目。")}</p>{/if}
        </div>
      {:else if activeSection === "editing"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("外观")}</h3><p>{$ui("主题切换会立即预览，取消后恢复。")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-6">{$ui("主题模式")}</label><span id="setting-field-6-hint">{$ui("浅色、深色或跟随系统。")}</span></div><select id="setting-field-6" aria-describedby="setting-field-6-hint" class="select compact-control" value={draft.appearance.themeMode} on:change={(event) => change({ ...draft, appearance: { themeMode: event.currentTarget.value as ThemeMode } })}><option value="system">{$ui("跟随系统")}</option><option value="light">{$ui("浅色")}</option><option value="dark">{$ui("深色")}</option></select></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("正文排版")}</h3><p>{$ui("统一编辑器的阅读密度和缩进节奏。")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-7">{$ui("字号")}</label><span id="setting-field-7-hint">12–28 px。</span></div><input data-config-field="fontSize" id="setting-field-7" aria-describedby="setting-field-7-hint" class="input compact-control" type="number" min="12" max="28" value={draft.editor.fontSize} on:change={(event) => change({ ...draft, editor: { ...draft.editor, fontSize: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-8">{$ui("行高")}</label><span id="setting-field-8-hint">1.2–2.2。</span></div><input data-config-field="lineHeight" id="setting-field-8" aria-describedby="setting-field-8-hint" class="input compact-control" type="number" min="1.2" max="2.2" step="0.05" value={draft.editor.lineHeight} on:change={(event) => change({ ...draft, editor: { ...draft.editor, lineHeight: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-9">{$ui("Tab 宽度")}</label><span id="setting-field-9-hint">{$ui("使用 2、4 或 8 个空格。")}</span></div><select id="setting-field-9" aria-describedby="setting-field-9-hint" class="select compact-control" value={draft.editor.tabSize} on:change={(event) => change({ ...draft, editor: { ...draft.editor, tabSize: Number(event.currentTarget.value) } })}><option value="2">2</option><option value="4">4</option><option value="8">8</option></select></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("编辑辅助")}</h3><p>{$ui("只保留写作过程中持续有用的视觉提示。")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-10">{$ui("显示行号")}</label><span id="setting-field-10-hint">{$ui("在正文左侧显示行号栏。")}</span></div><label class="switch"><input id="setting-field-10" aria-describedby="setting-field-10-hint" type="checkbox" checked={draft.editor.showLineNumbers} on:change={(event) => change({ ...draft, editor: { ...draft.editor, showLineNumbers: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-11">{$ui("自动换行")}</label><span id="setting-field-11-hint">{$ui("长行按编辑区宽度折行。")}</span></div><label class="switch"><input id="setting-field-11" aria-describedby="setting-field-11-hint" type="checkbox" checked={draft.editor.lineWrapping} on:change={(event) => change({ ...draft, editor: { ...draft.editor, lineWrapping: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-12">{$ui("突出当前行")}</label><span id="setting-field-12-hint">{$ui("使用低对比背景标识光标行。")}</span></div><label class="switch"><input id="setting-field-12" aria-describedby="setting-field-12-hint" type="checkbox" checked={draft.editor.highlightActiveLine} on:change={(event) => change({ ...draft, editor: { ...draft.editor, highlightActiveLine: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-13">{$ui("文章列表封面")}</label><span id="setting-field-13-hint">{$ui("在文章标题左侧显示缩略图。")}</span></div><label class="switch"><input id="setting-field-13" aria-describedby="setting-field-13-hint" type="checkbox" checked={draft.articleList.showCover} on:change={(event) => change({ ...draft, articleList: { showCover: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
      {:else if activeSection === "images"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("图片工作流")}</h3><p>{$ui("决定导入、粘贴和拖入图片时的目标。")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-14">{$ui("默认来源")}</label><span id="setting-field-14-hint">{$ui("本地项目目录、Cloudflare-ImgBed 或已启用的插件图床。")}</span></div><select id="setting-field-14" aria-describedby="setting-field-14-hint" class="select compact-control" value={draft.imageBed.defaultProvider} on:change={(event) => change({ ...draft, imageBed: { ...draft.imageBed, defaultProvider: event.currentTarget.value as AppConfigV3["imageBed"]["defaultProvider"] } })}><option value="local">{$ui("本地图片")}</option><option value="cloudflare-imgbed">Cloudflare-ImgBed</option>{#if draft.imageBed.defaultProvider.startsWith("plugin:")}<option value={draft.imageBed.defaultProvider}>{$ui("插件图床")}</option>{/if}</select></div>
          <div class="setting-row"><div class="setting-copy"><strong>{$ui("图片插入方式")}</strong><span>{$ui("粘贴或拖入后立即插入本地图片，图床上传成功后自动更新地址。")}</span></div><span class="muted-line">{$ui("自动")}</span></div>
        </div>
        {#if draft.imageBed.defaultProvider !== "local"}
          <div class="settings-block provider-block">
            <div class="settings-block-heading"><h3>{draft.imageBed.defaultProvider === "cloudflare-imgbed" ? $ui("Cloudflare 连接") : $ui("插件图床")}</h3><p>{draft.imageBed.defaultProvider === "cloudflare-imgbed" ? $ui("连接信息、凭据状态和操作集中管理。") : $ui("插件设置和连接测试由插件管理器提供。")}</p></div>
            {#if draft.imageBed.defaultProvider === "cloudflare-imgbed"}
            <CloudflareImageBedSettings settings={draft.imageBed} {credential} {legacyCredentialAvailable} busy={credentialBusy} statusMessage={tokenStatusMessage} onChange={updateImageBed} onAcquireToken={prepareAcquireToken} onMigrateLegacyToken={migrateLegacyCredential} onTestConnection={testCredential} onDeleteToken={deleteCredential} />
            {:else}
              <p class="muted-line">{$ui("当前插件已接入编辑器的粘贴、文件选择上传链路。")}</p>
            {/if}
          </div>
        {/if}
      {:else if activeSection === "hexoPublish"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("浏览器预览")}</h3><p>{$ui("软件内不嵌入主题页面；真实 Hexo 页面在系统浏览器打开。")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-15">{$ui("预览端口")}</label><span id="setting-field-15-hint">{$ui("默认使用 4000。")}</span></div><input data-config-field="previewPort" id="setting-field-15" aria-describedby="setting-field-15-hint" class="input compact-control" type="number" min="300" max="65535" value={draft.hexo.previewPort} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, previewPort: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-16">{$ui("打开项目后自动启动预览")}</label><span id="setting-field-16-hint">{$ui("只在后台启动 Hexo Server，不自动弹出浏览器。")}</span></div><label class="switch"><input id="setting-field-16" aria-describedby="setting-field-16-hint" type="checkbox" checked={draft.hexo.autoStartPreview} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, autoStartPreview: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-17">{$ui("预览草稿")}</label><span id="setting-field-17-hint">{$ui("启动 Hexo Server 时使用固定的 --draft 参数。")}</span></div><label class="switch"><input id="setting-field-17" aria-describedby="setting-field-17-hint" type="checkbox" checked={draft.hexo.previewDrafts} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, previewDrafts: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("发布流水线")}</h3><p>{$ui("发布快捷键为")} {shortcutLabel("⇧P")}。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>{$ui("发布前保存")}</strong><span>{$ui("始终先保存当前文章；保存失败时不会继续发布。")}</span></div><span class="muted-line">{$ui("已启用")}</span></div>
          <div class="setting-row"><div class="setting-copy"><strong>{$ui("始终重新生成")}</strong><span>{$ui("每次发布固定执行“清理缓存 → 重新生成 → 部署”，避免发布旧版本。")}</span></div><span class="muted-line">{$ui("已启用")}</span></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-18">{$ui("部署后 Git Push")}</label><span id="setting-field-18-hint">{$ui("执行固定 git push，不接受自定义参数。")}</span></div><label class="switch"><input id="setting-field-18" aria-describedby="setting-field-18-hint" type="checkbox" checked={draft.publish.gitPushAfterDeploy} on:change={(event) => change({ ...draft, publish: { ...draft.publish, gitPushAfterDeploy: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
      {:else if activeSection === "sync"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("项目同步")}</h3><p>{$ui("将 Hexo 项目源文件、文章、草稿、主题和配置同步到 GitHub 独立分支，或你自己的 WebDAV 服务器。")}</p></div>
          {#if !session}
            <p class="muted-line">{$ui("请先打开一个 Hexo 项目。")}</p>
          {:else}
            <ol class="sync-workflow" aria-label={$ui("同步步骤")}>
              <li class:active={!syncStatus.enabled}>{$ui("1. 连接并检查")}</li>
              <li class:active={syncStatus.enabled && !syncStatus.lastSyncedAt}>{$ui("2. 选择首次同步方向")}</li>
              <li class:active={Boolean(syncStatus.lastSyncedAt)}>{$ui("3. 自动上传后续保存")}</li>
            </ol>
            <p class="muted-line">{$ui("连接测试只检查权限。首次上传需要确认方向；之后保存的改动会在 30 秒后自动同步，也可手动立即上传。")}</p>
            {#if syncError}<p class="sync-error" role="alert">{syncError}</p>{/if}
            {#if syncBusy}
              <div class="sync-progress" role="status" aria-live="polite">
                <strong>{syncStopping ? $ui("正在停止同步...") : syncProgress?.message || $ui("正在准备同步操作。")}</strong>
                {#if syncProgress?.totalFiles != null && syncProgress.totalFiles > 0}
                  <progress max={syncProgress.totalFiles} value={syncProgress.completedFiles ?? 0} aria-label={$ui("同步文件进度")}></progress>
                  <span>{syncProgress.completedFiles ?? 0} / {syncProgress.totalFiles} {$ui("个文件")}</span>
                {/if}
                <span>{$ui("已用时")} {syncElapsed} s · {$ui("单次网络请求最长等待 30 秒")}</span>
                <button class="button" type="button" disabled={syncStopping} on:click={stopSync}>{syncStopping ? $ui("等待当前请求结束") : $ui("停止同步")}</button>
              </div>
            {/if}
            {#if !syncStatus.enabled}
              <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-19">{$ui("同步方式")}</label><span id="setting-field-19-hint">{$ui("选择 GitHub 或任意兼容 WebDAV 的服务器。")}</span></div><select id="setting-field-19" aria-describedby="setting-field-19-hint" aria-label={$ui("同步方式")} class="select compact-control" disabled={syncBusy} bind:value={syncProvider} on:change={() => { syncPreflight = null; webDavPreflight = null; }}><option value="github">GitHub</option><option value="webdav">WebDAV</option></select></div>
            {/if}
            {#if !syncStatus.enabled && syncProvider === "github"}
              {#if !syncCandidates.length}
                <p class="muted-line">{$ui("没有检测到 GitHub Pages 或 GitHub deploy 仓库。你仍可改用 WebDAV。")}</p>
              {:else}
                {#if syncCandidates.length > 1}
                  <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-20">{$ui("目标仓库")}</label><span id="setting-field-20-hint">{$ui("检测到多个 Git deploy 仓库，请明确选择。")}</span></div><select id="setting-field-20" aria-describedby="setting-field-20-hint" aria-label={$ui("目标仓库")} class="select compact-control" disabled={syncBusy} value={syncCandidate?.repository ?? ""} on:change={(event) => { syncCandidate = syncCandidates.find((item) => item.repository === event.currentTarget.value) ?? null; syncPreflight = null; publicAcknowledged = false; }}><option value="" disabled>{$ui("请选择仓库")}</option>{#each syncCandidates as candidate}<option value={candidate.repository}>{candidate.repository}</option>{/each}</select></div>
                {/if}
                {#if !syncCandidate}
                  <p class="muted-line">{$ui("选择目标仓库后才能预检和启用内容同步。")}</p>
                {:else}
                  <div class="sync-summary"><strong>{syncCandidate.repository}</strong><span>{syncCandidate.source} {$ui("· 仓库可见性：")}{syncCandidate.visibility === "public" ? $ui("公开") : $ui("未确认")}</span></div>
                  <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-21">{$ui("项目同步分支")}</label><span id="setting-field-21-hint">{$ui("与 Pages 发布分支隔离，默认 hexo-lite-content。")}</span></div><input id="setting-field-21" aria-describedby="setting-field-21-hint" aria-label={$ui("内容分支")} class="input compact-control" value={syncBranch} disabled={syncBusy} on:input={(event) => { syncBranch = event.currentTarget.value; syncPreflight = null; }} /></div>
                  {#if syncCandidate.visibility === "public" || syncCandidate.visibility === "unknown"}<label class="sync-warning"><input type="checkbox" bind:checked={publicAcknowledged} /><span>{$ui("我确认项目同步分支会继承仓库可见性，文章草稿、主题和 Hexo 配置也会公开；凭据、私钥、依赖及生成目录会被排除。")}</span></label>{/if}
                  {#if syncPreflight}<div class="sync-summary"><strong>{$ui("启用预检")}</strong><span>{$ui("本地")} {syncPreflight.fileCount} {$ui("个文件 ·")} {(syncPreflight.totalBytes / 1024 / 1024).toFixed(2)} MB</span>{#if syncPreflight.remoteBranchExists && syncPreflight.remoteManifestValid}<span>{$ui("远端")} {syncPreflight.remoteFileCount} {$ui("个文件 ·")} {(syncPreflight.remoteTotalBytes / 1024 / 1024).toFixed(2)} MB</span><span>{$ui("仅本地")} {syncPreflight.localOnlyCount} {$ui("· 仅远端")} {syncPreflight.remoteOnlyCount} {$ui("· 内容不同")} {syncPreflight.differentCount}</span>{:else}<span>{syncPreflight.remoteBranchExists ? $ui("远端分支没有合法清单，不能接管") : $ui("将创建新的孤立分支")}</span>{/if}</div>{/if}
                  <div class="button-row"><button class="button" type="button" disabled={syncBusy} on:click={preflightSync}>{$ui("预检")}</button><button class="button primary" type="button" disabled={syncBusy || !syncPreflight || (syncPreflight.remoteBranchExists && !syncPreflight.remoteManifestValid) || ((syncCandidate.visibility === "public" || syncCandidate.visibility === "unknown") && !publicAcknowledged)} on:click={() => configureSync()}>{$ui("确认启用")}</button></div>
                {/if}
              {/if}
            {:else if syncProvider === "webdav"}
              <details class="sync-connection-details" open={!syncStatus.enabled || webDavConnectionOpen} on:toggle={(event) => { if (syncStatus.enabled) webDavConnectionOpen = event.currentTarget.open; }}>
                <summary>{$ui("WebDAV 连接设置")}{#if syncStatus.enabled}<span>{$ui("点击修改服务器或凭据")}</span>{/if}</summary>
              <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-22">{$ui("服务器地址")}</label><span id="setting-field-22-hint">{$ui("启用后仍可修改；更换地址必须重新测试并明确应用。")}</span></div><input id="setting-field-22" aria-describedby="setting-field-22-hint" aria-label={$ui("WebDAV 服务器地址")} class="input compact-control" type="url" placeholder="https://dav.example.com/dav" value={webDavEndpoint} disabled={syncBusy} on:input={(event) => { webDavEndpoint = event.currentTarget.value; webDavCredential = { configured: false }; webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} on:blur={refreshWebDavCredential} /></div>
              <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-23">{$ui("远端目录")}</label><span id="setting-field-23-hint">{$ui("在该目录中保存完整项目源文件；依赖、生成目录、凭据和私钥不会上传。")}</span></div><input id="setting-field-23" aria-describedby="setting-field-23-hint" aria-label={$ui("WebDAV 远端目录")} class="input compact-control" value={webDavRemoteDir} disabled={syncBusy} on:input={(event) => { webDavRemoteDir = event.currentTarget.value; webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} /></div>
              <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-24">{$ui("用户名")}</label><span id="setting-field-24-hint">{$ui("可回显已保存用户名，完整密码永远不会返回前端。")}</span></div><input id="setting-field-24" aria-describedby="setting-field-24-hint" aria-label={$ui("WebDAV 用户名")} class="input compact-control" autocomplete="username" bind:value={webDavUsername} disabled={syncBusy} on:input={() => { webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} /></div>
              <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-25">{$ui("密码")}</label><span id="setting-field-25-hint">{webDavCredential.configured ? $ui("留空沿用系统凭据库中的密码；填写内容用于测试成功后才会覆盖旧密码。") : $ui("请输入密码；只有真实连接和读写测试通过后才会保存。")}</span></div><input id="setting-field-25" aria-describedby="setting-field-25-hint" aria-label={$ui("WebDAV 密码")} class="input compact-control" type="password" autocomplete="current-password" bind:value={webDavPassword} disabled={syncBusy} on:input={() => { webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} /></div>
              {#if syncStatus.enabled && syncStatus.provider === "webdav"}<div class="sync-summary"><strong>{$ui("当前已应用连接")}</strong><span>{syncStatus.endpoint}/{syncStatus.remoteDir}</span><span>{$ui("同步状态：")}{$ui(syncStatusLabel(syncStatus))} · {syncStatus.message || ""}</span></div>{/if}
              {#if webDavConnectionDirty}<p class="sync-warning" role="status">{$ui("服务器地址或远端目录已修改，尚未应用。请重新测试后点击“应用连接设置”。")}</p>{/if}
              {#if syncStatus.status === "authRequired"}<p class="sync-warning" role="alert">{$ui("当前凭据无法认证。请直接修改用户名或密码，然后重新测试。")}</p>{/if}
              {#if webDavConnectionError}<p class="sync-error" role="alert">{webDavConnectionError}</p>{/if}
              <div class="button-row"><button class="button" type="button" disabled={syncBusy || !webDavEndpoint.trim() || !webDavRemoteDir.trim() || !webDavUsername.trim() || (!webDavPassword && !webDavCredential.configured)} on:click={testWebDavConnection}>{syncBusy ? $ui("正在真实测试...") : $ui("保存并测试连接")}</button>{#if webDavCredential.configured}<button class="button danger" type="button" disabled={syncBusy} on:click={deleteWebDavCredential}>{$ui("删除凭据")}</button>{/if}</div>
              {#if webDavPreflight}<div class="sync-summary"><strong>{$ui("WebDAV 真实连接和预检通过")}</strong><span>{webDavPreflight.endpoint}/{webDavPreflight.remoteDir}</span>{#if webDavTestedAt}<span>{$ui("验证时间：")}{new Date(webDavTestedAt).toLocaleString()}</span>{/if}<span>{$ui("已验证目录访问、上传、下载和删除权限；本地")} {webDavPreflight.fileCount} {$ui("个文件 ·")} {(webDavPreflight.totalBytes / 1024 / 1024).toFixed(2)} MB</span>{#if webDavPreflight.remoteExists && webDavPreflight.remoteManifestValid}<span>{$ui("远端")} {webDavPreflight.remoteFileCount} {$ui("个文件 ·")} {(webDavPreflight.remoteTotalBytes / 1024 / 1024).toFixed(2)} MB</span><span>{$ui("仅本地")} {webDavPreflight.localOnlyCount} {$ui("· 仅远端")} {webDavPreflight.remoteOnlyCount} {$ui("· 内容不同")} {webDavPreflight.differentCount}</span>{:else}<span>{webDavPreflight.remoteExists ? $ui("远端目录没有合法清单，不能接管") : $ui("将初始化新的 WebDAV 远端目录")}</span>{/if}</div>{/if}
              {#if syncStatus.enabled && syncStatus.provider === "webdav"}
                {#if webDavConnectionDirty}<div class="button-row"><button class="button primary" type="button" disabled={syncBusy || !webDavTestMatches || (webDavPreflight?.remoteExists && !webDavPreflight.remoteManifestValid)} on:click={applyWebDavConnection}>{$ui("应用连接设置")}</button></div>{/if}
              {:else}
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy || !webDavTestMatches || !webDavPreflight || (webDavPreflight.remoteExists && !webDavPreflight.remoteManifestValid)} on:click={() => configureSync()}>{$ui("确认启用 WebDAV")}</button></div>
              {/if}
              </details>
            {/if}
            {#if syncStatus.enabled}
              <fieldset class="sync-decisions" disabled={syncBusy || webDavConnectionDirty}>
              <div class="sync-summary"><strong>{syncStatus.provider === "webdav" ? $ui("WebDAV 项目同步") : $ui("GitHub 项目同步")}</strong><span>{syncStatus.provider === "webdav" ? `${syncStatus.endpoint}/${syncStatus.remoteDir}` : `${syncStatus.repository} · ${syncStatus.branch}`}</span></div>
              <div class="sync-status-row"><span class={`sync-status ${syncStatus.status}`}>{$ui(syncStatusLabel(syncStatus))}</span><span>{syncStatus.message || ""}</span></div>
              {#if syncStatus.requiresScopeConfirmation}
                <p class="sync-warning" role="alert">{$ui("同步范围已升级为完整项目。确认后，草稿、主题和 Hexo 配置也会上传到当前 GitHub 分支。")}</p>
                <div class="button-row"><button class="button danger" type="button" disabled={syncBusy} on:click={() => (pendingSyncOverwrite = "overwriteRemote")}>{$ui("确认同步完整项目")}</button><button class="button danger" type="button" disabled={syncBusy} on:click={disableSync}>{$ui("关闭同步")}</button></div>
              {:else if syncStatus.status === "localPending" && !syncStatus.lastSyncedAt}
                <p class="sync-warning" role="status">{$ui("连接已启用。选择“上传本地内容”开始首次同步，或使用远端已有内容初始化本机。")}</p>
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy} on:click={() => (pendingInitialChoice = "local")}>{$ui("上传本地内容")}</button><button class="button" type="button" disabled={syncBusy} on:click={() => (pendingInitialChoice = "remote")}>{$ui("使用远端内容")}</button></div>
              {:else if syncStatus.status === "remoteAhead"}
                <p class="sync-warning" role="alert">{$ui("云端存在较新的项目版本。选择任一方向前都会再次读取最新云端状态；覆盖本地时会先创建备份。")}</p>
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy} on:click={() => (pendingSyncOverwrite = "overwriteLocal")}>{$ui("使用云端最新版本")}</button><button class="button danger" type="button" disabled={syncBusy} on:click={() => (pendingSyncOverwrite = "overwriteRemote")}>{$ui("用本机项目覆盖云端")}</button><button class="button" type="button" on:click={() => session && platform.openContentSyncBackups(session.projectId, session.generation)}>{$ui("打开备份目录")}</button></div>
              {:else if syncStatus.status === "conflict"}
                <p class="muted-line">{$ui("本地与远端修改了同一文件，请逐项选择。")}</p>
                <div class="button-row"><button class="button" type="button" on:click={() => runSync("auto")}>{$ui("重新检查冲突")}</button><button class="button danger" type="button" on:click={disableSync}>{$ui("关闭同步")}</button></div>
                <div class="button-row"><button class="button" type="button" on:click={() => chooseAllConflicts("local")}>{$ui("全部选择本地")}</button><button class="button" type="button" on:click={() => chooseAllConflicts("remote")}>{$ui("全部选择远端")}</button></div>
                <div class="sync-conflict-list">
                  {#each syncConflicts as conflict}
                    <article class="sync-conflict-card">
                      <strong>{conflict.path}</strong>
                      <span>{conflict.kind === "markdown" ? $ui("Markdown 文本") : $ui("二进制 · 本地 {p0} B / 远端 {p1} B", { p0: conflict.localSize ?? 0, p1: conflict.remoteSize ?? 0 })}</span>
                      {#if conflict.kind === "binary"}<code class="sync-conflict-hashes">{$ui("本地")} {conflict.localHash ?? $ui("已删除")} {$ui("· 远端")} {conflict.remoteHash ?? $ui("已删除")}</code>{/if}
                      {#if conflict.kind === "markdown"}<details><summary>{$ui("查看两端内容")}</summary><div class="sync-diff"><pre>{conflict.localText ?? $ui("（本地已删除）")}</pre><pre>{conflict.remoteText ?? $ui("（远端已删除）")}</pre></div></details>{/if}
                      <div class="button-row"><label><input type="radio" name={`sync-${conflict.path}`} value="local" checked={conflictChoices[conflict.path] === "local"} on:change={() => (conflictChoices = { ...conflictChoices, [conflict.path]: "local" })} /> {$ui("本地")}</label><label><input type="radio" name={`sync-${conflict.path}`} value="remote" checked={conflictChoices[conflict.path] === "remote"} on:change={() => (conflictChoices = { ...conflictChoices, [conflict.path]: "remote" })} /> {$ui("远端")}</label></div>
                    </article>
                  {/each}
                </div>
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy || !syncConflicts.length || syncConflicts.some((item) => !conflictChoices[item.path])} on:click={submitConflictChoices}>{$ui("提交冲突选择")}</button><button class="button" type="button" on:click={() => session && platform.openContentSyncBackups(session.projectId, session.generation)}>{$ui("打开备份目录")}</button></div>
              {:else}
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy} on:click={() => runSync("auto")}>{["error", "offline", "authRequired"].includes(syncStatus.status) ? $ui("重试同步") : $ui("立即上传变更")}</button>{#if syncStatus.status === "authRequired" && syncProvider === "github"}<button class="button" type="button" disabled={syncBusy} on:click={reconnectSync}>{$ui("重新认证")}</button>{/if}<button class="button danger" type="button" disabled={syncBusy} on:click={disableSync}>{$ui("关闭同步")}</button></div>
              {/if}
              {#if syncStatus.lastSyncedAt}<small class="muted-line">{$ui("上次同步：")}{new Date(syncStatus.lastSyncedAt).toLocaleString()}</small>{/if}
              </fieldset>
            {/if}
          {/if}
        </div>
      {:else}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>{$ui("更新与恢复")}</h3><p>{$ui("更新只从固定的项目 Releases 页面检查。")}</p></div>
          <div class="setting-row"><div class="setting-copy"><label class="setting-title" for="setting-field-26">{$ui("自动检查并下载更新")}</label><span id="setting-field-26-hint">{$ui("启动后每天最多检查一次；仅下载版本号更高且签名有效的更新，安装前会询问。")}</span></div><label class="switch"><input id="setting-field-26" aria-describedby="setting-field-26-hint" type="checkbox" checked={draft.update.checkOnStart} on:change={(event) => change({ ...draft, update: { checkOnStart: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><strong>{$ui("恢复默认设置")}</strong><span>{$ui("Token 不会被删除；默认值保存前仍可取消。")}</span></div><button class="button" type="button" on:click={() => (showReset = true)}><RotateCcw size={14} />{$ui("恢复默认")}</button></div>
        </div>
      {/if}
    </section>
  </div>
</div>

{#if showAcquireToken}
  <ModalDialog title={$ui("获取 Cloudflare-ImgBed Token")} description={$ui("管理员凭据只用于本次登录和创建 Token，不会写入配置或日志。")} onClose={closeAcquireToken}>
    <div class="modal-form">
      <label><span>{$ui("管理员用户名")}</span><input class="input" data-autofocus autocomplete="username" bind:value={adminUsername} placeholder={$ui("按服务端配置填写，可留空")} /></label>
      <label><span>{$ui("管理员密码")}</span><input class="input" type="password" autocomplete="current-password" bind:value={adminPassword} placeholder={$ui("按服务端配置填写，可留空")} /></label>
      {#if tokenStatusMessage}<p class="modal-status" role="status">{tokenStatusMessage}</p>{/if}
    </div>
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={credentialBusy} on:click={closeAcquireToken}>{$ui("取消")}</button><button class="button primary" type="button" disabled={credentialBusy} on:click={acquireToken}><KeyRound size={14} />{credentialBusy ? $ui("正在获取 Token...") : $ui("获取并保存")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if showReset}
  <ModalDialog title={$ui("恢复默认设置？")} description={$ui("默认值会先进入设置草稿，点击保存后才会写入；系统凭据库中的 Token 不受影响。")} onClose={() => (showReset = false)}>
    <svelte:fragment slot="actions"><button class="button" type="button" on:click={() => (showReset = false)}>{$ui("取消")}</button><button class="button danger" type="button" data-autofocus on:click={restoreDefaults}>{$ui("恢复默认")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if showClearRecent}
  <ModalDialog title={$ui("清空最近项目？")} description={$ui("只删除最近项目记录，不会删除磁盘上的博客文件。")} onClose={() => (showClearRecent = false)}>
    <svelte:fragment slot="actions"><button class="button" type="button" on:click={() => (showClearRecent = false)}>{$ui("取消")}</button><button class="button danger" type="button" data-autofocus on:click={clearRecent}>{$ui("清空记录")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if pendingSyncOverwrite}
  <ModalDialog
    title={pendingSyncOverwrite === "overwriteLocal" ? $ui("使用云端最新项目？") : $ui("用本机项目覆盖云端？")}
    description={pendingSyncOverwrite === "overwriteLocal"
      ? $ui("云端项目会覆盖本地同名文件，并删除云端已删除的本地文件。应用会先创建本地备份。")
      : $ui("将基于刚读取的云端最新提交创建一个新版本，使云端项目内容与本机一致。其他设备尚未上传的改动会被覆盖。")}
    onClose={() => (pendingSyncOverwrite = null)}
  >
    <svelte:fragment slot="actions"><button class="button" type="button" on:click={() => (pendingSyncOverwrite = null)}>{$ui("取消")}</button><button class="button danger" type="button" data-autofocus on:click={confirmSyncOverwrite}>{$ui("确认覆盖")}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if pendingInitialChoice}
  <ModalDialog title={pendingInitialChoice === "remote" ? $ui("使用远端内容初始化？") : $ui("上传本地内容初始化？")} description={pendingInitialChoice === "remote" ? $ui("将用远端内容覆盖同步范围内的本地文件，并删除远端没有的文件；应用会先创建本地备份。旧版清单未包含的配置、主题和草稿将保留。") : $ui("远端同步目录将以本地项目内容为准。远端独有的内容可能被移除，请先确认预检中的差异。")} onClose={() => (pendingInitialChoice = null)}>
    <svelte:fragment slot="actions"><button class="button" type="button" on:click={() => (pendingInitialChoice = null)}>{$ui("取消")}</button><button class="button danger" type="button" on:click={() => { const direction = pendingInitialChoice; pendingInitialChoice = null; if (direction) void configureSync(direction); }}>{$ui("确认初始化")}</button></svelte:fragment>
  </ModalDialog>
{/if}

<style>
  .setting-title { font-weight: 600; cursor: pointer; }
  .sync-decisions { border: 0; margin: 0; padding: 0; min-width: 0; }
  .sync-workflow { display: flex; gap: 16px; padding: 0; list-style: none; flex-wrap: wrap; color: var(--text-muted); font-size: 12px; }
  .sync-workflow .active { color: var(--text); font-weight: 600; }
  .sync-progress { display: grid; gap: 8px; padding: 14px; border: 1px solid var(--border); border-radius: 8px; margin-block: 12px; overflow-wrap: anywhere; }
  .sync-progress progress { width: 100%; }
  .sync-progress span { font-size: 12px; color: var(--text-muted); }
  .sync-progress .button { justify-self: start; }
  .sync-connection-details { margin-block: 12px; }
  .sync-connection-details > summary { padding-block: 10px; cursor: pointer; font-weight: 600; }
  .sync-connection-details > summary span { margin-left: 12px; font-weight: 400; font-size: 12px; color: var(--text-muted); }
</style>
