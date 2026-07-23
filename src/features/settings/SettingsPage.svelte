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
    CredentialStatus,
    RecentProjectView,
    SettingsSectionId,
    ThemeMode,
    TaskEvent,
    TaskLogSummary
  } from "$shared/types/app";

  export let config: AppConfigV3;
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

  $: dirty = JSON.stringify(draft) !== JSON.stringify(saved);
  $: currentSection = sections.find((section) => section.id === activeSection) ?? sections[0];
  $: dirtySections = {
    general: JSON.stringify(draft.general) !== JSON.stringify(saved.general),
    editing: JSON.stringify([draft.appearance, draft.editor, draft.articleList]) !== JSON.stringify([saved.appearance, saved.editor, saved.articleList]),
    images: JSON.stringify(draft.imageBed) !== JSON.stringify(saved.imageBed),
    hexoPublish: JSON.stringify([draft.hexo, draft.publish]) !== JSON.stringify([saved.hexo, saved.publish]),
    maintenance: JSON.stringify([draft.diagnostics, draft.update]) !== JSON.stringify([saved.diagnostics, saved.update])
  } satisfies Record<SettingsSectionId, boolean>;

  onMount(() => {
    const stored = localStorage.getItem(sectionStorageKey) as SettingsSectionId | null;
    activeSection = initialSection ?? (sections.some((section) => section.id === stored) ? stored! : "general");
    onRegisterSettingsController({ save: saveDraft, discard, hasDirty: () => dirty });
    void refreshCredential();
    void refreshLogs();
  });

  onDestroy(() => onRegisterSettingsController(null));

  function selectSection(section: SettingsSectionId) {
    activeSection = section;
    localStorage.setItem(sectionStorageKey, section);
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
