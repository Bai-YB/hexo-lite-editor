<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { FileText, KeyRound, RotateCcw, Save, Trash2 } from "@lucide/svelte";
  import PageHeader from "$shared/components/PageHeader.svelte";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import CloudflareImageBedSettings from "./CloudflareImageBedSettings.svelte";
  import LocalImageBedSettings from "./LocalImageBedSettings.svelte";
  import { defaultConfig } from "$shared/types/app";
  import { normalizeError, platform } from "$platform/tauri";
  import { shortcutLabel } from "$platform/os";
  import type { SettingsController } from "./controller";
  import type {
    AppConfigV3,
    ContentSyncConflict,
    ContentSyncPreflight,
    ContentSyncProvider,
    CredentialStatus,
    RecentProjectView,
    SettingsSectionId,
    ThemeMode,
    TaskEvent,
    TaskLogSummary,
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
  export let onNotice: (message: string) => void = () => {};

  const sectionStorageKey = "hexo-lite-editor:settings-active-section";
  const sections: Array<{ id: SettingsSectionId; title: string; description: string }> = [
    { id: "general", title: "常规", description: "启动、保存与项目记录" },
    { id: "editing", title: "编辑体验", description: "外观、排版与编辑辅助" },
    { id: "images", title: "图片与图床", description: "导入目标与连接状态" },
    { id: "hexoPublish", title: "Hexo 与发布", description: "浏览器预览与发布流水线" },
    { id: "sync", title: "内容同步", description: "GitHub 或 WebDAV" },
    { id: "maintenance", title: "诊断与维护", description: "日志、更新与恢复" }
  ];

  let saved = structuredClone(config);
  let draft = structuredClone(config);
  let activeSection: SettingsSectionId = "general";
  let dirty = false;
  let saving = false;
  let credential: CredentialStatus = { configured: false };
  let credentialBusy = false;
  let tokenStatusMessage = "";
  let showAcquireToken = false;
  let adminUsername = "";
  let adminPassword = "";
  let showReset = false;
  let showClearRecent = false;
  let logs: TaskLogSummary[] = [];
  let selectedLog = "";
  let logEvents: TaskEvent[] = [];
  let logsBusy = false;
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

  $: dirty = JSON.stringify(draft) !== JSON.stringify(saved);
  $: currentSection = sections.find((section) => section.id === activeSection) ?? sections[0];
  $: webDavConnectionDirty = syncStatus.enabled && syncStatus.provider === "webdav"
    && (webDavEndpoint.trim().replace(/\/$/, "") !== (syncStatus.endpoint ?? "").replace(/\/$/, "")
      || webDavRemoteDir.trim().replace(/^\/+|\/+$/g, "") !== (syncStatus.remoteDir ?? ""));
  $: webDavTestMatches = Boolean(webDavPreflight)
    && webDavEndpoint.trim().replace(/\/$/, "") === webDavTestedEndpoint
    && webDavRemoteDir.trim().replace(/^\/+|\/+$/g, "") === webDavTestedRemoteDir;
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
    onRegisterSettingsController({ save: saveDraft, discard, hasDirty: () => dirty });
    void refreshCredential();
    void refreshLogs();
    void refreshSync();
  });

  onDestroy(() => onRegisterSettingsController(null));

  function selectSection(section: SettingsSectionId) {
    activeSection = section;
    localStorage.setItem(sectionStorageKey, section);
  }

  async function refreshSync() {
    if (!session) {
      syncCandidate = null;
      syncStatus = { enabled: false, status: "off", provider: "github", conflicts: [] };
      return;
    }
    try {
      const detection = await platform.detectContentSync(session.projectId, session.generation);
      syncCandidates = detection.candidates;
      syncStatus = await platform.getContentSyncStatus(session.projectId, session.generation);
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
    if (!session || syncBusy || (syncProvider === "github" && !syncCandidate)) return;
    syncBusy = true;
    try {
      if (syncStatus.enabled && initialChoice) {
        syncStatus = await platform.runContentSync(session.projectId, session.generation, initialChoice);
      } else if (syncProvider === "github" && syncCandidate) {
        syncStatus = await platform.enableContentSync({
          projectId: session.projectId,
          sessionGeneration: session.generation,
          repository: syncCandidate.repository,
          branch: syncBranch,
          initialChoice,
          confirmPublic: syncCandidate.visibility !== "public" || publicAcknowledged
        });
      } else {
        syncStatus = await platform.enableWebDavContentSync({
          projectId: session.projectId,
          sessionGeneration: session.generation,
          endpoint: webDavEndpoint,
          remoteDir: webDavRemoteDir,
          initialChoice
        });
      }
      onNotice(syncStatus.message || "内容同步设置已更新。");
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      syncBusy = false;
    }
  }

  async function preflightSync() {
    if (!session || syncBusy || (syncProvider === "github" && !syncCandidate)) return;
    syncBusy = true;
    try {
      if (syncProvider === "github" && syncCandidate) {
        syncPreflight = await platform.preflightContentSync(session.projectId, session.generation, syncCandidate.repository, syncBranch);
        syncCandidate = syncPreflight.candidate;
      }
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      syncBusy = false;
    }
  }

  async function refreshWebDavCredential() {
    if (!webDavEndpoint.trim()) {
      webDavCredential = { configured: false };
      return;
    }
    try {
      webDavCredential = await platform.webDavCredentialStatus(webDavEndpoint);
      if (!webDavUsername && webDavCredential.username) webDavUsername = webDavCredential.username;
    }
    catch { webDavCredential = { configured: false }; }
  }

  async function testWebDavConnection() {
    if (!session || syncBusy || !webDavEndpoint.trim() || !webDavRemoteDir.trim() || !webDavUsername.trim()) return;
    syncBusy = true;
    webDavConnectionError = "";
    try {
      const result = await platform.testWebDavContentSync({
        projectId: session.projectId,
        sessionGeneration: session.generation,
        endpoint: webDavEndpoint,
        remoteDir: webDavRemoteDir,
        username: webDavUsername,
        password: webDavPassword || undefined
      });
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
      syncBusy = false;
    }
  }

  async function applyWebDavConnection() {
    if (!session || syncBusy || !webDavTestMatches) return;
    syncBusy = true;
    webDavConnectionError = "";
    try {
      syncStatus = await platform.updateWebDavContentSync({
        projectId: session.projectId,
        sessionGeneration: session.generation,
        endpoint: webDavEndpoint,
        remoteDir: webDavRemoteDir
      });
      onNotice(syncStatus.message || "WebDAV 连接设置已应用。");
    } catch (error) {
      webDavConnectionError = normalizeError(error).message;
      onNotice(webDavConnectionError);
    } finally {
      syncBusy = false;
    }
  }

  async function deleteWebDavCredential() {
    if (syncBusy || !webDavEndpoint.trim()) return;
    syncBusy = true;
    try {
      webDavCredential = await platform.webDavCredentialDelete(webDavEndpoint);
      webDavPreflight = null;
      webDavTestedAt = "";
      webDavConnectionError = "";
      onNotice("WebDAV 凭据已删除。");
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      syncBusy = false;
    }
  }

  async function refreshSyncConflicts() {
    if (!session || syncStatus.status !== "conflict") {
      syncConflicts = [];
      conflictChoices = {};
      return;
    }
    syncConflicts = await platform.getContentSyncConflicts(session.projectId, session.generation);
    conflictChoices = Object.fromEntries(syncConflicts.map((item) => [item.path, "local"]));
  }

  async function submitConflictChoices() {
    if (!session || syncBusy || syncConflicts.some((item) => !conflictChoices[item.path])) return;
    syncBusy = true;
    try {
      syncStatus = await platform.resolveContentSyncConflicts(session.projectId, session.generation, conflictChoices);
      syncConflicts = [];
      conflictChoices = {};
      onNotice(syncStatus.message || "冲突已解决。");
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      syncBusy = false;
    }
  }

  async function runSync(direction: "auto" | "local" | "remote" = "auto") {
    if (!session || syncBusy) return;
    syncBusy = true;
    try {
      syncStatus = await platform.runContentSync(session.projectId, session.generation, direction);
      await refreshSyncConflicts();
      onNotice(syncStatus.message || "同步检查完成。");
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      syncBusy = false;
    }
  }

  async function disableSync() {
    if (!session || syncBusy) return;
    syncBusy = true;
    try {
      syncStatus = await platform.disableContentSync(session.projectId, session.generation);
      onNotice("内容同步已关闭，本地文章不会被删除。");
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      syncBusy = false;
    }
  }

  async function reconnectSync() {
    if (!session || syncBusy) return;
    syncBusy = true;
    try {
      syncStatus = await platform.reconnectContentSync(session.projectId, session.generation);
      onNotice(syncStatus.message || "系统 Git 认证检查完成。");
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      syncBusy = false;
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
    draft = next;
    if (next.appearance.themeMode !== saved.appearance.themeMode) onThemePreview(next.appearance.themeMode);
  }

  async function persistConfig(nextConfig: AppConfigV3, message = "设置已保存。") {
    saving = true;
    try {
      const next = await onSaveConfig(structuredClone(nextConfig));
      saved = structuredClone(next);
      draft = structuredClone(next);
      onNotice(message);
    } finally {
      saving = false;
    }
  }

  async function saveDraft() {
    if (dirty && !saving) await persistConfig(draft);
  }

  function discard() {
    draft = structuredClone(saved);
    onThemePreview(saved.appearance.themeMode);
  }

  function restoreDefaults() {
    draft = structuredClone(defaultConfig);
    onThemePreview(draft.appearance.themeMode);
    showReset = false;
  }

  async function refreshCredential() {
    try { credential = await platform.credentialStatus(); }
    catch { credential = { configured: false }; }
  }

  async function prepareAcquireToken() {
    if (!draft.imageBed.cloudflareName.trim()) return onNotice("请先填写图床名称。");
    if (!draft.imageBed.cloudflareApiUrl.trim()) return onNotice("请先填写 Cloudflare-ImgBed 服务地址。");
    try {
      if (dirty) await persistConfig(draft, "图床基础配置已保存。");
      showAcquireToken = true;
      tokenStatusMessage = "";
    } catch (error) { onNotice(normalizeError(error).message); }
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
      const result = await platform.acquireCloudflareImgbedToken({
        baseUrl: draft.imageBed.cloudflareApiUrl,
        adminUsername: adminUsername.trim() || undefined,
        adminPassword: adminPassword || undefined,
        tokenName: draft.imageBed.cloudflareName,
        owner: "Hexo Lite Editor",
        permissions: ["upload", "list", "delete"],
        expiresAt: null,
        autoDelete: false
      });
      credential = { configured: result.configured };
      await persistConfig({ ...draft, imageBed: { ...draft.imageBed, cloudflareTokenId: result.tokenId } }, "Token 已创建并保存到系统凭据库。");
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
    try { tokenStatusMessage = (await platform.testCloudflareImgbedToken(draft.imageBed.cloudflareApiUrl)).message; }
    catch (error) { tokenStatusMessage = normalizeError(error).message; }
    finally { credentialBusy = false; }
  }

  async function deleteCredential() {
    credentialBusy = true;
    try {
      credential = await platform.credentialDelete();
      const { cloudflareTokenId: _removed, ...imageBed } = draft.imageBed;
      await persistConfig({ ...draft, imageBed }, "Cloudflare Token 已从系统凭据库删除。");
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

  async function refreshLogs() {
    logsBusy = true;
    try {
      logs = await platform.listTaskLogs();
      if (selectedLog && !logs.some((log) => log.taskId === selectedLog)) { selectedLog = ""; logEvents = []; }
    } catch (error) { onNotice(normalizeError(error).message); }
    finally { logsBusy = false; }
  }

  async function openLog(taskId: string) {
    logsBusy = true;
    try { const page = await platform.readTaskLog(taskId); selectedLog = taskId; logEvents = page.events; }
    catch (error) { onNotice(normalizeError(error).message); }
    finally { logsBusy = false; }
  }

  async function removeLog(taskId: string) {
    try { await platform.deleteTaskLog(taskId); await refreshLogs(); }
    catch (error) { onNotice(normalizeError(error).message); }
  }

  async function clearLogs() {
    try { await platform.clearTaskLogs(); selectedLog = ""; logEvents = []; await refreshLogs(); }
    catch (error) { onNotice(normalizeError(error).message); }
  }
</script>

<div class="workspace-page settings-page">
  <div class="settings-sticky-header">
    <PageHeader title="设置" description="按工作流程整理；更改只在保存后写入应用配置。">
      <span class:warning={dirty} class:success={!dirty} class="settings-save-state">{saving ? "正在保存" : dirty ? "有未保存更改" : "已保存"}</span>
      <button class="button" type="button" on:click={() => (showReset = true)}><RotateCcw size={15} />恢复默认</button>
      <button class="button" type="button" disabled={!dirty || saving} on:click={discard}>取消</button>
      <button class="button primary" type="button" disabled={!dirty || saving} on:click={saveDraft}><Save size={15} />保存</button>
    </PageHeader>
  </div>

  <div class="settings-layout">
    <nav class="settings-nav" aria-label="设置分类">
      {#each sections as section, index (section.id)}
        <button
          type="button"
          class:active={activeSection === section.id}
          data-settings-section={section.id}
          aria-current={activeSection === section.id ? "page" : undefined}
          on:click={() => selectSection(section.id)}
          on:keydown={(event) => handleNavKeydown(event, index)}
        >
          <span><strong>{section.title}</strong><small>{section.description}</small></span>
          {#if dirtySections[section.id]}<i class="settings-dirty-dot" aria-label="此分类有未保存更改"></i>{/if}
        </button>
      {/each}
    </nav>

    <section class="panel settings-content-panel" aria-labelledby={`settings-title-${activeSection}`}>
      <header class="settings-content-heading">
        <p>设置分类</p>
        <h2 id={`settings-title-${activeSection}`}>{currentSection.title}</h2>
        <span>{currentSection.description}</span>
      </header>

      {#if activeSection === "general"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>启动</h3><p>控制应用进入工作区时的恢复行为。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>启动时打开最近项目</strong><span>只恢复上次经过验证的 Hexo 项目。</span></div><label class="switch"><input type="checkbox" checked={draft.general.openRecentProjectOnStart} on:change={(event) => change({ ...draft, general: { ...draft.general, openRecentProjectOnStart: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>保存与备份</h3><p>减少输入中断，同时保留明确的安全边界。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>自动保存</strong><span>按文章、revision 和项目会话串行保存。</span></div><label class="switch"><input type="checkbox" checked={draft.general.autoSave} on:change={(event) => change({ ...draft, general: { ...draft.general, autoSave: event.currentTarget.checked } })} /><span></span></label></div>
          <div class:disabled={!draft.general.autoSave} class="setting-row setting-row-dependent"><div class="setting-copy"><strong>自动保存延迟</strong><span>停止输入后等待的毫秒数。</span></div><input class="input compact-control" disabled={!draft.general.autoSave} type="number" min="500" max="30000" step="100" value={draft.general.autoSaveDelayMs} on:change={(event) => change({ ...draft, general: { ...draft.general, autoSaveDelayMs: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><strong>保存前创建备份</strong><span>在 .hlex-backups 中保留上一版本。</span></div><label class="switch"><input type="checkbox" checked={draft.general.backupBeforeSave} on:change={(event) => change({ ...draft, general: { ...draft.general, backupBeforeSave: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <div class="settings-block">
          <div class="setting-subsection-heading"><div><h3>最近项目</h3><span>最多保留 10 个项目，路径仅由后端管理。</span></div>{#if recentProjects.length}<button class="button danger" type="button" on:click={() => (showClearRecent = true)}>清空</button>{/if}</div>
          {#if recentProjects.length}<div class="recent-project-list">{#each recentProjects as recent (recent.recentId)}<div class="recent-project-row"><div><strong>{recent.name}</strong><span>{recent.displayPath}</span></div><span class:warning={!recent.available} class="recent-availability">{recent.available ? "可用" : "不可用"}</span><button class="icon-button" type="button" title="移除记录" aria-label={`移除 ${recent.name}`} on:click={() => removeRecent(recent.recentId)}><Trash2 size={15} /></button></div>{/each}</div>{:else}<p class="muted-line">尚无最近项目。</p>{/if}
        </div>
      {:else if activeSection === "editing"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>外观</h3><p>主题切换会立即预览，取消后恢复。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>主题模式</strong><span>浅色、深色或跟随系统。</span></div><select class="select compact-control" value={draft.appearance.themeMode} on:change={(event) => change({ ...draft, appearance: { themeMode: event.currentTarget.value as ThemeMode } })}><option value="system">跟随系统</option><option value="light">浅色</option><option value="dark">深色</option></select></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>正文排版</h3><p>统一编辑器的阅读密度和缩进节奏。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>字号</strong><span>12–28 px。</span></div><input class="input compact-control" type="number" min="12" max="28" value={draft.editor.fontSize} on:change={(event) => change({ ...draft, editor: { ...draft.editor, fontSize: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><strong>行高</strong><span>1.2–2.2。</span></div><input class="input compact-control" type="number" min="1.2" max="2.2" step="0.05" value={draft.editor.lineHeight} on:change={(event) => change({ ...draft, editor: { ...draft.editor, lineHeight: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><strong>Tab 宽度</strong><span>使用 2、4 或 8 个空格。</span></div><select class="select compact-control" value={draft.editor.tabSize} on:change={(event) => change({ ...draft, editor: { ...draft.editor, tabSize: Number(event.currentTarget.value) } })}><option value="2">2</option><option value="4">4</option><option value="8">8</option></select></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>编辑辅助</h3><p>只保留写作过程中持续有用的视觉提示。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>显示行号</strong><span>在正文左侧显示行号栏。</span></div><label class="switch"><input type="checkbox" checked={draft.editor.showLineNumbers} on:change={(event) => change({ ...draft, editor: { ...draft.editor, showLineNumbers: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><strong>自动换行</strong><span>长行按编辑区宽度折行。</span></div><label class="switch"><input type="checkbox" checked={draft.editor.lineWrapping} on:change={(event) => change({ ...draft, editor: { ...draft.editor, lineWrapping: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><strong>突出当前行</strong><span>使用低对比背景标识光标行。</span></div><label class="switch"><input type="checkbox" checked={draft.editor.highlightActiveLine} on:change={(event) => change({ ...draft, editor: { ...draft.editor, highlightActiveLine: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><strong>文章列表封面</strong><span>在文章标题左侧显示缩略图。</span></div><label class="switch"><input type="checkbox" checked={draft.articleList.showCover} on:change={(event) => change({ ...draft, articleList: { showCover: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
      {:else if activeSection === "images"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>图片工作流</h3><p>决定导入、粘贴和拖入图片时的目标。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>默认来源</strong><span>本地项目目录或 Cloudflare-ImgBed。</span></div><select class="select compact-control" value={draft.imageBed.defaultProvider} on:change={(event) => change({ ...draft, imageBed: { ...draft.imageBed, defaultProvider: event.currentTarget.value as AppConfigV3["imageBed"]["defaultProvider"] } })}><option value="local">本地图片</option><option value="cloudflare-imgbed">Cloudflare-ImgBed</option></select></div>
          <div class="setting-row"><div class="setting-copy"><strong>上传后插入 Markdown</strong><span>成功后插入当前文章的最后光标位置。</span></div><label class="switch"><input type="checkbox" checked={draft.imageBed.autoInsertMarkdown} on:change={(event) => change({ ...draft, imageBed: { ...draft.imageBed, autoInsertMarkdown: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <div class="settings-block provider-block">
          <div class="settings-block-heading"><h3>{draft.imageBed.defaultProvider === "local" ? "本地图片目录" : "Cloudflare 连接"}</h3><p>{draft.imageBed.defaultProvider === "local" ? "路径由后端验证，不能离开项目的 source 目录。" : "连接信息、凭据状态和操作集中管理。"}</p></div>
          {#if draft.imageBed.defaultProvider === "local"}
            <LocalImageBedSettings settings={draft.imageBed} onChange={(imageBed) => change({ ...draft, imageBed })} />
          {:else}
            <CloudflareImageBedSettings settings={draft.imageBed} {credential} busy={credentialBusy} statusMessage={tokenStatusMessage} onChange={(imageBed) => change({ ...draft, imageBed })} onAcquireToken={prepareAcquireToken} onTestConnection={testCredential} onDeleteToken={deleteCredential} />
          {/if}
        </div>
      {:else if activeSection === "hexoPublish"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>浏览器预览</h3><p>软件内不嵌入主题页面；真实 Hexo 页面在系统浏览器打开。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>预览端口</strong><span>默认使用 4000。</span></div><input class="input compact-control" type="number" min="300" max="65535" value={draft.hexo.previewPort} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, previewPort: Number(event.currentTarget.value) } })} /></div>
          <div class="setting-row"><div class="setting-copy"><strong>打开项目后自动启动预览</strong><span>只在后台启动 Hexo Server，不自动弹出浏览器。</span></div><label class="switch"><input type="checkbox" checked={draft.hexo.autoStartPreview} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, autoStartPreview: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><strong>预览草稿</strong><span>启动 Hexo Server 时使用固定的 --draft 参数。</span></div><label class="switch"><input type="checkbox" checked={draft.hexo.previewDrafts} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, previewDrafts: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>发布流水线</h3><p>发布快捷键为 {shortcutLabel("⇧P")}。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>运行前保存文章</strong><span>保存失败时中止发布并保留内容。</span></div><label class="switch"><input type="checkbox" checked={draft.publish.saveBeforeRun} on:change={(event) => change({ ...draft, publish: { ...draft.publish, saveBeforeRun: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><strong>生成前清理</strong><span>先执行 hexo clean。</span></div><label class="switch"><input type="checkbox" checked={draft.publish.cleanBeforeGenerate} on:change={(event) => change({ ...draft, publish: { ...draft.publish, cleanBeforeGenerate: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><strong>部署前生成</strong><span>在 deploy 前执行 hexo generate。</span></div><label class="switch"><input type="checkbox" checked={draft.publish.generateBeforeDeploy} on:change={(event) => change({ ...draft, publish: { ...draft.publish, generateBeforeDeploy: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><strong>部署后 Git Push</strong><span>执行固定 git push，不接受自定义参数。</span></div><label class="switch"><input type="checkbox" checked={draft.publish.gitPushAfterDeploy} on:change={(event) => change({ ...draft, publish: { ...draft.publish, gitPushAfterDeploy: event.currentTarget.checked } })} /><span></span></label></div>
        </div>
      {:else if activeSection === "sync"}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>内容同步</h3><p>可将文章和资源同步到 GitHub 独立分支，或你自己的 WebDAV 服务器。</p></div>
          {#if !session}
            <p class="muted-line">请先打开一个 Hexo 项目。</p>
          {:else}
            {#if !syncStatus.enabled}
              <div class="setting-row"><div class="setting-copy"><strong>同步方式</strong><span>选择 GitHub 或任意兼容 WebDAV 的服务器。</span></div><select aria-label="同步方式" class="select compact-control" bind:value={syncProvider} on:change={() => { syncPreflight = null; webDavPreflight = null; }}><option value="github">GitHub</option><option value="webdav">WebDAV</option></select></div>
            {/if}
            {#if !syncStatus.enabled && syncProvider === "github"}
              {#if !syncCandidates.length}
                <p class="muted-line">没有检测到 GitHub Pages 或 GitHub deploy 仓库。你仍可改用 WebDAV。</p>
              {:else}
                {#if syncCandidates.length > 1}
                  <div class="setting-row"><div class="setting-copy"><strong>目标仓库</strong><span>检测到多个 Git deploy 仓库，请明确选择。</span></div><select aria-label="目标仓库" class="select compact-control" value={syncCandidate?.repository ?? ""} on:change={(event) => { syncCandidate = syncCandidates.find((item) => item.repository === event.currentTarget.value) ?? null; syncPreflight = null; publicAcknowledged = false; }}><option value="" disabled>请选择仓库</option>{#each syncCandidates as candidate}<option value={candidate.repository}>{candidate.repository}</option>{/each}</select></div>
                {/if}
                {#if !syncCandidate}
                  <p class="muted-line">选择目标仓库后才能预检和启用内容同步。</p>
                {:else}
                  <div class="sync-summary"><strong>{syncCandidate.repository}</strong><span>{syncCandidate.source} · 仓库可见性：{syncCandidate.visibility === "public" ? "公开" : "未确认"}</span></div>
                  <div class="setting-row"><div class="setting-copy"><strong>内容分支</strong><span>与 Pages 发布分支隔离，默认 hexo-lite-content。</span></div><input aria-label="内容分支" class="input compact-control" value={syncBranch} disabled={syncBusy} on:input={(event) => { syncBranch = event.currentTarget.value; syncPreflight = null; }} /></div>
                  {#if syncCandidate.visibility === "public" || syncCandidate.visibility === "unknown"}<label class="sync-warning"><input type="checkbox" bind:checked={publicAcknowledged} /><span>我确认这个仓库的内容分支会继承仓库可见性，不放入草稿或凭据。</span></label>{/if}
                  {#if syncPreflight}<div class="sync-summary"><strong>启用预检</strong><span>本地 {syncPreflight.fileCount} 个文件 · {(syncPreflight.totalBytes / 1024 / 1024).toFixed(2)} MB</span>{#if syncPreflight.remoteBranchExists && syncPreflight.remoteManifestValid}<span>远端 {syncPreflight.remoteFileCount} 个文件 · {(syncPreflight.remoteTotalBytes / 1024 / 1024).toFixed(2)} MB</span><span>仅本地 {syncPreflight.localOnlyCount} · 仅远端 {syncPreflight.remoteOnlyCount} · 内容不同 {syncPreflight.differentCount}</span>{:else}<span>{syncPreflight.remoteBranchExists ? "远端分支没有合法清单，不能接管" : "将创建新的孤立分支"}</span>{/if}</div>{/if}
                  <div class="button-row"><button class="button" type="button" disabled={syncBusy} on:click={preflightSync}>预检</button><button class="button primary" type="button" disabled={syncBusy || !syncPreflight || (syncPreflight.remoteBranchExists && !syncPreflight.remoteManifestValid) || ((syncCandidate.visibility === "public" || syncCandidate.visibility === "unknown") && !publicAcknowledged)} on:click={() => configureSync()}>确认启用</button></div>
                {/if}
              {/if}
            {:else if syncProvider === "webdav"}
              <div class="setting-row"><div class="setting-copy"><strong>服务器地址</strong><span>启用后仍可修改；更换地址必须重新测试并明确应用。</span></div><input aria-label="WebDAV 服务器地址" class="input compact-control" type="url" placeholder="https://dav.example.com/dav" value={webDavEndpoint} disabled={syncBusy} on:input={(event) => { webDavEndpoint = event.currentTarget.value; webDavCredential = { configured: false }; webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} on:blur={refreshWebDavCredential} /></div>
              <div class="setting-row"><div class="setting-copy"><strong>远端目录</strong><span>仅在该目录中保存同步清单、文章和图片；修改后不会自动生效。</span></div><input aria-label="WebDAV 远端目录" class="input compact-control" value={webDavRemoteDir} disabled={syncBusy} on:input={(event) => { webDavRemoteDir = event.currentTarget.value; webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} /></div>
              <div class="setting-row"><div class="setting-copy"><strong>用户名</strong><span>可回显已保存用户名，完整密码永远不会返回前端。</span></div><input aria-label="WebDAV 用户名" class="input compact-control" autocomplete="username" bind:value={webDavUsername} disabled={syncBusy} on:input={() => { webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} /></div>
              <div class="setting-row"><div class="setting-copy"><strong>密码</strong><span>{webDavCredential.configured ? "留空沿用系统凭据库中的密码；填写内容用于测试成功后才会覆盖旧密码。" : "请输入密码；只有真实连接和读写测试通过后才会保存。"}</span></div><input aria-label="WebDAV 密码" class="input compact-control" type="password" autocomplete="current-password" bind:value={webDavPassword} disabled={syncBusy} on:input={() => { webDavPreflight = null; webDavTestedAt = ""; webDavConnectionError = ""; }} /></div>
              {#if syncStatus.enabled && syncStatus.provider === "webdav"}<div class="sync-summary"><strong>当前已应用连接</strong><span>{syncStatus.endpoint}/{syncStatus.remoteDir}</span><span>同步状态：{syncStatus.status} · {syncStatus.message || ""}</span></div>{/if}
              {#if webDavConnectionDirty}<p class="sync-warning" role="status">服务器地址或远端目录已修改，尚未应用。请重新测试后点击“应用连接设置”。</p>{/if}
              {#if syncStatus.status === "authRequired"}<p class="sync-warning" role="alert">当前凭据无法认证。请直接修改用户名或密码，然后重新测试。</p>{/if}
              {#if webDavConnectionError}<p class="sync-error" role="alert">{webDavConnectionError}</p>{/if}
              <div class="button-row"><button class="button" type="button" disabled={syncBusy || !webDavEndpoint.trim() || !webDavRemoteDir.trim() || !webDavUsername.trim() || (!webDavPassword && !webDavCredential.configured)} on:click={testWebDavConnection}>{syncBusy ? "正在真实测试..." : "保存并测试连接"}</button>{#if webDavCredential.configured}<button class="button danger" type="button" disabled={syncBusy} on:click={deleteWebDavCredential}>删除凭据</button>{/if}</div>
              {#if webDavPreflight}<div class="sync-summary"><strong>WebDAV 真实连接和预检通过</strong><span>{webDavPreflight.endpoint}/{webDavPreflight.remoteDir}</span>{#if webDavTestedAt}<span>验证时间：{new Date(webDavTestedAt).toLocaleString()}</span>{/if}<span>已验证目录访问、上传、下载和删除权限；本地 {webDavPreflight.fileCount} 个文件 · {(webDavPreflight.totalBytes / 1024 / 1024).toFixed(2)} MB</span>{#if webDavPreflight.remoteExists && webDavPreflight.remoteManifestValid}<span>远端 {webDavPreflight.remoteFileCount} 个文件 · {(webDavPreflight.remoteTotalBytes / 1024 / 1024).toFixed(2)} MB</span><span>仅本地 {webDavPreflight.localOnlyCount} · 仅远端 {webDavPreflight.remoteOnlyCount} · 内容不同 {webDavPreflight.differentCount}</span>{:else}<span>{webDavPreflight.remoteExists ? "远端目录没有合法清单，不能接管" : "将初始化新的 WebDAV 远端目录"}</span>{/if}</div>{/if}
              {#if syncStatus.enabled && syncStatus.provider === "webdav"}
                <div class="button-row">{#if webDavConnectionDirty}<button class="button primary" type="button" disabled={syncBusy || !webDavTestMatches || (webDavPreflight?.remoteExists && !webDavPreflight.remoteManifestValid)} on:click={applyWebDavConnection}>应用连接设置</button>{/if}{#if syncStatus.status === "localPending" && !syncStatus.lastSyncedAt}<button class="button primary" type="button" disabled={syncBusy || webDavConnectionDirty} on:click={() => configureSync("local")}>上传本地内容</button><button class="button" type="button" disabled={syncBusy || webDavConnectionDirty} on:click={() => configureSync("remote")}>使用远端内容</button>{:else}<button class="button primary" type="button" disabled={syncBusy || webDavConnectionDirty || syncStatus.status === "authRequired"} on:click={() => runSync("auto")}>立即同步</button>{/if}<button class="button danger" type="button" disabled={syncBusy} on:click={disableSync}>关闭同步</button></div>
                {#if syncStatus.lastSyncedAt}<small class="muted-line">上次同步：{new Date(syncStatus.lastSyncedAt).toLocaleString()}</small>{/if}
              {:else}
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy || !webDavTestMatches || !webDavPreflight || (webDavPreflight.remoteExists && !webDavPreflight.remoteManifestValid)} on:click={() => configureSync()}>确认启用 WebDAV</button></div>
              {/if}
            {:else}
              <div class="sync-summary"><strong>{syncStatus.provider === "webdav" ? "WebDAV 内容同步" : "GitHub 内容同步"}</strong><span>{syncStatus.provider === "webdav" ? `${syncStatus.endpoint}/${syncStatus.remoteDir}` : `${syncStatus.repository} · ${syncStatus.branch}`}</span></div>
              <div class="sync-status-row"><span class={`sync-status ${syncStatus.status}`}>{syncStatus.status}</span><span>{syncStatus.message || ""}</span></div>
              {#if syncStatus.status === "localPending" && !syncStatus.lastSyncedAt}
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy} on:click={() => configureSync("local")}>上传本地内容</button><button class="button" type="button" disabled={syncBusy} on:click={() => configureSync("remote")}>使用远端内容</button></div>
              {:else if syncStatus.status === "conflict"}
                <p class="muted-line">本地与远端修改了同一文件，请逐项选择。</p>
                <div class="sync-conflict-list">
                  {#each syncConflicts as conflict}
                    <article class="sync-conflict-card">
                      <strong>{conflict.path}</strong>
                      <span>{conflict.kind === "markdown" ? "Markdown 文本" : `二进制 · 本地 ${conflict.localSize ?? 0} B / 远端 ${conflict.remoteSize ?? 0} B`}</span>
                      {#if conflict.kind === "binary"}<code class="sync-conflict-hashes">本地 {conflict.localHash ?? "已删除"} · 远端 {conflict.remoteHash ?? "已删除"}</code>{/if}
                      {#if conflict.kind === "markdown"}<details><summary>查看两端内容</summary><div class="sync-diff"><pre>{conflict.localText ?? "（本地已删除）"}</pre><pre>{conflict.remoteText ?? "（远端已删除）"}</pre></div></details>{/if}
                      <div class="button-row"><label><input type="radio" name={`sync-${conflict.path}`} value="local" checked={conflictChoices[conflict.path] === "local"} on:change={() => (conflictChoices = { ...conflictChoices, [conflict.path]: "local" })} /> 本地</label><label><input type="radio" name={`sync-${conflict.path}`} value="remote" checked={conflictChoices[conflict.path] === "remote"} on:change={() => (conflictChoices = { ...conflictChoices, [conflict.path]: "remote" })} /> 远端</label></div>
                    </article>
                  {/each}
                </div>
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy || !syncConflicts.length} on:click={submitConflictChoices}>提交冲突选择</button><button class="button" type="button" on:click={() => session && platform.openContentSyncBackups(session.projectId, session.generation)}>打开备份目录</button></div>
              {:else}
                <div class="button-row"><button class="button primary" type="button" disabled={syncBusy} on:click={() => runSync("auto")}>立即同步</button>{#if syncStatus.status === "authRequired"}<button class="button" type="button" disabled={syncBusy} on:click={reconnectSync}>重新认证</button>{/if}<button class="button danger" type="button" disabled={syncBusy} on:click={disableSync}>关闭同步</button></div>
              {/if}
              {#if syncStatus.lastSyncedAt}<small class="muted-line">上次同步：{new Date(syncStatus.lastSyncedAt).toLocaleString()}</small>{/if}
            {/if}
          {/if}
        </div>
      {:else}
        <div class="settings-block">
          <div class="settings-block-heading"><h3>日志策略</h3><p>Token、Authorization 和 URL 凭据写入前会脱敏。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>保留时间</strong><span>启动应用和任务完成后清理过期日志。</span></div><select class="select compact-control" value={draft.diagnostics.logRetentionDays} on:change={(event) => change({ ...draft, diagnostics: { ...draft.diagnostics, logRetentionDays: Number(event.currentTarget.value) as 7 | 14 | 30 } })}><option value="7">7 天</option><option value="14">14 天</option><option value="30">30 天</option></select></div>
          <div class="setting-row"><div class="setting-copy"><strong>总体积上限</strong><span>单任务最多 2MB，全部日志最多 100 个文件。</span></div><select class="select compact-control" value={draft.diagnostics.maxLogStorageMb} on:change={(event) => change({ ...draft, diagnostics: { ...draft.diagnostics, maxLogStorageMb: Number(event.currentTarget.value) as 10 | 20 | 50 } })}><option value="10">10 MB</option><option value="20">20 MB</option><option value="50">50 MB</option></select></div>
        </div>
        <div class="settings-block">
          <div class="setting-subsection-heading"><div><h3>任务日志</h3><span>{logs.length ? `${logs.length} 条记录` : "目前没有日志"}</span></div><div class="button-row"><button class="button quiet" type="button" disabled={logsBusy} on:click={refreshLogs}>刷新</button>{#if logs.length}<button class="button danger" type="button" on:click={clearLogs}>全部清除</button>{/if}</div></div>
          {#if logs.length}<div class="diagnostic-log-list">{#each logs as log (log.taskId)}<div class:active={selectedLog === log.taskId} class="diagnostic-log-row"><button type="button" on:click={() => openLog(log.taskId)}><FileText size={15} /><span><strong>{log.projectName} · {log.taskType}</strong><small>{new Date(log.startedAt).toLocaleString()} · {(log.size / 1024).toFixed(1)} KB{log.truncated ? " · 已截断" : ""}</small></span></button><button class="icon-button" type="button" aria-label="删除日志" title="删除日志" on:click={() => removeLog(log.taskId)}><Trash2 size={14} /></button></div>{/each}</div>{/if}
          {#if selectedLog}<pre class="diagnostic-log-view" aria-label="任务日志内容">{logEvents.map((event) => `${event.timestamp}  ${event.step ?? event.kind}${event.line ? `  ${event.line}` : ""}`).join("\n") || "日志为空。"}</pre>{/if}
        </div>
        <div class="settings-block">
          <div class="settings-block-heading"><h3>更新与恢复</h3><p>更新只从固定的项目 Releases 页面检查。</p></div>
          <div class="setting-row"><div class="setting-copy"><strong>启动时检查更新</strong><span>只有远程 SemVer 更高时才提示。</span></div><label class="switch"><input type="checkbox" checked={draft.update.checkOnStart} on:change={(event) => change({ ...draft, update: { checkOnStart: event.currentTarget.checked } })} /><span></span></label></div>
          <div class="setting-row"><div class="setting-copy"><strong>恢复默认设置</strong><span>Token 不会被删除；默认值保存前仍可取消。</span></div><button class="button" type="button" on:click={() => (showReset = true)}><RotateCcw size={14} />恢复默认</button></div>
        </div>
      {/if}
    </section>
  </div>
</div>

{#if showAcquireToken}
  <ModalDialog title="获取 Cloudflare-ImgBed Token" description="管理员凭据只用于本次登录和创建 Token，不会写入配置或日志。" onClose={closeAcquireToken}>
    <div class="modal-form">
      <label><span>管理员用户名</span><input class="input" data-autofocus autocomplete="username" bind:value={adminUsername} placeholder="按服务端配置填写，可留空" /></label>
      <label><span>管理员密码</span><input class="input" type="password" autocomplete="current-password" bind:value={adminPassword} placeholder="按服务端配置填写，可留空" /></label>
      {#if tokenStatusMessage}<p class="modal-status" role="status">{tokenStatusMessage}</p>{/if}
    </div>
    <svelte:fragment slot="actions"><button class="button" type="button" disabled={credentialBusy} on:click={closeAcquireToken}>取消</button><button class="button primary" type="button" disabled={credentialBusy} on:click={acquireToken}><KeyRound size={14} />{credentialBusy ? "正在获取 Token..." : "获取并保存"}</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if showReset}
  <ModalDialog title="恢复默认设置？" description="默认值会先进入设置草稿，点击保存后才会写入；系统凭据库中的 Token 不受影响。" onClose={() => (showReset = false)}>
    <svelte:fragment slot="actions"><button class="button" type="button" on:click={() => (showReset = false)}>取消</button><button class="button danger" type="button" data-autofocus on:click={restoreDefaults}>恢复默认</button></svelte:fragment>
  </ModalDialog>
{/if}

{#if showClearRecent}
  <ModalDialog title="清空最近项目？" description="只删除最近项目记录，不会删除磁盘上的博客文件。" onClose={() => (showClearRecent = false)}>
    <svelte:fragment slot="actions"><button class="button" type="button" on:click={() => (showClearRecent = false)}>取消</button><button class="button danger" type="button" data-autofocus on:click={clearRecent}>清空记录</button></svelte:fragment>
  </ModalDialog>
{/if}
