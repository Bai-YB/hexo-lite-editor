export type AppPage = "editor" | "imageBed" | "settings" | "about";
export type ImageBedProvider = "local" | "cloudflare-imgbed";
export type ThemeMode = "light" | "dark" | "system";
export type PreviewServerState = "starting" | "running" | "stopping" | "stopped" | "error";
export type SettingsSectionId = "general" | "editing" | "images" | "hexoPublish" | "sync" | "maintenance";
export type ArticleKind = "post" | "draft";
export type TaskType =
  | "clean"
  | "generate"
  | "deploy"
  | "publish"
  | "gitStatus"
  | "serverStart"
  | "serverStop";

export interface AppError {
  code: string;
  message: string;
  recoverable: boolean;
  details?: unknown;
}

export interface ProjectSessionView {
  projectId: string;
  generation: number;
  name: string;
  displayPath: string;
  warnings: string[];
}

export interface ArticleSummary {
  articleId: string;
  relativePath: string;
  title: string;
  kind: ArticleKind;
  frontMatterDate?: string;
  createdAt?: string;
  modifiedAt: string;
  tags: string[];
  categories: string[];
  cover: ArticleCover;
  parseError?: string;
}

export interface ArticleCover {
  source:
    | "cover"
    | "topImg"
    | "banner"
    | "thumbnail"
    | "indexImg"
    | "placeholder";
  previewUrl?: string;
  originalSource?: string;
  alt: string;
}

export interface CreateArticleRequest {
  projectId: string;
  sessionGeneration: number;
  title: string;
  fileName: string;
  kind: ArticleKind;
  date: string;
  tags: string[];
  categories: string[];
}

export interface OpenProjectResult {
  session: ProjectSessionView;
  articles: ArticleSummary[];
  sync?: ContentSyncView;
}

export interface ProjectRescanResult {
  projectId: string;
  generation: number;
  articles: ArticleSummary[];
}

export interface DocumentSnapshot {
  projectId: string;
  articleId: string;
  content: string;
  revision: number;
  sessionGeneration: number;
}

export interface SaveDocumentRequest {
  projectId: string;
  articleId: string;
  content: string;
  revision: number;
  sessionGeneration: number;
}

export interface SaveDocumentResult {
  articleId: string;
  acceptedRevision: number;
  savedAt: string;
}

export interface FrontMatterResult {
  attributes: Record<string, unknown>;
  body: string;
  error?: string;
}

export interface TaskEvent {
  taskId: string;
  projectId: string;
  sequence: number;
  kind: "queued" | "stepStarted" | "log" | "stepFinished" | "finished";
  step?: string;
  stream?: "stdout" | "stderr";
  line?: string;
  success?: boolean;
  exitCode?: number;
  timestamp: string;
}

export interface CredentialStatus {
  configured: boolean;
  username?: string;
}

export interface AcquireCloudflareImgbedTokenRequest {
  baseUrl: string;
  adminUsername?: string;
  adminPassword?: string;
  tokenName?: string;
  owner?: string;
  permissions?: Array<"upload" | "list" | "delete">;
  expiresAt?: string | null;
  autoDelete?: boolean;
}

export interface AcquireCloudflareImgbedTokenResult {
  configured: boolean;
  tokenId: string;
  tokenName: string;
  owner: string;
  permissions: Array<"upload" | "list" | "delete">;
  createdAt: string;
  expiresAt?: string | null;
}

export interface ImgBedConnectionTestResult {
  ok: boolean;
  baseUrl: string;
  listEndpoint: string;
  message: string;
}

export interface CloseWindowState {
  hasUnsavedChanges: boolean;
  isClosing: boolean;
}

export interface AppConfigV3 {
  schemaVersion: 3;
  general: {
    openRecentProjectOnStart: boolean;
    autoSave: boolean;
    autoSaveDelayMs: number;
    backupBeforeSave: boolean;
  };
  appearance: {
    themeMode: ThemeMode;
  };
  editor: {
    fontSize: number;
    lineHeight: number;
    showLineNumbers: boolean;
    lineWrapping: boolean;
    highlightActiveLine: boolean;
    tabSize: number;
  };
  articleList: {
    showCover: boolean;
  };
  layout: {
    articleListWidth: number;
    previewWidth: number;
    previewRatio: number;
    previewVisible: boolean;
  };
  hexo: {
    previewPort: number;
    autoStartPreview: boolean;
    previewDrafts: boolean;
  };
  imageBed: {
    defaultProvider: ImageBedProvider;
    localImageDir: string;
    localMarkdownPrefix: string;
    cloudflareName: string;
    cloudflareApiUrl: string;
    cloudflareTokenId?: string;
    uploadFolder: string;
    autoInsertMarkdown: boolean;
  };
  publish: {
    saveBeforeRun: boolean;
    cleanBeforeGenerate: boolean;
    generateBeforeDeploy: boolean;
    gitPushAfterDeploy: boolean;
  };
  diagnostics: {
    logRetentionDays: 7 | 14 | 30;
    maxLogStorageMb: 10 | 20 | 50;
  };
  update: {
    checkOnStart: boolean;
  };
}

export interface ConfigLoadResult {
  config: AppConfigV3;
  warnings: string[];
}

export interface LocalImage {
  imageId: string;
  name: string;
  relativePath: string;
  markdownUrl: string;
  mime: string;
  size: number;
  previewUrl: string;
}

export interface ResolveArticlePreviewImagesRequest {
  projectId: string;
  sessionGeneration: number;
  articleId: string;
  sources: string[];
}

export type PreviewImageState = "ready" | "unavailable";

export type PreviewImageFailureKind =
  | "invalidSource"
  | "unsafeSource"
  | "notFound"
  | "empty"
  | "notImage"
  | "unsupported"
  | "tooLarge"
  | "network";

export interface PreviewImageResult {
  originalSource: string;
  state: PreviewImageState;
  previewUrl?: string;
  httpStatus?: number;
  failureKind?: PreviewImageFailureKind;
  message?: string;
}

export interface UploadResult {
  url: string;
  markdown: string;
  fileName: string;
}

export interface EditorImageInput {
  name: string;
  mime: string;
  bytes: number[];
}

export interface ImageImportResult {
  fileName: string;
  url?: string;
  markdown?: string;
  error?: AppError;
}

export interface RecentProjectView {
  recentId: string;
  name: string;
  displayPath: string;
  lastOpenedAt: string;
  available: boolean;
}

export type RemoteAssetKind = "folder" | "image" | "archive" | "document" | "audio" | "video" | "file";

export interface RemoteAssetItem {
  assetId: string;
  kind: RemoteAssetKind;
  name: string;
  fileName: string;
  directory: string;
  extension?: string;
  url?: string;
  previewUrl?: string;
  mime?: string;
  size?: number;
  createdAt?: string;
  canPreview: boolean;
}

export interface RemoteAssetBreadcrumb {
  name: string;
  directory: string;
}

export interface RemoteAssetPage {
  currentDirectory: string;
  breadcrumbs: RemoteAssetBreadcrumb[];
  items: RemoteAssetItem[];
  totalCount: number;
  returnedCount: number;
  nextOffset?: number;
}

export interface PreviewServerView {
  projectId: string;
  sessionGeneration: number;
  state: PreviewServerState;
  port: number;
  baseUrl?: string;
  draftsEnabled: boolean;
  startedAt?: string;
  error?: AppError;
}

export type ContentSyncStatus =
  | "off"
  | "checking"
  | "synced"
  | "localPending"
  | "remoteAhead"
  | "conflict"
  | "offline"
  | "authRequired"
  | "error";

export type ContentSyncProvider = "github" | "webdav";

export interface ContentSyncView {
  enabled: boolean;
  status: ContentSyncStatus;
  provider: ContentSyncProvider;
  repository?: string;
  branch?: string;
  endpoint?: string;
  remoteDir?: string;
  visibility?: string;
  message?: string;
  conflicts: string[];
  lastSyncedAt?: string;
}

export interface ContentSyncEvent {
  phase: "checking" | "waiting" | "attention" | "failed" | "completed" | string;
  status: ContentSyncStatus;
  message?: string;
}

export interface ContentSyncCandidate {
  repository: string;
  source: string;
  pagesBranch?: string;
  visibility: string;
  defaultBranch?: string;
}

export interface ContentSyncDetection {
  candidates: ContentSyncCandidate[];
  requiresSelection: boolean;
}

export interface ContentSyncPreflight {
  candidate: ContentSyncCandidate;
  branch: string;
  fileCount: number;
  totalBytes: number;
  remoteFileCount: number;
  remoteTotalBytes: number;
  localOnlyCount: number;
  remoteOnlyCount: number;
  differentCount: number;
  remoteBranchExists: boolean;
  remoteManifestValid: boolean;
}

export interface WebDavContentSyncPreflight {
  endpoint: string;
  remoteDir: string;
  fileCount: number;
  totalBytes: number;
  remoteFileCount: number;
  remoteTotalBytes: number;
  localOnlyCount: number;
  remoteOnlyCount: number;
  differentCount: number;
  remoteExists: boolean;
  remoteManifestValid: boolean;
}

export interface WebDavConnectionTestResult {
  preflight: WebDavContentSyncPreflight;
  username: string;
  testedAt: string;
  sync: ContentSyncView;
}

export interface ContentSyncConflict {
  path: string;
  kind: "markdown" | "binary";
  localHash?: string;
  remoteHash?: string;
  localSize?: number;
  remoteSize?: number;
  localText?: string;
  remoteText?: string;
}

export interface TaskLogSummary {
  taskId: string;
  projectName: string;
  taskType: TaskType;
  startedAt: string;
  finishedAt?: string;
  success?: boolean;
  size: number;
  truncated: boolean;
}

export interface TaskLogPage {
  events: TaskEvent[];
  nextCursor?: number;
}

export interface RuntimeInfo {
  version: string;
  operatingSystem: string;
  architecture: string;
  webview: string;
}

export interface UpdateCheckResult {
  currentVersion: string;
  latestVersion: string;
  hasUpdate: boolean;
  releaseNotes?: string;
  releasePageUrl: string;
}

export const defaultConfig: AppConfigV3 = {
  schemaVersion: 3,
  general: {
    openRecentProjectOnStart: true,
    autoSave: true,
    autoSaveDelayMs: 2000,
    backupBeforeSave: false
  },
  appearance: { themeMode: "system" },
  editor: {
    fontSize: 15,
    lineHeight: 1.65,
    showLineNumbers: true,
    lineWrapping: true,
    highlightActiveLine: true,
    tabSize: 2
  },
  articleList: { showCover: true },
  layout: { articleListWidth: 280, previewWidth: 380, previewRatio: 0.5, previewVisible: true },
  hexo: {
    previewPort: 4000,
    autoStartPreview: false,
    previewDrafts: true
  },
  imageBed: {
    defaultProvider: "local",
    localImageDir: "source/images",
    localMarkdownPrefix: "/images",
    cloudflareName: "",
    cloudflareApiUrl: "",
    uploadFolder: "blog",
    autoInsertMarkdown: true
  },
  publish: {
    saveBeforeRun: true,
    cleanBeforeGenerate: false,
    generateBeforeDeploy: true,
    gitPushAfterDeploy: false
  },
  diagnostics: { logRetentionDays: 14, maxLogStorageMb: 20 },
  update: { checkOnStart: true }
};
