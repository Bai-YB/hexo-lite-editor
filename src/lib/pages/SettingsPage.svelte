<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    ArrowLeft,
    Brush,
    CheckCircle2,
    Code2,
    Download,
    FileCog,
    FolderCog,
    Info,
    RotateCcw,
    Rocket,
    Settings,
    UploadCloud
  } from "@lucide/svelte";
  import MarkdownEditor from "$lib/components/MarkdownEditor.svelte";
  import { checkUpdate as checkUpdateApi, openConfigDir, getAppVersion, openReleasePage } from "$lib/api/app";
  import {
    backupHexoConfig,
    openHexoConfigExternal,
    readHexoConfig,
    restoreLatestHexoConfigBackup,
    saveHexoConfig,
    type HexoConfigFile
  } from "$lib/api/hexoConfig";
  import { copyImageToProject } from "$lib/api/uploader";
  import { resetSettings } from "$lib/api/config";
  import type { HexoProject } from "$lib/types/project";
  import type { AppSettings, UpdateCheckResult } from "$lib/types/settings";

  type SettingTab = "general" | "appearance" | "editor" | "hexo" | "uploader" | "publish" | "update" | "about";

  export let settings: AppSettings;
  export let project: HexoProject | null = null;
  export let onBack: () => void = () => {};
  export let onChange: (settings: AppSettings) => void | Promise<void> = () => {};

  let activeTab: SettingTab = "general";
  let hexoConfig: HexoConfigFile | null = null;
  let hexoConfigContent = "";
  let hexoMessage = "";
  let uploadTestResult = "";
  let updateResult: UpdateCheckResult | null = null;
  let updateMessage = "";
  let currentVersion = "";

  const tabs: { id: SettingTab; label: string; icon: typeof Settings }[] = [
    { id: "general", label: "常规设置", icon: Settings },
    { id: "appearance", label: "外观设置", icon: Brush },
    { id: "editor", label: "编辑器设置", icon: Code2 },
    { id: "hexo", label: "Hexo 配置", icon: FileCog },
    { id: "uploader", label: "图床配置", icon: UploadCloud },
    { id: "publish", label: "发布配置", icon: Rocket },
    { id: "update", label: "更新检查", icon: Download },
    { id: "about", label: "关于软件", icon: Info }
  ];

  onMount(async () => {
    currentVersion = await getAppVersion();
    if (project) {
      await loadHexoConfig();
    }
  });

  $: if (activeTab === "hexo" && project && !hexoConfig) {
    loadHexoConfig();
  }

  async function commit(next: AppSettings) {
    settings = next;
    await onChange(settings);
  }

  function patch<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    commit({ ...settings, [key]: value });
  }

  function patchNested<K extends keyof AppSettings, F extends keyof AppSettings[K]>(
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

  async function resetAll() {
    if (!confirm("确定要重置所有设置吗？")) return;
    const next = await resetSettings();
    await commit(next);
  }

  async function loadHexoConfig() {
    if (!project) return;
    try {
      hexoConfig = await readHexoConfig(project.rootPath);
      hexoConfigContent = hexoConfig.content;
      hexoMessage = hexoConfig.exists ? "已读取 _config.yml" : "当前项目没有 _config.yml";
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
      hexoMessage = "已恢复最近备份";
    } catch (error) {
      hexoMessage = `恢复失败: ${error}`;
    }
  }

  async function testUpload() {
    if (!project) {
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
      const result = await copyImageToProject(project.rootPath, selected);
      uploadTestResult = `${result.url}\n${result.markdown}`;
    } catch (error) {
      uploadTestResult = `测试上传失败: ${error}`;
    }
  }

  async function copyUploadMarkdown() {
    const markdown = uploadTestResult.split("\n").find((line) => line.startsWith("!["));
    if (markdown) await navigator.clipboard.writeText(markdown);
  }

  async function checkUpdate() {
    updateMessage = "正在检查更新...";
    updateResult = null;
    try {
      updateResult = await checkUpdateApi(settings.update);
      updateMessage = updateResult.hasUpdate ? "发现新版本" : "当前已是最新版";
    } catch (error) {
      updateMessage = `检查失败: ${error}`;
    }
  }
</script>

<section class="settings-page">
  <header class="settings-top">
    <div>
      <h1>设置中心</h1>
      <span>集中管理应用配置、主题、Hexo 配置、发布和更新。</span>
    </div>
    <button on:click={onBack}><ArrowLeft size={17} />返回编辑器</button>
  </header>

  <div class="settings-layout">
    <nav class="settings-nav">
      {#each tabs as tab}
        <button class:active={activeTab === tab.id} on:click={() => (activeTab = tab.id)}>
          <svelte:component this={tab.icon} size={17} />
          {tab.label}
        </button>
      {/each}
    </nav>

    <main class="settings-content">
      {#if activeTab === "general"}
        <section class="settings-section">
          <h2>常规设置</h2>
          <div class="settings-grid">
            <label class="switch-row">
              <input
                type="checkbox"
                checked={settings.general.openRecentProjectOnStart}
                on:change={(event) => patchNested("general", "openRecentProjectOnStart", event.currentTarget.checked)}
              />
              <span>启动时打开最近项目</span>
            </label>
            <label class="switch-row">
              <input
                type="checkbox"
                checked={settings.general.autoSave}
                on:change={(event) => patchNested("general", "autoSave", event.currentTarget.checked)}
              />
              <span>自动保存</span>
            </label>
            <label class="switch-row">
              <input
                type="checkbox"
                checked={settings.general.backupBeforeSave}
                on:change={(event) => patchNested("general", "backupBeforeSave", event.currentTarget.checked)}
              />
              <span>保存前自动备份文章</span>
            </label>
            <label>
              <span>自动保存间隔 ms</span>
              <input
                type="number"
                min="1000"
                step="500"
                value={settings.general.autoSaveInterval}
                on:input={(event) => patchNested("general", "autoSaveInterval", Number(event.currentTarget.value))}
              />
            </label>
            <label>
              <span>默认打开页面</span>
              <select
                value={settings.general.defaultPage}
                on:change={(event) => patchNested("general", "defaultPage", event.currentTarget.value as AppSettings["general"]["defaultPage"])}
              >
                <option value="editor">编辑器</option>
                <option value="settings">设置中心</option>
                <option value="welcome">欢迎页</option>
              </select>
            </label>
            <label>
              <span>日志保留数量</span>
              <input
                type="number"
                min="50"
                max="5000"
                value={settings.general.maxLogCount}
                on:input={(event) => patchNested("general", "maxLogCount", Number(event.currentTarget.value))}
              />
            </label>
          </div>
          <div class="button-row">
            <button on:click={openConfigDir}><FolderCog size={16} />打开配置目录</button>
            <button class="danger" on:click={resetAll}><RotateCcw size={16} />重置所有设置</button>
          </div>
        </section>
      {:else if activeTab === "appearance"}
        <section class="settings-section">
          <h2>外观设置</h2>
          <div class="settings-grid">
            <label>
              <span>主题模式</span>
              <select
                value={settings.appearance.themeMode}
                on:change={(event) => patchNested("appearance", "themeMode", event.currentTarget.value as AppSettings["appearance"]["themeMode"])}
              >
                <option value="system">跟随系统</option>
                <option value="light">浅色模式</option>
                <option value="dark">深色模式</option>
              </select>
            </label>
            <label class="switch-row">
              <input
                type="checkbox"
                checked={settings.appearance.compactMode}
                on:change={(event) => patchNested("appearance", "compactMode", event.currentTarget.checked)}
              />
              <span>紧凑模式</span>
            </label>
            <label class="switch-row">
              <input
                type="checkbox"
                checked={settings.appearance.showPostCover}
                on:change={(event) => patchNested("appearance", "showPostCover", event.currentTarget.checked)}
              />
              <span>文章列表显示封面</span>
            </label>
            <label>
              <span>界面字体缩放</span>
              <input
                type="number"
                min="0.85"
                max="1.25"
                step="0.05"
                value={settings.appearance.fontScale}
                on:input={(event) => patchNested("appearance", "fontScale", Number(event.currentTarget.value))}
              />
            </label>
          </div>
        </section>
      {:else if activeTab === "editor"}
        <section class="settings-section">
          <h2>编辑器设置</h2>
          <div class="settings-grid">
            <label>
              <span>字体大小</span>
              <input type="number" min="12" max="24" value={settings.editor.fontSize} on:input={(event) => patchNested("editor", "fontSize", Number(event.currentTarget.value))} />
            </label>
            <label>
              <span>行高</span>
              <input type="number" min="1.2" max="2.2" step="0.1" value={settings.editor.lineHeight} on:input={(event) => patchNested("editor", "lineHeight", Number(event.currentTarget.value))} />
            </label>
            <label>
              <span>Tab 宽度</span>
              <input type="number" min="2" max="8" value={settings.editor.tabSize} on:input={(event) => patchNested("editor", "tabSize", Number(event.currentTarget.value))} />
            </label>
            <label>
              <span>默认编辑模式</span>
              <select value={settings.editor.defaultEditorMode} on:change={(event) => patchNested("editor", "defaultEditorMode", event.currentTarget.value as AppSettings["editor"]["defaultEditorMode"])}>
                <option value="split">双栏</option>
                <option value="editor">编辑</option>
                <option value="preview">预览</option>
              </select>
            </label>
            <label class="switch-row"><input type="checkbox" checked={settings.editor.showLineNumbers} on:change={(event) => patchNested("editor", "showLineNumbers", event.currentTarget.checked)} /><span>显示行号</span></label>
            <label class="switch-row"><input type="checkbox" checked={settings.editor.lineWrapping} on:change={(event) => patchNested("editor", "lineWrapping", event.currentTarget.checked)} /><span>自动换行</span></label>
            <label class="switch-row"><input type="checkbox" checked={settings.editor.highlightActiveLine} on:change={(event) => patchNested("editor", "highlightActiveLine", event.currentTarget.checked)} /><span>高亮当前行</span></label>
            <label class="switch-row"><input type="checkbox" checked={settings.editor.markdownHighlight} on:change={(event) => patchNested("editor", "markdownHighlight", event.currentTarget.checked)} /><span>Markdown 语法高亮</span></label>
          </div>
        </section>
      {:else if activeTab === "hexo"}
        <section class="settings-section hexo-config-section">
          <h2>Hexo 配置</h2>
          <div class="config-meta">
            <span>当前项目：{project?.rootPath ?? "未打开项目"}</span>
            <span>配置文件：{hexoConfig?.config_path ?? "未读取"}</span>
            <span>{hexoMessage}</span>
          </div>
          <div class="button-row">
            <button disabled={!project} on:click={loadHexoConfig}>打开配置文件</button>
            <button disabled={!project || !hexoConfig?.exists} on:click={saveCurrentHexoConfig}>保存配置</button>
            <button disabled={!project || !hexoConfig?.exists} on:click={backupCurrentHexoConfig}>备份</button>
            <button disabled={!project || !hexoConfig?.latest_backup_path} on:click={restoreHexoConfig}>恢复备份</button>
            <button disabled={!project || !hexoConfig?.exists} on:click={() => project && openHexoConfigExternal(project.rootPath)}>外部打开</button>
          </div>
          <div class="yaml-editor">
            {#if project}
              <MarkdownEditor
                content={hexoConfigContent}
                syntax="yaml"
                fontSize={settings.editor.fontSize}
                lineHeight={settings.editor.lineHeight}
                showLineNumbers={settings.editor.showLineNumbers}
                lineWrapping={settings.editor.lineWrapping}
                highlightLine={settings.editor.highlightActiveLine}
                tabSize={settings.editor.tabSize}
                onChange={(value) => (hexoConfigContent = value)}
                onSave={saveCurrentHexoConfig}
              />
            {:else}
              <div class="settings-empty">请先返回编辑器打开 Hexo 项目。</div>
            {/if}
          </div>
        </section>
      {:else if activeTab === "uploader"}
        <section class="settings-section">
          <h2>图床配置</h2>
          <div class="settings-grid">
            <label>
              <span>默认图床类型</span>
              <select value={settings.uploader.defaultType} on:change={(event) => patchNested("uploader", "defaultType", event.currentTarget.value as AppSettings["uploader"]["defaultType"])}>
                <option value="local">本地图床</option>
                <option value="custom">自定义 API</option>
                <option value="smms">SM.MS</option>
              </select>
            </label>
            <label><span>API 地址</span><input value={settings.uploader.apiUrl} on:input={(event) => patchNested("uploader", "apiUrl", event.currentTarget.value)} /></label>
            <label><span>Token</span><input value={settings.uploader.token} on:input={(event) => patchNested("uploader", "token", event.currentTarget.value)} /></label>
            <label><span>请求方式</span><select value={settings.uploader.method} on:change={(event) => patchNested("uploader", "method", event.currentTarget.value as "POST" | "PUT")}><option value="POST">POST</option><option value="PUT">PUT</option></select></label>
            <label><span>文件字段名</span><input value={settings.uploader.fileField} on:input={(event) => patchNested("uploader", "fileField", event.currentTarget.value)} /></label>
            <label><span>返回 URL 字段</span><input value={settings.uploader.urlField} on:input={(event) => patchNested("uploader", "urlField", event.currentTarget.value)} /></label>
            <label class="switch-row"><input type="checkbox" checked={settings.uploader.autoInsertMarkdown} on:change={(event) => patchNested("uploader", "autoInsertMarkdown", event.currentTarget.checked)} /><span>上传后自动插入 Markdown</span></label>
          </div>
          <div class="button-row">
            <button on:click={testUpload}>测试上传</button>
            <button disabled={!uploadTestResult.includes("![")} on:click={copyUploadMarkdown}>复制 Markdown</button>
          </div>
          <pre class="result-box">{uploadTestResult || "本地图床测试会复制图片到 source/images。"}</pre>
        </section>
      {:else if activeTab === "publish"}
        <section class="settings-section">
          <h2>发布配置</h2>
          <div class="settings-grid">
            <label><span>Hexo Server 命令</span><input value={settings.publish.hexoServerCommand} on:input={(event) => patchNested("publish", "hexoServerCommand", event.currentTarget.value)} /></label>
            <label><span>Hexo Clean 命令</span><input value={settings.publish.hexoCleanCommand} on:input={(event) => patchNested("publish", "hexoCleanCommand", event.currentTarget.value)} /></label>
            <label><span>Hexo Generate 命令</span><input value={settings.publish.hexoGenerateCommand} on:input={(event) => patchNested("publish", "hexoGenerateCommand", event.currentTarget.value)} /></label>
            <label><span>Hexo Deploy 命令</span><input value={settings.publish.hexoDeployCommand} on:input={(event) => patchNested("publish", "hexoDeployCommand", event.currentTarget.value)} /></label>
            <label class="switch-row"><input type="checkbox" checked={settings.publish.saveBeforePublish} on:change={(event) => patchNested("publish", "saveBeforePublish", event.currentTarget.checked)} /><span>发布前自动保存文章</span></label>
            <label class="switch-row"><input type="checkbox" checked={settings.publish.cleanBeforeGenerate} on:change={(event) => patchNested("publish", "cleanBeforeGenerate", event.currentTarget.checked)} /><span>生成前自动 clean</span></label>
            <label class="switch-row"><input type="checkbox" checked={settings.publish.generateBeforeDeploy} on:change={(event) => patchNested("publish", "generateBeforeDeploy", event.currentTarget.checked)} /><span>发布前自动 generate</span></label>
            <label class="switch-row"><input type="checkbox" checked={settings.publish.gitPushAfterDeploy} on:change={(event) => patchNested("publish", "gitPushAfterDeploy", event.currentTarget.checked)} /><span>发布后自动 git push</span></label>
          </div>
        </section>
      {:else if activeTab === "update"}
        <section class="settings-section">
          <h2>更新检查</h2>
          <div class="settings-grid">
            <label class="switch-row"><input type="checkbox" checked={settings.update.checkUpdateOnStart} on:change={(event) => patchNested("update", "checkUpdateOnStart", event.currentTarget.checked)} /><span>启动时检查更新</span></label>
            <label><span>更新来源</span><select value={settings.update.updateSource} on:change={(event) => patchNested("update", "updateSource", event.currentTarget.value as AppSettings["update"]["updateSource"])}><option value="github">GitHub Releases</option><option value="custom">自定义 JSON</option></select></label>
            <label><span>GitHub Owner</span><input value={settings.update.githubOwner} on:input={(event) => patchNested("update", "githubOwner", event.currentTarget.value)} /></label>
            <label><span>GitHub Repo</span><input value={settings.update.githubRepo} on:input={(event) => patchNested("update", "githubRepo", event.currentTarget.value)} /></label>
            <label><span>自定义更新地址</span><input value={settings.update.customUpdateUrl} on:input={(event) => patchNested("update", "customUpdateUrl", event.currentTarget.value)} /></label>
          </div>
          <div class="button-row">
            <button on:click={checkUpdate}><CheckCircle2 size={16} />检查更新</button>
            <button disabled={!updateResult?.releasePageUrl && !updateResult?.downloadUrl} on:click={() => updateResult && openReleasePage(updateResult.releasePageUrl || updateResult.downloadUrl || "")}>打开下载页面</button>
          </div>
          <div class="update-card">
            <span>当前版本：{currentVersion || "读取中"}</span>
            <span>最新版本：{updateResult?.latestVersion ?? "未检查"}</span>
            <span>状态：{updateMessage || "等待检查"}</span>
            <pre>{updateResult?.releaseNotes ?? "暂无更新日志。"}</pre>
          </div>
        </section>
      {:else}
        <section class="settings-section">
          <h2>关于软件</h2>
          <div class="about-card">
            <strong>Hexo Lite Editor</strong>
            <span>Version: {currentVersion || "0.2.0"}</span>
            <span>Tauri + Svelte + TypeScript</span>
            <span>轻量级 Hexo 桌面博客编辑器</span>
            <span>作者：User</span>
            <span>许可证：MIT</span>
            <span>运行环境：Windows / WebView2 / Tauri 2</span>
          </div>
        </section>
      {/if}
    </main>
  </div>
</section>
