export type AppPage = "editor" | "imageBed" | "settings" | "about";
export type ImageBedProvider = "local" | "cloudflare-imgbed" | `plugin:${string}`;
export type ThemeMode = "light" | "dark" | "system";
export type AppLanguage = "system" | "zh-CN" | "en-US";
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
  assetFolder: string;
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
    language: AppLanguage;
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
    cloudflareConnectionId: string;
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
  uploadId?: string;
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
  path: string;
  extension?: string;
  url?: string;
  previewUrl?: string;
  mime?: string;
  size?: number;
  createdAt?: string;
  canPreview: boolean;
  capabilities: RemoteAssetCapabilities;
}

export interface RemoteAssetCapabilities {
  open: boolean;
  preview: boolean;
  copyUrl: boolean;
  copyMarkdown: boolean;
  download: boolean;
  rename: boolean;
  move: boolean;
  delete: boolean;
  createChildFolder: boolean;
  extract: boolean;
}

export interface CreateRemoteFolderRequest {
  projectId: string;
  sessionGeneration: number;
  parentDirectory: string;
  name: string;
}

export interface RenameRemoteAssetRequest {
  projectId: string;
  sessionGeneration: number;
  assetId: string;
  newName: string;
}

export interface MoveRemoteAssetRequest {
  projectId: string;
  sessionGeneration: number;
  assetId: string;
  targetDirectory: string;
}

export interface RenameArticleRequest {
  projectId: string;
  sessionGeneration: number;
  articleId: string;
  newTitle: string;
  newFileName?: string;
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
  requiresScopeConfirmation?: boolean;
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

export type UpdateStatus = "idle" | "checking" | "upToDate" | "available" | "downloading" | "downloaded" | "installing" | "error";
export type UpdateErrorStage = "check" | "download" | "install";
export interface UpdateSnapshot {
  currentVersion: string; status: UpdateStatus; latestVersion?: string; releaseNotes?: string;
  releaseDate?: string; downloadedBytes?: number; totalBytes?: number; errorStage?: UpdateErrorStage;
  errorMessage?: string; releasePageUrl?: string; assetDownloadUrl?: string;
}

export const defaultConfig: AppConfigV3 = {
  schemaVersion: 3,
  general: {
    language: "system",
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
    cloudflareConnectionId: "primary",
    uploadFolder: "/",
    autoInsertMarkdown: true
  },
  publish: {
    saveBeforeRun: true,
    cleanBeforeGenerate: true,
    generateBeforeDeploy: true,
    gitPushAfterDeploy: false
  },
  diagnostics: { logRetentionDays: 14, maxLogStorageMb: 20 },
  update: { checkOnStart: true }
};
