<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    Brush,
    CheckCircle2,
    Code2,
    Download,
    FileCog,
    FolderCog,
    Info,
    KeyRound,
    ClipboardPaste,
    GitPullRequest,
    RefreshCw,
    RotateCcw,
    Rocket,
    Settings,
    UploadCloud,
    X
  } from "@lucide/svelte";
  import MarkdownEditor from "$lib/components/MarkdownEditor.svelte";
  import { checkUpdate, getAppVersion, openConfigDir, openReleasePage } from "$lib/api/app";
  import {
    backupHexoConfig,
    backupHexoConfigFile,
    listHexoConfigFiles,
    openHexoConfigExternal,
    openHexoConfigFileExternal,
    readHexoConfig,
    readHexoConfigFile,
    restoreLatestHexoConfigFileBackup,
    restoreLatestHexoConfigBackup,
    saveHexoConfig,
    saveHexoConfigFile,
    type HexoConfigEntry,
    type HexoConfigFile
  } from "$lib/api/hexoConfig";
  import { copyImageToProject, openCloudflareImgBedAdmin, uploadImagePathToCloudflareImgBed } from "$lib/api/uploader";
  import { resetSettings } from "$lib/api/config";
  import { getContentSyncStatus, initContentSync, pullContentSync, runContentSync } from "$lib/api/sync";
  import type { HexoProject } from "$lib/types/project";
  import type { AppSettings, UpdateCheckResult } from "$lib/types/settings";
  import type { ContentSyncStatus } from "$lib/types/sync";

  type Tab = "general" | "appearance" | "editor" | "hexo" | "uploader" | "sync" | "publish" | "update" | "about";

  export let settings: AppSettings;
  export let project: HexoProject | null = null;
  export let onChange: (settings: AppSettings) => void | Promise<void> = () => {};
  export let onClose: () => void = () => {};

  let activeTab: Tab = "general";
  let appVersion = "";
  let hexoConfig: HexoConfigFile | null = null;
  let hexoConfigContent = "";
  let hexoMessage = "未读取配置文件。";
  let hexoConfigEntries: HexoConfigEntry[] = [];
  let selectedConfigPath = "";
  let uploadTestResult = "";
  let syncStatus: ContentSyncStatus | null = null;
  let syncMessage = "等待检查。";
  let syncRunning = false;
  let updateResult: UpdateCheckResult | null = null;
  let updateMessage = "等待检查。";

  const tabs: { id: Tab; label: string; icon: typeof Settings }[] = [
    { id: "general", label: "常规", icon: Settings },
    { id: "appearance", label: "外观", icon: Brush },
    { id: "editor", label: "编辑器", icon: Code2 },
    { id: "hexo", label: "Hexo 配置", icon: FileCog },
    { id: "uploader", label: "图床", icon: UploadCloud },
    { id: "sync", label: "同步", icon: RefreshCw },
    { id: "publish", label: "发布", icon: Rocket },
    { id: "update", label: "更新", icon: Download },
    { id: "about", label: "关于", icon: Info }
  ];

  onMount(async () => {
    appVersion = await getAppVersion();
  });

  async function commit(next: AppSettings) {
    settings = next;
    await onChange(settings);
  }

  function patch<K extends keyof AppSettings, F extends keyof AppSettings[K]>(
    section: K,
    field: F,
    value: AppSettings[K][F]
  ) {
    commit({
      ...settings,
      [section]: {
        ...(settings[section] as object),
        [field]: value
      }
    } as AppSettings);
  }

  async function resetAllSettings() {
    if (!confirm("确定要重置所有设置吗？")) return;
    await commit(await resetSettings());
  }

  async function refreshHexoConfigEntries() {
    const currentProject = project;
    if (!currentProject) {
      hexoMessage = "请先打开 Hexo 项目。";
      return;
    }
    try {
      hexoConfigEntries = await listHexoConfigFiles(currentProject.rootPath);
      const preferred =
        hexoConfigEntries.find((entry) => entry.kind === "root") ??
        hexoConfigEntries.find((entry) => entry.is_active_theme) ??
        hexoConfigEntries[0];
      selectedConfigPath = selectedConfigPath || preferred?.path || "";
      if (selectedConfigPath) await loadSelectedHexoConfig();
      else hexoMessage = "没有找到可编辑的 Hexo 配置文件。";
    } catch (error) {
      hexoMessage = `扫描配置失败: ${error}`;
    }
  }

  async function loadSelectedHexoConfig() {
    if (!selectedConfigPath) {
      await refreshHexoConfigEntries();
      return;
    }
    try {
      hexoConfig = await readHexoConfigFile(selectedConfigPath);
      hexoConfigContent = hexoConfig.content;
      hexoMessage = hexoConfig.exists ? "已读取配置文件。" : "配置文件不存在。";
    } catch (error) {
      hexoMessage = `读取失败: ${error}`;
    }
  }

  async function saveSelectedHexoConfig() {
    if (!selectedConfigPath) return;
    try {
      const result = await saveHexoConfigFile(selectedConfigPath, hexoConfigContent);
      hexoMessage = `已保存，备份: ${result.backup_path}`;
      await loadSelectedHexoConfig();
    } catch (error) {
      hexoMessage = `保存失败: ${error}`;
    }
  }

  async function backupSelectedHexoConfig() {
    if (!selectedConfigPath) return;
    try {
      const result = await backupHexoConfigFile(selectedConfigPath);
      hexoMessage = `已备份: ${result.backup_path}`;
      await loadSelectedHexoConfig();
    } catch (error) {
      hexoMessage = `备份失败: ${error}`;
    }
  }

  async function restoreSelectedHexoConfig() {
    if (!selectedConfigPath || !confirm("确定恢复最近一次该配置文件备份吗？当前内容会被覆盖。")) return;
    try {
      hexoConfig = await restoreLatestHexoConfigFileBackup(selectedConfigPath);
      hexoConfigContent = hexoConfig.content;
      hexoMessage = "已恢复最近备份。";
    } catch (error) {
      hexoMessage = `恢复失败: ${error}`;
    }
  }

  async function openSelectedHexoConfigExternal() {
    if (!selectedConfigPath) return;
    try {
      await openHexoConfigFileExternal(selectedConfigPath);
    } catch (error) {
      hexoMessage = `外部打开失败: ${error}`;
    }
  }

  async function loadHexoConfig() {
    const currentProject = project;
    if (!currentProject) {
      hexoMessage = "请先打开 Hexo 项目。";
      return;
    }
    try {
      hexoConfig = await readHexoConfig(currentProject.rootPath);
      hexoConfigContent = hexoConfig.content;
      hexoMessage = hexoConfig.exists ? "已读取 _config.yml。" : "当前项目没有 _config.yml。";
    } catch (error) {
      hexoMessage = `读取失败: ${error}`;
    }
  }

  async function saveCurrentHexoConfig() {
    if (!project) return;
    try {
      const result = await saveHexoConfig(project.rootPath, hexoConfigContent);
      hexoMessage = `已保存，备份: ${result.backup_path}`;
      await loadHexoConfig();
    } catch (error) {
      hexoMessage = `保存失败: ${error}`;
    }
  }

  async function backupCurrentHexoConfig() {
    if (!project) return;
    try {
      const result = await backupHexoConfig(project.rootPath);
      hexoMessage = `已备份: ${result.backup_path}`;
      await loadHexoConfig();
    } catch (error) {
      hexoMessage = `备份失败: ${error}`;
    }
  }

  async function restoreHexoConfig() {
    if (!project || !confirm("确定恢复最近一次 _config.yml 备份吗？当前配置会被覆盖。")) return;
    try {
      hexoConfig = await restoreLatestHexoConfigBackup(project.rootPath);
      hexoConfigContent = hexoConfig.content;
      hexoMessage = "已恢复最近备份。";
    } catch (error) {
      hexoMessage = `恢复失败: ${error}`;
    }
  }

  async function testUpload() {
    if (!project && settings.uploader.defaultType !== "cloudflare-imgbed") {
      uploadTestResult = "请先打开 Hexo 项目。";
      return;
    }
    const selected = await open({
      multiple: false,
      title: "选择测试上传图片",
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg"] }]
    });
    if (typeof selected !== "string") return;
    try {
      const result =
        settings.uploader.defaultType === "cloudflare-imgbed"
          ? await uploadImagePathToCloudflareImgBed(settings.uploader.apiUrl, settings.uploader.token, selected)
          : await copyImageToProject(project!.rootPath, selected);
      uploadTestResult = `${result.url}\n${result.markdown}`;
    } catch (error) {
      uploadTestResult = `测试上传失败: ${error}`;
    }
  }

  async function copyUploadMarkdown() {
    const markdown = uploadTestResult.split("\n").find((line) => line.startsWith("!["));
    if (markdown) await navigator.clipboard.writeText(markdown);
  }

  async function openUploaderTokenPage() {
    try {
      await openCloudflareImgBedAdmin(settings.uploader.apiUrl);
      uploadTestResult = "已打开图床后台。请创建具备 upload/list/delete 权限的 API Token，然后复制回来。";
    } catch (error) {
      uploadTestResult = `打开图床后台失败: ${error}`;
    }
  }

  async function pasteUploaderToken() {
    try {
      const token = (await navigator.clipboard.readText()).trim();
      if (!token) {
        uploadTestResult = "剪贴板里没有可用的 Token。";
        return;
      }
      await commit({
        ...settings,
        uploader: {
          ...settings.uploader,
          defaultType: "cloudflare-imgbed",
          token
        }
      });
      uploadTestResult = "Token 已从剪贴板填入并保存。";
    } catch (error) {
      uploadTestResult = `读取剪贴板失败: ${error}`;
    }
  }

  async function runUpdateCheck() {
    updateMessage = "正在检查更新...";
    updateResult = null;
    try {
      updateResult = await checkUpdate(settings.update);
      updateMessage = updateResult.hasUpdate ? "发现新版本。" : "当前已是最新版。";
    } catch (error) {
      updateMessage = `检查失败: ${error}`;
    }
  }

  async function refreshSyncStatus() {
    if (!project) {
      syncMessage = "请先打开 Hexo 项目。";
      syncStatus = null;
      return;
    }
    try {
      syncStatus = await getContentSyncStatus(project.rootPath, settings.sync);
      syncMessage = syncStatus.message;
    } catch (error) {
      syncMessage = `检查同步状态失败: ${error}`;
    }
  }

  async function enableAndInitSync() {
    if (!project) {
      syncMessage = "请先打开 Hexo 项目。";
      return;
    }
    syncRunning = true;
    const next = {
      ...settings,
      sync: {
        ...settings.sync,
        enabled: true
      }
    };
    try {
      await commit(next);
      syncStatus = await initContentSync(project.rootPath, next.sync);
      syncMessage = syncStatus.message;
    } catch (error) {
      syncMessage = `初始化同步失败: ${error}`;
    } finally {
      syncRunning = false;
    }
  }

  async function pullSync() {
    if (!project) return;
    syncRunning = true;
    try {
      syncStatus = await pullContentSync(project.rootPath, settings.sync);
      syncMessage = syncStatus.message;
    } catch (error) {
      syncMessage = `拉取同步失败: ${error}`;
    } finally {
      syncRunning = false;
    }
  }

  async function runSync() {
    if (!project) return;
    syncRunning = true;
    try {
      syncStatus = await runContentSync(project.rootPath, settings.sync);
      syncMessage = syncStatus.message;
      if (syncStatus.status !== "conflict" && syncStatus.status !== "error") {
        await commit({
          ...settings,
          sync: {
            ...settings.sync,
            lastSyncAt: new Date().toISOString()
          }
        });
      }
    } catch (error) {
      syncMessage = `同步失败: ${error}`;
    } finally {
      syncRunning = false;
    }
  }

  function syncStatusText(status?: string) {
    if (!status) return "未检查";
    if (status === "notConfigured") return "未配置";
    if (status === "ready") return "已就绪";
    if (status === "syncing") return "同步中";
    if (status === "hasLocalChanges") return "有本地变更";
    if (status === "needsPull") return "需要拉取";
    if (status === "needsPush") return "需要推送";
    if (status === "conflict") return "存在冲突";
    if (status === "error") return "异常";
    return status;
  }
</script>

<aside class="settings-panel">
  <div class="settings-panel-header">
    <div>
      <h2>设置</h2>
      <span>1.0.1 核心配置</span>
    </div>
    <button class="icon-only" title="关闭" on:click={onClose}><X size={18} /></button>
  </div>

  <nav class="settings-tabs">
    {#each tabs as tab}
      <button class:active={activeTab === tab.id} on:click={() => (activeTab = tab.id)}>
        <svelte:component this={tab.icon} size={15} />
        {tab.label}
      </button>
    {/each}
  </nav>

  <div class="settings-panel-body">
    {#if activeTab === "general"}
      <section class="settings-section">
        <h3>常规</h3>
        <label class="toggle-row"><input type="checkbox" checked={settings.general.openRecentProjectOnStart} on:change={(event) => patch("general", "openRecentProjectOnStart", event.currentTarget.checked)} /><span>启动时打开最近项目</span></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.general.autoSave} on:change={(event) => patch("general", "autoSave", event.currentTarget.checked)} /><span>自动保存</span></label>
        <label><span>自动保存间隔 ms</span><input type="number" min="1000" step="500" value={settings.general.autoSaveInterval} on:input={(event) => patch("general", "autoSaveInterval", Number(event.currentTarget.value))} /></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.general.backupBeforeSave} on:change={(event) => patch("general", "backupBeforeSave", event.currentTarget.checked)} /><span>保存前自动备份文章</span></label>
        <label><span>日志保留数量</span><input type="number" min="50" max="5000" value={settings.general.maxLogCount} on:input={(event) => patch("general", "maxLogCount", Number(event.currentTarget.value))} /></label>
        <div class="button-row">
          <button on:click={openConfigDir}><FolderCog size={16} />打开配置目录</button>
          <button class="danger" on:click={resetAllSettings}><RotateCcw size={16} />重置设置</button>
        </div>
      </section>
    {:else if activeTab === "appearance"}
      <section class="settings-section">
        <h3>外观</h3>
        <label><span>主题模式</span><select value={settings.appearance.themeMode} on:change={(event) => patch("appearance", "themeMode", event.currentTarget.value as AppSettings["appearance"]["themeMode"])}><option value="system">跟随系统</option><option value="light">浅色模式</option><option value="dark">深色模式</option></select></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.appearance.compactMode} on:change={(event) => patch("appearance", "compactMode", event.currentTarget.checked)} /><span>紧凑模式</span></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.appearance.showPostCover} on:change={(event) => patch("appearance", "showPostCover", event.currentTarget.checked)} /><span>文章列表显示封面</span></label>
        <label><span>字体缩放</span><input type="number" min="0.85" max="1.25" step="0.05" value={settings.appearance.fontScale} on:input={(event) => patch("appearance", "fontScale", Number(event.currentTarget.value))} /></label>
      </section>
    {:else if activeTab === "editor"}
      <section class="settings-section">
        <h3>编辑器</h3>
        <label><span>字体大小</span><input type="number" min="12" max="24" value={settings.editor.fontSize} on:input={(event) => patch("editor", "fontSize", Number(event.currentTarget.value))} /></label>
        <label><span>行高</span><input type="number" min="1.2" max="2.2" step="0.1" value={settings.editor.lineHeight} on:input={(event) => patch("editor", "lineHeight", Number(event.currentTarget.value))} /></label>
        <label><span>Tab 宽度</span><input type="number" min="2" max="8" value={settings.editor.tabSize} on:input={(event) => patch("editor", "tabSize", Number(event.currentTarget.value))} /></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.editor.showLineNumbers} on:change={(event) => patch("editor", "showLineNumbers", event.currentTarget.checked)} /><span>显示行号</span></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.editor.lineWrapping} on:change={(event) => patch("editor", "lineWrapping", event.currentTarget.checked)} /><span>自动换行</span></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.editor.highlightActiveLine} on:change={(event) => patch("editor", "highlightActiveLine", event.currentTarget.checked)} /><span>高亮当前行</span></label>
      </section>
    {:else if activeTab === "hexo"}
      <section class="settings-section hexo-drawer-section">
        <h3>Hexo 配置</h3>
        <div class="setting-card">
          <small>当前项目：{project?.rootPath ?? "未打开项目"}</small>
          <small>配置文件：{hexoConfig?.config_path ?? "未读取"}</small>
          <small>{hexoMessage}</small>
        </div>
        <label>
          <span>配置文件</span>
          <select
            disabled={!project || !hexoConfigEntries.length}
            value={selectedConfigPath}
            on:change={async (event) => {
              selectedConfigPath = event.currentTarget.value;
              await loadSelectedHexoConfig();
            }}
          >
            {#each hexoConfigEntries as entry}
              <option value={entry.path}>
                {entry.label}{entry.is_active_theme ? "（当前主题）" : ""}{entry.exists ? "" : "（不存在）"}
              </option>
            {/each}
          </select>
        </label>
        <div class="button-row">
          <button disabled={!project} on:click={refreshHexoConfigEntries}>扫描</button>
          <button disabled={!project || !selectedConfigPath} on:click={loadSelectedHexoConfig}>打开</button>
          <button disabled={!project || !hexoConfig?.exists} on:click={saveSelectedHexoConfig}>保存</button>
          <button disabled={!project || !hexoConfig?.exists} on:click={backupSelectedHexoConfig}>备份</button>
          <button disabled={!project || !hexoConfig?.latest_backup_path} on:click={restoreSelectedHexoConfig}>恢复</button>
          <button title="外部打开配置文件" disabled={!project || !hexoConfig?.exists} on:click={openSelectedHexoConfigExternal}>外部</button>
        </div>
        <div class="drawer-yaml-editor">
          {#if project}
            <MarkdownEditor
              content={hexoConfigContent}
              syntax="yaml"
              colorScheme={settings.appearance.colorScheme}
              fontSize={settings.editor.fontSize}
              lineHeight={settings.editor.lineHeight}
              showLineNumbers={settings.editor.showLineNumbers}
              lineWrapping={settings.editor.lineWrapping}
              highlightLine={settings.editor.highlightActiveLine}
              tabSize={settings.editor.tabSize}
              onChange={(value) => (hexoConfigContent = value)}
              onSave={saveSelectedHexoConfig}
            />
          {:else}
            <div class="settings-empty">请先打开 Hexo 项目。</div>
          {/if}
        </div>
      </section>
    {:else if activeTab === "uploader"}
      <section class="settings-section">
        <h3>图床</h3>
        <label><span>默认图床类型</span><select value={settings.uploader.defaultType} on:change={(event) => patch("uploader", "defaultType", event.currentTarget.value as AppSettings["uploader"]["defaultType"])}><option value="local">本地图床</option><option value="cloudflare-imgbed">CloudFlare-ImgBed</option><option value="custom">自定义 API</option><option value="smms">SM.MS</option></select></label>
        <label><span>API 地址</span><input placeholder={settings.uploader.defaultType === "cloudflare-imgbed" ? "https://your-imgbed.example.com 或 https://your-imgbed.example.com/upload" : ""} value={settings.uploader.apiUrl} on:input={(event) => patch("uploader", "apiUrl", event.currentTarget.value)} /></label>
        <label><span>Token / authCode</span><input value={settings.uploader.token} on:input={(event) => patch("uploader", "token", event.currentTarget.value)} /></label>
        <label><span>请求方式</span><select value={settings.uploader.method} on:change={(event) => patch("uploader", "method", event.currentTarget.value as "POST" | "PUT")}><option value="POST">POST</option><option value="PUT">PUT</option></select></label>
        <label><span>文件字段名</span><input value={settings.uploader.fileField} on:input={(event) => patch("uploader", "fileField", event.currentTarget.value)} /></label>
        <label><span>返回 URL 字段</span><input value={settings.uploader.urlField} on:input={(event) => patch("uploader", "urlField", event.currentTarget.value)} /></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.uploader.autoInsertMarkdown} on:change={(event) => patch("uploader", "autoInsertMarkdown", event.currentTarget.checked)} /><span>上传后自动插入 Markdown</span></label>
        <div class="button-row">
          <button on:click={openUploaderTokenPage}><KeyRound size={16} />获取 Token</button>
          <button on:click={pasteUploaderToken}><ClipboardPaste size={16} />从剪贴板填入 Token</button>
          <button on:click={testUpload}>测试上传</button>
          <button disabled={!uploadTestResult.includes("![")} on:click={copyUploadMarkdown}>复制 Markdown</button>
        </div>
        <pre class="result-box">{uploadTestResult || (settings.uploader.defaultType === "cloudflare-imgbed" ? "CloudFlare-ImgBed 使用 POST /upload，表单字段 file，上传成功后返回 Markdown。" : "本地图床测试会复制图片到 source/images。")}</pre>
      </section>
    {:else if activeTab === "sync"}
      <section class="settings-section">
        <h3>内容同步</h3>
        <div class="setting-card">
          <small>当前项目：{project?.rootPath ?? "未打开项目"}</small>
          <small>状态：{syncStatusText(syncStatus?.status)}</small>
          <small>同步分支：{syncStatus?.remoteName ?? settings.sync.remoteName}/{syncStatus?.branchName ?? settings.sync.branchName}</small>
          <small>隐藏工作区：{syncStatus?.worktreePath ?? "未初始化"}</small>
          <small>Ahead / Behind：{syncStatus?.ahead ?? 0} / {syncStatus?.behind ?? 0}</small>
          <small>最近同步：{settings.sync.lastSyncAt ?? "从未同步"}</small>
          <small>{syncMessage}</small>
          {#if syncStatus?.conflicts.length}
            <pre>{syncStatus.conflicts.join("\n")}</pre>
          {/if}
        </div>
        <label class="toggle-row"><input type="checkbox" checked={settings.sync.enabled} on:change={(event) => patch("sync", "enabled", event.currentTarget.checked)} /><span>启用内容同步</span></label>
        <label><span>Git Remote</span><input value={settings.sync.remoteName} on:input={(event) => patch("sync", "remoteName", event.currentTarget.value)} /></label>
        <label><span>同步分支</span><input value={settings.sync.branchName} on:input={(event) => patch("sync", "branchName", event.currentTarget.value)} /></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.sync.autoSaveBeforeSync} on:change={(event) => patch("sync", "autoSaveBeforeSync", event.currentTarget.checked)} /><span>同步前自动保存当前文章</span></label>
        <div class="button-row">
          <button disabled={!project || syncRunning} on:click={refreshSyncStatus}><RefreshCw size={16} />检查状态</button>
          <button disabled={!project || syncRunning} on:click={enableAndInitSync}><GitPullRequest size={16} />初始化同步分支</button>
          <button disabled={!project || syncRunning || !settings.sync.enabled} on:click={pullSync}>从分支拉取</button>
          <button disabled={!project || syncRunning || !settings.sync.enabled} on:click={runSync}>同步到分支</button>
        </div>
        <pre class="result-box">同步范围：source/_posts、source/_drafts、source/images。冲突时会停止，不会覆盖任一设备的内容。</pre>
      </section>
    {:else if activeTab === "publish"}
      <section class="settings-section">
        <h3>发布</h3>
        <label><span>Hexo Server 命令</span><input value={settings.publish.hexoServerCommand} on:input={(event) => patch("publish", "hexoServerCommand", event.currentTarget.value)} /></label>
        <label><span>Hexo Clean 命令</span><input value={settings.publish.hexoCleanCommand} on:input={(event) => patch("publish", "hexoCleanCommand", event.currentTarget.value)} /></label>
        <label><span>Hexo Generate 命令</span><input value={settings.publish.hexoGenerateCommand} on:input={(event) => patch("publish", "hexoGenerateCommand", event.currentTarget.value)} /></label>
        <label><span>Hexo Deploy 命令</span><input value={settings.publish.hexoDeployCommand} on:input={(event) => patch("publish", "hexoDeployCommand", event.currentTarget.value)} /></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.publish.saveBeforePublish} on:change={(event) => patch("publish", "saveBeforePublish", event.currentTarget.checked)} /><span>发布前自动保存文章</span></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.publish.cleanBeforeGenerate} on:change={(event) => patch("publish", "cleanBeforeGenerate", event.currentTarget.checked)} /><span>生成前自动 clean</span></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.publish.generateBeforeDeploy} on:change={(event) => patch("publish", "generateBeforeDeploy", event.currentTarget.checked)} /><span>发布前自动 generate</span></label>
        <label class="toggle-row"><input type="checkbox" checked={settings.publish.gitPushAfterDeploy} on:change={(event) => patch("publish", "gitPushAfterDeploy", event.currentTarget.checked)} /><span>发布后自动 git push</span></label>
      </section>
    {:else if activeTab === "update"}
      <section class="settings-section">
        <h3>更新</h3>
        <label class="toggle-row"><input type="checkbox" checked={settings.update.checkUpdateOnStart} on:change={(event) => patch("update", "checkUpdateOnStart", event.currentTarget.checked)} /><span>启动时检查更新</span></label>
        <label><span>更新来源</span><select value={settings.update.updateSource} on:change={(event) => patch("update", "updateSource", event.currentTarget.value as AppSettings["update"]["updateSource"])}><option value="github">GitHub Releases</option><option value="custom">自定义 JSON</option></select></label>
        <label><span>GitHub Owner</span><input value={settings.update.githubOwner} on:input={(event) => patch("update", "githubOwner", event.currentTarget.value)} /></label>
        <label><span>GitHub Repo</span><input value={settings.update.githubRepo} on:input={(event) => patch("update", "githubRepo", event.currentTarget.value)} /></label>
        <label><span>自定义更新地址</span><input value={settings.update.customUpdateUrl} on:input={(event) => patch("update", "customUpdateUrl", event.currentTarget.value)} /></label>
        <div class="button-row">
          <button on:click={runUpdateCheck}><CheckCircle2 size={16} />检查更新</button>
          <button disabled={!updateResult?.releasePageUrl && !updateResult?.downloadUrl} on:click={() => updateResult && openReleasePage(updateResult.releasePageUrl || updateResult.downloadUrl || "")}>打开下载页</button>
        </div>
        <div class="setting-card">
          <small>当前版本：{appVersion || "读取中"}</small>
          <small>最新版本：{updateResult?.latestVersion ?? "未检查"}</small>
          <small>状态：{updateMessage}</small>
          <pre>{updateResult?.releaseNotes ?? "暂无更新日志。"}</pre>
        </div>
      </section>
    {:else}
      <section class="settings-section">
        <h3>关于</h3>
        <div class="setting-card about-card">
          <strong>Hexo Lite Editor</strong>
          <small>Version: {appVersion || "1.0.1"}</small>
          <small>Tauri + Svelte + TypeScript</small>
          <small>轻量级 Hexo 桌面博客编辑器</small>
          <small>License: MIT</small>
          <small>Runtime: Windows / WebView2 / Tauri 2</small>
        </div>
      </section>
    {/if}
  </div>
</aside>
