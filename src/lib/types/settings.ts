export type AppPage = "editor" | "settings" | "welcome";
export type ThemeMode = "light" | "dark" | "system";
export type ColorScheme = "classic" | "vscode-light" | "vscode-dark" | "monokai" | "one-dark" | "solarized-light";
export type EditorMode = "split" | "editor" | "preview";
export type UploaderType = "local" | "custom" | "smms" | "cloudflare-imgbed";
export type UpdateSource = "github" | "custom";

export interface GeneralSettings {
  openRecentProjectOnStart: boolean;
  autoSave: boolean;
  autoSaveInterval: number;
  backupBeforeSave: boolean;
  defaultPage: AppPage;
  maxLogCount: number;
}

export interface AppearanceSettings {
  themeMode: ThemeMode;
  colorScheme: ColorScheme;
  compactMode: boolean;
  showPostCover: boolean;
  fontScale: number;
}

export interface EditorSettings {
  fontSize: number;
  lineHeight: number;
  showLineNumbers: boolean;
  lineWrapping: boolean;
  highlightActiveLine: boolean;
  markdownHighlight: boolean;
  tabSize: number;
  defaultEditorMode: EditorMode;
}

export interface LayoutSettings {
  sidebarWidth: number;
  previewWidth: number;
  logPanelHeight: number;
  showPreview: boolean;
  showLogPanel: boolean;
}

export interface RibbonSettings {
  activeTab: "write" | "preview" | "publish" | "settings";
}

export interface PostListSettings {
  showCover: boolean;
  coverSourcePriority: string[];
}

export interface UploaderSettings {
  defaultType: UploaderType;
  apiUrl: string;
  token: string;
  method: "POST" | "PUT";
  fileField: string;
  urlField: string;
  autoInsertMarkdown: boolean;
}

export interface PublishSettings {
  hexoServerCommand: string;
  hexoCleanCommand: string;
  hexoGenerateCommand: string;
  hexoDeployCommand: string;
  saveBeforePublish: boolean;
  cleanBeforeGenerate: boolean;
  generateBeforeDeploy: boolean;
  gitPushAfterDeploy: boolean;
}

export interface UpdateSettings {
  checkUpdateOnStart: boolean;
  updateSource: UpdateSource;
  githubOwner: string;
  githubRepo: string;
  customUpdateUrl: string;
}

export interface AppSettings {
  general: GeneralSettings;
  appearance: AppearanceSettings;
  editor: EditorSettings;
  layout: LayoutSettings;
  ribbon: RibbonSettings;
  postList: PostListSettings;
  uploader: UploaderSettings;
  publish: PublishSettings;
  update: UpdateSettings;
  recentProjects: string[];
}

export interface UpdateCheckResult {
  currentVersion: string;
  latestVersion: string;
  hasUpdate: boolean;
  releaseNotes?: string;
  downloadUrl?: string;
  releasePageUrl?: string;
}

export const defaultSettings: AppSettings = {
  general: {
    openRecentProjectOnStart: true,
    autoSave: true,
    autoSaveInterval: 3000,
    backupBeforeSave: false,
    defaultPage: "editor",
    maxLogCount: 500
  },
  appearance: {
    themeMode: "system",
    colorScheme: "vscode-light",
    compactMode: false,
    showPostCover: true,
    fontScale: 1
  },
  editor: {
    fontSize: 16,
    lineHeight: 1.6,
    showLineNumbers: true,
    lineWrapping: true,
    highlightActiveLine: true,
    markdownHighlight: true,
    tabSize: 2,
    defaultEditorMode: "split"
  },
  layout: {
    sidebarWidth: 300,
    previewWidth: 0,
    logPanelHeight: 240,
    showPreview: true,
    showLogPanel: false
  },
  ribbon: {
    activeTab: "write"
  },
  postList: {
    showCover: true,
    coverSourcePriority: ["cover"]
  },
  uploader: {
    defaultType: "local",
    apiUrl: "",
    token: "",
    method: "POST",
    fileField: "file",
    urlField: "data.url",
    autoInsertMarkdown: true
  },
  publish: {
    hexoServerCommand: "npx hexo server",
    hexoCleanCommand: "npx hexo clean",
    hexoGenerateCommand: "npx hexo generate",
    hexoDeployCommand: "npx hexo deploy",
    saveBeforePublish: true,
    cleanBeforeGenerate: false,
    generateBeforeDeploy: true,
    gitPushAfterDeploy: false
  },
  update: {
    checkUpdateOnStart: false,
    updateSource: "github",
    githubOwner: "",
    githubRepo: "",
    customUpdateUrl: ""
  },
  recentProjects: []
};
