<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { FileText, KeyRound, RotateCcw, Save, ShieldCheck, Trash2 } from "@lucide/svelte";
  import PageHeader from "$shared/components/PageHeader.svelte";
  import ModalDialog from "$shared/components/ModalDialog.svelte";
  import { defaultConfig } from "$shared/types/app";
  import { normalizeError, platform } from "$platform/tauri";
  import type { SettingsController } from "./controller";
  import type {
    AppConfigV3,
    CredentialStatus,
    RecentProjectView,
    ThemeMode,
    TaskEvent,
    TaskLogSummary
  } from "$shared/types/app";

  export let config: AppConfigV3;
  export let recentProjects: RecentProjectView[] = [];
  export let onSaveConfig: (config: AppConfigV3) => Promise<AppConfigV3> = async (value) => value;
  export let onThemePreview: (mode: ThemeMode) => void = () => {};
  export let onRegisterSettingsController: (controller: SettingsController | null) => void = () => {};
  export let onRemoveRecentProject: (recentId: string) => Promise<void> = async () => {};
  export let onClearRecentProjects: () => Promise<void> = async () => {};
  export let onNotice: (message: string) => void = () => {};

  let saved = structuredClone(config);
  let draft = structuredClone(config);
  let dirty = false;
  let saving = false;
  let credential: CredentialStatus = { configured: false };
  let token = "";
  let credentialBusy = false;
  let showReset = false;
  let showClearRecent = false;
  let logs: TaskLogSummary[] = [];
  let selectedLog = "";
  let logEvents: TaskEvent[] = [];
  let logsBusy = false;

  $: dirty = JSON.stringify(draft) !== JSON.stringify(saved);

  onMount(() => {
    const controller: SettingsController = {
      save: saveDraft,
      discard,
      hasDirty: () => dirty
    };
    onRegisterSettingsController(controller);
    void refreshCredential();
    void refreshLogs();
  });

  onDestroy(() => onRegisterSettingsController(null));

  function change(next: AppConfigV3) {
    draft = next;
    if (next.appearance.themeMode !== saved.appearance.themeMode) {
      onThemePreview(next.appearance.themeMode);
    }
  }

  async function saveDraft() {
    if (!dirty || saving) return;
    saving = true;
    try {
      const next = await onSaveConfig(structuredClone(draft));
      saved = structuredClone(next);
      draft = structuredClone(next);
      onNotice("设置已保存。");
    } finally {
      saving = false;
    }
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
    try {
      credential = await platform.credentialStatus();
    } catch {
      credential = { configured: false };
    }
  }

  async function saveCredential() {
    credentialBusy = true;
    try {
      credential = await platform.credentialSet(token);
      token = "";
      onNotice("Cloudflare Token 已保存到系统凭据库。");
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      credentialBusy = false;
    }
  }

  async function deleteCredential() {
    credentialBusy = true;
    try {
      credential = await platform.credentialDelete();
      token = "";
      onNotice("Cloudflare Token 已从系统凭据库删除。");
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      credentialBusy = false;
    }
  }

  async function removeRecent(recentId: string) {
    try {
      await onRemoveRecentProject(recentId);
    } catch (error) {
      onNotice(normalizeError(error).message);
    }
  }

  async function clearRecent() {
    try {
      await onClearRecentProjects();
      showClearRecent = false;
    } catch (error) {
      onNotice(normalizeError(error).message);
    }
  }

  async function refreshLogs() {
    logsBusy = true;
    try {
      logs = await platform.listTaskLogs();
      if (selectedLog && !logs.some((log) => log.taskId === selectedLog)) {
        selectedLog = "";
        logEvents = [];
      }
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      logsBusy = false;
    }
  }

  async function openLog(taskId: string) {
    logsBusy = true;
    try {
      const page = await platform.readTaskLog(taskId);
      selectedLog = taskId;
      logEvents = page.events;
    } catch (error) {
      onNotice(normalizeError(error).message);
    } finally {
      logsBusy = false;
    }
  }

  async function removeLog(taskId: string) {
    try {
      await platform.deleteTaskLog(taskId);
      await refreshLogs();
    } catch (error) {
      onNotice(normalizeError(error).message);
    }
  }

  async function clearLogs() {
    try {
      await platform.clearTaskLogs();
      selectedLog = "";
      logEvents = [];
      await refreshLogs();
    } catch (error) {
      onNotice(normalizeError(error).message);
    }
  }
</script>

<div class="workspace-page settings-page">
  <div class="settings-sticky-header">
    <PageHeader title="设置" description="更改会先保留在本页，确认保存后才写入应用配置。">
      <span class:warning={dirty} class:success={!dirty} class="settings-save-state">{saving ? "正在保存" : dirty ? "有未保存更改" : "已保存"}</span>
      <button class="button" type="button" on:click={() => (showReset = true)}><RotateCcw size={15} />恢复默认</button>
      <button class="button" type="button" disabled={!dirty || saving} on:click={discard}>取消</button>
      <button class="button primary" type="button" disabled={!dirty || saving} on:click={saveDraft}><Save size={15} />保存</button>
    </PageHeader>
  </div>

  <div class="settings-sections">
    <section class="panel settings-group">
      <div class="settings-group-heading"><h2>常规</h2><p>启动、保存和最近项目。</p></div>
      <div class="setting-row"><div class="setting-copy"><strong>启动时打开最近项目</strong><span>只恢复上次经过验证的 Hexo 项目。</span></div><label class="switch"><input type="checkbox" checked={draft.general.openRecentProjectOnStart} on:change={(event) => change({ ...draft, general: { ...draft.general, openRecentProjectOnStart: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row"><div class="setting-copy"><strong>自动保存</strong><span>按文章、revision 和项目会话串行保存。</span></div><label class="switch"><input type="checkbox" checked={draft.general.autoSave} on:change={(event) => change({ ...draft, general: { ...draft.general, autoSave: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row"><div class="setting-copy"><strong>自动保存延迟</strong><span>停止输入后等待的毫秒数。</span></div><input class="input compact-control" type="number" min="500" max="30000" step="100" value={draft.general.autoSaveDelayMs} on:change={(event) => change({ ...draft, general: { ...draft.general, autoSaveDelayMs: Number(event.currentTarget.value) } })} /></div>
      <div class="setting-row"><div class="setting-copy"><strong>保存前创建备份</strong><span>在 .hlex-backups 中保留上一版本。</span></div><label class="switch"><input type="checkbox" checked={draft.general.backupBeforeSave} on:change={(event) => change({ ...draft, general: { ...draft.general, backupBeforeSave: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-subsection">
        <div class="setting-subsection-heading"><div><strong>最近项目</strong><span>最多保留 10 个项目，路径仅由后端管理。</span></div>{#if recentProjects.length}<button class="button danger" type="button" on:click={() => (showClearRecent = true)}>清空</button>{/if}</div>
        {#if recentProjects.length}
          <div class="recent-project-list">
            {#each recentProjects as recent (recent.recentId)}
              <div class="recent-project-row"><div><strong>{recent.name}</strong><span>{recent.displayPath}</span></div><span class:warning={!recent.available} class="recent-availability">{recent.available ? "可用" : "不可用"}</span><button class="icon-button" type="button" title="移除记录" aria-label={`移除 ${recent.name}`} on:click={() => removeRecent(recent.recentId)}><Trash2 size={15} /></button></div>
            {/each}
          </div>
        {:else}<p class="muted-line">尚无最近项目。</p>{/if}
      </div>
    </section>

    <section class="panel settings-group">
      <div class="settings-group-heading"><h2>外观</h2><p>仅保留浅色、深色和跟随系统。</p></div>
      <div class="setting-row"><div class="setting-copy"><strong>主题模式</strong><span>选择后立即预览，取消可恢复。</span></div><select class="select compact-control" value={draft.appearance.themeMode} on:change={(event) => change({ ...draft, appearance: { themeMode: event.currentTarget.value as ThemeMode } })}><option value="system">跟随系统</option><option value="light">浅色</option><option value="dark">深色</option></select></div>
    </section>

    <section class="panel settings-group">
      <div class="settings-group-heading"><h2>编辑器</h2><p>正文排版、列表封面与编辑辅助。</p></div>
      <div class="setting-row"><div class="setting-copy"><strong>字号</strong><span>12–28 px。</span></div><input class="input compact-control" type="number" min="12" max="28" value={draft.editor.fontSize} on:change={(event) => change({ ...draft, editor: { ...draft.editor, fontSize: Number(event.currentTarget.value) } })} /></div>
      <div class="setting-row"><div class="setting-copy"><strong>行高</strong><span>1.2–2.2。</span></div><input class="input compact-control" type="number" min="1.2" max="2.2" step="0.05" value={draft.editor.lineHeight} on:change={(event) => change({ ...draft, editor: { ...draft.editor, lineHeight: Number(event.currentTarget.value) } })} /></div>
      <div class="setting-row"><div class="setting-copy"><strong>Tab 宽度</strong><span>使用 2、4 或 8 个空格。</span></div><select class="select compact-control" value={draft.editor.tabSize} on:change={(event) => change({ ...draft, editor: { ...draft.editor, tabSize: Number(event.currentTarget.value) } })}><option value="2">2</option><option value="4">4</option><option value="8">8</option></select></div>
      <div class="setting-row"><div class="setting-copy"><strong>显示行号</strong><span>在正文左侧显示行号栏。</span></div><label class="switch"><input type="checkbox" checked={draft.editor.showLineNumbers} on:change={(event) => change({ ...draft, editor: { ...draft.editor, showLineNumbers: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row"><div class="setting-copy"><strong>自动换行</strong><span>长行按编辑区宽度折行。</span></div><label class="switch"><input type="checkbox" checked={draft.editor.lineWrapping} on:change={(event) => change({ ...draft, editor: { ...draft.editor, lineWrapping: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row"><div class="setting-copy"><strong>突出当前行</strong><span>使用低对比背景标识光标行。</span></div><label class="switch"><input type="checkbox" checked={draft.editor.highlightActiveLine} on:change={(event) => change({ ...draft, editor: { ...draft.editor, highlightActiveLine: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row"><div class="setting-copy"><strong>文章列表封面</strong><span>在每篇文章标题左侧显示 42px 缩略图。</span></div><label class="switch"><input type="checkbox" checked={draft.articleList.showCover} on:change={(event) => change({ ...draft, articleList: { showCover: event.currentTarget.checked } })} /><span></span></label></div>
    </section>

    <section class="panel settings-group">
      <div class="settings-group-heading"><h2>Hexo</h2><p>本地预览服务仅作为博客子进程运行。</p></div>
      <div class="setting-row"><div class="setting-copy"><strong>预览端口</strong><span>默认使用 4000。</span></div><input class="input compact-control" type="number" min="300" max="65535" value={draft.hexo.previewPort} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, previewPort: Number(event.currentTarget.value) } })} /></div>
      <div class="setting-row"><div class="setting-copy"><strong>打开项目后自动启动预览</strong><span>只在后台启动 Hexo Server，不自动弹出浏览器或日志。</span></div><label class="switch"><input type="checkbox" checked={draft.hexo.autoStartPreview} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, autoStartPreview: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row"><div class="setting-copy"><strong>预览草稿</strong><span>启动 Hexo Server 时使用固定的 --draft 参数。</span></div><label class="switch"><input type="checkbox" checked={draft.hexo.previewDrafts} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, previewDrafts: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row"><div class="setting-copy"><strong>默认预览模式</strong><span>即时 Markdown 或博客真实主题。</span></div><select class="select compact-control" value={draft.hexo.defaultPreviewMode} on:change={(event) => change({ ...draft, hexo: { ...draft.hexo, defaultPreviewMode: event.currentTarget.value as "markdown" | "theme" } })}><option value="markdown">Markdown</option><option value="theme">主题预览</option></select></div>
    </section>

    <section class="panel settings-group">
      <div class="settings-group-heading"><h2>图床</h2><p>本地图片与 Cloudflare-ImgBed。</p></div>
      <div class="setting-row"><div class="setting-copy"><strong>默认来源</strong><span>决定编辑器导入、粘贴和拖入图片的目标。</span></div><select class="select compact-control" value={draft.imageBed.defaultProvider} on:change={(event) => change({ ...draft, imageBed: { ...draft.imageBed, defaultProvider: event.currentTarget.value as AppConfigV3["imageBed"]["defaultProvider"] } })}><option value="local">本地图片</option><option value="cloudflare-imgbed">Cloudflare-ImgBed</option></select></div>
      <div class="setting-row"><div class="setting-copy"><strong>Cloudflare API 地址</strong><span>生产环境必须使用 HTTPS。</span></div><input class="input control-wide" type="url" value={draft.imageBed.cloudflareApiUrl} placeholder="https://…" on:change={(event) => change({ ...draft, imageBed: { ...draft.imageBed, cloudflareApiUrl: event.currentTarget.value } })} /></div>
      <div class="setting-row"><div class="setting-copy"><strong>上传后插入 Markdown</strong><span>成功后插入当前文章的最后光标位置。</span></div><label class="switch"><input type="checkbox" checked={draft.imageBed.autoInsertMarkdown} on:change={(event) => change({ ...draft, imageBed: { ...draft.imageBed, autoInsertMarkdown: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row credential-row"><div class="setting-copy"><strong>Cloudflare Token</strong><span>只写入系统凭据库，应用不会回显明文。</span></div><div class="credential-control"><input class="input" type="password" bind:value={token} autocomplete="new-password" placeholder={credential.configured ? "已配置，输入新值可替换" : "输入 Token"} /><div class="button-row"><button class="button secondary" type="button" disabled={credentialBusy || !token.trim()} on:click={saveCredential}><KeyRound size={14} />保存凭据</button>{#if credential.configured}<button class="button danger" type="button" disabled={credentialBusy} on:click={deleteCredential}>删除</button>{/if}</div></div></div>
      <div class="setting-row"><div class="setting-copy"><strong>凭据状态</strong><span>状态来自系统凭据库。</span></div><span class:success={credential.configured} class:warning={!credential.configured} class="credential-status"><ShieldCheck size={14} />{credential.configured ? "已配置" : "未配置"}</span></div>
    </section>

    <section class="panel settings-group">
      <div class="settings-group-heading"><h2>发布</h2><p>发布快捷键为 Ctrl+Shift+P。</p></div>
      <div class="setting-row"><div class="setting-copy"><strong>运行前保存文章</strong><span>保存失败时中止发布并保留内容。</span></div><label class="switch"><input type="checkbox" checked={draft.publish.saveBeforeRun} on:change={(event) => change({ ...draft, publish: { ...draft.publish, saveBeforeRun: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row"><div class="setting-copy"><strong>生成前清理</strong><span>在 publish 流程中先执行 hexo clean。</span></div><label class="switch"><input type="checkbox" checked={draft.publish.cleanBeforeGenerate} on:change={(event) => change({ ...draft, publish: { ...draft.publish, cleanBeforeGenerate: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row"><div class="setting-copy"><strong>部署前生成</strong><span>在 deploy 前执行 hexo generate。</span></div><label class="switch"><input type="checkbox" checked={draft.publish.generateBeforeDeploy} on:change={(event) => change({ ...draft, publish: { ...draft.publish, generateBeforeDeploy: event.currentTarget.checked } })} /><span></span></label></div>
      <div class="setting-row"><div class="setting-copy"><strong>部署后 Git Push</strong><span>执行固定 git push，不接受自定义参数。</span></div><label class="switch"><input type="checkbox" checked={draft.publish.gitPushAfterDeploy} on:change={(event) => change({ ...draft, publish: { ...draft.publish, gitPushAfterDeploy: event.currentTarget.checked } })} /><span></span></label></div>
    </section>

    <section class="panel settings-group diagnostics-group">
      <div class="settings-group-heading"><h2>诊断与日志</h2><p>日志只在这里由你主动查看，Token、Authorization 和 URL 凭据会在写入前脱敏。</p></div>
      <div class="setting-row"><div class="setting-copy"><strong>保留时间</strong><span>启动应用和任务完成后自动清理过期日志。</span></div><select class="select compact-control" value={draft.diagnostics.logRetentionDays} on:change={(event) => change({ ...draft, diagnostics: { ...draft.diagnostics, logRetentionDays: Number(event.currentTarget.value) as 7 | 14 | 30 } })}><option value="7">7 天</option><option value="14">14 天</option><option value="30">30 天</option></select></div>
      <div class="setting-row"><div class="setting-copy"><strong>总体积上限</strong><span>单任务最多 2MB，全部日志最多 100 个文件。</span></div><select class="select compact-control" value={draft.diagnostics.maxLogStorageMb} on:change={(event) => change({ ...draft, diagnostics: { ...draft.diagnostics, maxLogStorageMb: Number(event.currentTarget.value) as 10 | 20 | 50 } })}><option value="10">10 MB</option><option value="20">20 MB</option><option value="50">50 MB</option></select></div>
      <div class="setting-subsection">
        <div class="setting-subsection-heading"><div><strong>任务日志</strong><span>{logs.length ? `${logs.length} 条记录` : "目前没有日志"}</span></div><div class="button-row"><button class="button quiet" type="button" disabled={logsBusy} on:click={refreshLogs}>刷新</button>{#if logs.length}<button class="button danger" type="button" on:click={clearLogs}>全部清除</button>{/if}</div></div>
        {#if logs.length}
          <div class="diagnostic-log-list">
            {#each logs as log (log.taskId)}
              <div class:active={selectedLog === log.taskId} class="diagnostic-log-row"><button type="button" on:click={() => openLog(log.taskId)}><FileText size={15} /><span><strong>{log.projectName} · {log.taskType}</strong><small>{new Date(log.startedAt).toLocaleString()} · {(log.size / 1024).toFixed(1)} KB{log.truncated ? " · 已截断" : ""}</small></span></button><button class="icon-button" type="button" aria-label="删除日志" title="删除日志" on:click={() => removeLog(log.taskId)}><Trash2 size={14} /></button></div>
            {/each}
          </div>
        {/if}
        {#if selectedLog}
          <pre class="diagnostic-log-view" aria-label="任务日志内容">{logEvents.map((event) => `${event.timestamp}  ${event.step ?? event.kind}${event.line ? `  ${event.line}` : ""}`).join("\n") || "日志为空。"}</pre>
        {/if}
      </div>
    </section>

    <section class="panel settings-group">
      <div class="settings-group-heading"><h2>更新</h2><p>固定检查项目 GitHub Releases。</p></div>
      <div class="setting-row"><div class="setting-copy"><strong>启动时检查更新</strong><span>只有远程 SemVer 更高时才提示。</span></div><label class="switch"><input type="checkbox" checked={draft.update.checkOnStart} on:change={(event) => change({ ...draft, update: { checkOnStart: event.currentTarget.checked } })} /><span></span></label></div>
    </section>
  </div>
</div>

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
