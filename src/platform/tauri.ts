import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import type {
  AcquireCloudflareImgbedTokenRequest,
  AcquireCloudflareImgbedTokenResult,
  AppConfigV3,
  AppError,
  ArticleSummary,
  CreateArticleRequest,
  ConfigLoadResult,
  CredentialStatus,
  DocumentSnapshot,
  FrontMatterResult,
  EditorImageInput,
  ImageImportResult,
  ImgBedConnectionTestResult,
  LocalImage,
  OpenProjectResult,
  ProjectSessionView,
  ProjectRescanResult,
  RuntimeInfo,
  RecentProjectView,
  RemoteAssetPage,
  PreviewImageResult,
  ResolveArticlePreviewImagesRequest,
  ContentSyncCandidate,
  ContentSyncConflict,
  ContentSyncDetection,
  ContentSyncEvent,
  ContentSyncPreflight,
  WebDavConnectionTestResult,
  ContentSyncView,
  PreviewServerView,
  SaveDocumentRequest,
  SaveDocumentResult,
  TaskEvent,
  TaskLogPage,
  TaskLogSummary,
  TaskType,
  UpdateCheckResult,
  UploadResult
} from "$shared/types/app";
import { defaultConfig } from "$shared/types/app";
import { browserMock } from "./browserMock";

export const isTauri = () =>
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const isBrowserDemo = () =>
  !isTauri() && import.meta.env.DEV && typeof location !== "undefined" && new URLSearchParams(location.search).get("demo") === "1";

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeError(error);
  }
}

export function normalizeError(error: unknown): AppError {
  if (typeof error === "object" && error !== null && "message" in error) {
    const candidate = error as Partial<AppError>;
    return {
      code: candidate.code ?? "unknown_error",
      message: String(candidate.message),
      recoverable: candidate.recoverable ?? true,
      details: candidate.details
    };
  }
  return {
    code: "unknown_error",
    message: typeof error === "string" ? error : "发生未知错误。",
    recoverable: true
  };
}

export const platform = {
  async loadConfig(): Promise<ConfigLoadResult> {
    if (isBrowserDemo()) return browserMock.loadConfig();
    if (!isTauri()) return { config: structuredClone(defaultConfig), warnings: [] };
    return call("load_app_config");
  },
  saveConfig(config: AppConfigV3) {
    if (isBrowserDemo()) return browserMock.saveConfig(config);
    if (!isTauri()) return Promise.resolve(config);
    return call<AppConfigV3>("save_app_config", { config });
  },
  resetConfig() {
    if (isBrowserDemo()) return browserMock.resetConfig();
    if (!isTauri()) return Promise.resolve(structuredClone(defaultConfig));
    return call<AppConfigV3>("reset_app_config");
  },
  pickProject() {
    if (isBrowserDemo()) return browserMock.pickProject();
    if (!isTauri()) return Promise.resolve<OpenProjectResult | null>(null);
    return call<OpenProjectResult | null>("pick_project");
  },
  reopenRecentProject() {
    if (isBrowserDemo()) return browserMock.reopenRecentProject();
    if (!isTauri()) return Promise.resolve<OpenProjectResult | null>(null);
    return call<OpenProjectResult | null>("reopen_recent_project");
  },
  listRecentProjects() {
    if (isBrowserDemo()) return browserMock.listRecentProjects();
    if (!isTauri()) return Promise.resolve<RecentProjectView[]>([]);
    return call<RecentProjectView[]>("list_recent_projects");
  },
  openRecentProject(recentId: string) {
    if (isBrowserDemo()) return browserMock.openRecentProject(recentId);
    return call<OpenProjectResult>("open_recent_project", { recentId });
  },
  removeRecentProject(recentId: string) {
    if (isBrowserDemo()) return browserMock.removeRecentProject(recentId);
    return call<void>("remove_recent_project", { recentId });
  },
  clearRecentProjects() {
    if (isBrowserDemo()) return browserMock.clearRecentProjects();
    return call<void>("clear_recent_projects");
  },
  currentProject() {
    return call<ProjectSessionView | null>("current_project");
  },
  closeProject() {
    return call<void>("close_project");
  },
  listArticles(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.listArticles();
    return call<ArticleSummary[]>("list_articles", { projectId, sessionGeneration });
  },
  loadDocument(projectId: string, articleId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.loadDocument(projectId, articleId);
    return call<DocumentSnapshot>("load_document", {
      projectId,
      articleId,
      sessionGeneration
    });
  },
  saveDocument(request: SaveDocumentRequest) {
    if (isBrowserDemo()) return browserMock.saveDocument(request);
    return call<SaveDocumentResult>("save_document", { request });
  },
  createArticle(request: CreateArticleRequest) {
    if (isBrowserDemo()) return browserMock.createArticle(request);
    return call<ArticleSummary>("create_article", { request });
  },
  parseFrontMatter(content: string) {
    return call<FrontMatterResult>("parse_document_front_matter", { content });
  },
  startTask(projectId: string, kind: TaskType) {
    if (isBrowserDemo()) return browserMock.startTask(projectId, kind);
    return call<{ taskId: string }>("start_task", { projectId, kind });
  },
  cancelTask(taskId: string) {
    if (isBrowserDemo()) return browserMock.cancelTask(taskId);
    return call<void>("cancel_task", { taskId });
  },
  async onTaskEvent(handler: (event: TaskEvent) => void): Promise<UnlistenFn> {
    if (isBrowserDemo()) return browserMock.onTaskEvent(handler);
    if (!isTauri()) return () => undefined;
    return listen<TaskEvent>("task-event", ({ payload }) => handler(payload));
  },
  listLocalImages(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.listLocalImages();
    return call<LocalImage[]>("list_local_images", { projectId, sessionGeneration });
  },
  importLocalImages(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.importLocalImages();
    return call<LocalImage[]>("import_local_images", { projectId, sessionGeneration });
  },
  deleteLocalImage(projectId: string, sessionGeneration: number, imageId: string) {
    if (isBrowserDemo()) return browserMock.deleteLocalImage();
    return call<void>("delete_local_image", { projectId, sessionGeneration, imageId });
  },
  uploadCloudflareImage(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.uploadCloudflareImage();
    return call<UploadResult | null>("upload_cloudflare_image", {
      projectId,
      sessionGeneration
    });
  },
  importEditorImages(
    projectId: string,
    sessionGeneration: number,
    provider: AppConfigV3["imageBed"]["defaultProvider"],
    files: EditorImageInput[]
  ) {
    if (isBrowserDemo()) return browserMock.importEditorImages(provider, files);
    return call<ImageImportResult[]>("import_editor_images", {
      projectId,
      sessionGeneration,
      provider,
      files
    });
  },
  importEditorImagePaths(
    projectId: string,
    sessionGeneration: number,
    provider: AppConfigV3["imageBed"]["defaultProvider"],
    paths: string[]
  ) {
    return call<ImageImportResult[]>("import_editor_image_paths", {
      projectId,
      sessionGeneration,
      provider,
      paths
    });
  },
  uploadCachedEditorImage(projectId: string, sessionGeneration: number, uploadId: string) {
    if (isBrowserDemo()) return browserMock.uploadCachedEditorImage(uploadId);
    return call<ImageImportResult>("upload_cached_editor_image", {
      projectId,
      sessionGeneration,
      uploadId
    });
  },
  finalizeCachedEditorImage(projectId: string, sessionGeneration: number, uploadId: string) {
    if (isBrowserDemo()) return browserMock.finalizeCachedEditorImage(uploadId);
    return call<void>("finalize_cached_editor_image", {
      projectId,
      sessionGeneration,
      uploadId
    });
  },
  listCloudflareAssets(
    projectId: string,
    sessionGeneration: number,
    offset: number,
    count: number,
    search: string,
    directory: string
  ) {
    if (isBrowserDemo()) return browserMock.listCloudflareAssets(offset, count, search, directory);
    return call<RemoteAssetPage>("list_cloudflare_assets", {
      projectId,
      sessionGeneration,
      offset,
      count,
      search,
      directory
    });
  },
  deleteCloudflareAsset(projectId: string, sessionGeneration: number, assetId: string) {
    if (isBrowserDemo()) return browserMock.deleteCloudflareAsset();
    return call<void>("delete_cloudflare_asset", { projectId, sessionGeneration, assetId });
  },
  revealLocalImage(projectId: string, sessionGeneration: number, imageId: string) {
    if (isBrowserDemo()) return browserMock.revealLocalImage();
    return call<void>("reveal_local_image", { projectId, sessionGeneration, imageId });
  },
  resolveArticlePreviewImages(request: ResolveArticlePreviewImagesRequest) {
    if (isBrowserDemo()) return browserMock.resolveArticlePreviewImages(request.sources);
    if (!isTauri()) {
      return Promise.resolve<PreviewImageResult[]>(
        request.sources.map((originalSource) => ({
          originalSource,
          state: "unavailable",
          message: "桌面后端不可用。"
        }))
      );
    }
    return call<PreviewImageResult[]>("resolve_article_preview_images", { request });
  },
  detectContentSync(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.detectContentSync();
    return call<ContentSyncDetection>("detect_content_sync", { projectId, sessionGeneration });
  },
  preflightContentSync(projectId: string, sessionGeneration: number, repository: string, branch: string) {
    if (isBrowserDemo()) return browserMock.preflightContentSync(repository, branch);
    return call<ContentSyncPreflight>("preflight_content_sync", { request: { projectId, sessionGeneration, repository, branch } });
  },
  testWebDavContentSync(request: { projectId: string; sessionGeneration: number; endpoint: string; remoteDir: string; username: string; password?: string }) {
    if (isBrowserDemo()) return browserMock.testWebDavContentSync(request);
    return call<WebDavConnectionTestResult>("test_webdav_content_sync", { request });
  },
  getContentSyncStatus(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.getContentSyncStatus();
    return call<ContentSyncView>("get_content_sync_status", { projectId, sessionGeneration });
  },
  enableContentSync(request: { projectId: string; sessionGeneration: number; repository: string; branch: string; initialChoice?: "local" | "remote"; confirmPublic: boolean }) {
    if (isBrowserDemo()) return browserMock.enableContentSync(request);
    return call<ContentSyncView>("enable_content_sync", { request });
  },
  enableWebDavContentSync(request: { projectId: string; sessionGeneration: number; endpoint: string; remoteDir: string; initialChoice?: "local" | "remote" }) {
    if (isBrowserDemo()) return browserMock.enableWebDavContentSync(request);
    return call<ContentSyncView>("enable_webdav_content_sync", { request });
  },
  updateWebDavContentSync(request: { projectId: string; sessionGeneration: number; endpoint: string; remoteDir: string }) {
    if (isBrowserDemo()) return browserMock.updateWebDavContentSync(request);
    return call<ContentSyncView>("update_webdav_content_sync", { request });
  },
  disableContentSync(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.disableContentSync();
    return call<ContentSyncView>("disable_content_sync", { projectId, sessionGeneration });
  },
  runContentSync(projectId: string, sessionGeneration: number, direction: "auto" | "local" | "remote" = "auto") {
    if (isBrowserDemo()) return browserMock.runContentSync();
    return call<ContentSyncView>("run_content_sync", { request: { projectId, sessionGeneration, direction } });
  },
  getContentSyncConflicts(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.getContentSyncConflicts();
    return call<ContentSyncConflict[]>("get_content_sync_conflicts", { projectId, sessionGeneration });
  },
  resolveContentSyncConflicts(projectId: string, sessionGeneration: number, choices: Record<string, "local" | "remote">) {
    if (isBrowserDemo()) return browserMock.resolveContentSyncConflicts();
    return call<ContentSyncView>("resolve_content_sync_conflicts", { request: { projectId, sessionGeneration, choices } });
  },
  openContentSyncBackups(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return Promise.resolve();
    return call<void>("open_content_sync_backups", { projectId, sessionGeneration });
  },
  reconnectContentSync(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.runContentSync();
    return call<ContentSyncView>("reconnect_content_sync", { projectId, sessionGeneration });
  },
  webDavCredentialStatus(endpoint: string) {
    if (isBrowserDemo()) return browserMock.webDavCredentialStatus(endpoint);
    return call<CredentialStatus>("webdav_credential_status", { endpoint });
  },
  webDavCredentialDelete(endpoint: string) {
    if (isBrowserDemo()) return browserMock.webDavCredentialDelete(endpoint);
    return call<CredentialStatus>("webdav_credential_delete", { endpoint });
  },
  async writeClipboard(text: string) {
    if (!isTauri()) {
      await navigator.clipboard.writeText(text);
      return;
    }
    await writeText(text);
  },
  credentialStatus(connectionId: string, baseUrl: string) {
    if (isBrowserDemo()) return browserMock.credentialStatus(connectionId, baseUrl);
    return call<CredentialStatus>("credential_status", { connectionId, baseUrl });
  },
  credentialSet(connectionId: string, baseUrl: string, token: string) {
    if (isBrowserDemo()) return browserMock.credentialSet(connectionId, baseUrl, token);
    return call<CredentialStatus>("credential_set", { connectionId, baseUrl, token });
  },
  credentialDelete(connectionId: string) {
    if (isBrowserDemo()) return browserMock.credentialDelete(connectionId);
    return call<CredentialStatus>("credential_delete", { connectionId });
  },
  credentialLegacyAvailable() {
    if (isBrowserDemo()) return browserMock.credentialLegacyAvailable();
    return call<boolean>("credential_legacy_available");
  },
  credentialMigrate(connectionId: string, baseUrl: string) {
    if (isBrowserDemo()) return browserMock.credentialMigrate(connectionId, baseUrl);
    return call<CredentialStatus>("credential_migrate", { connectionId, baseUrl });
  },
  acquireCloudflareImgbedToken(
    connectionId: string,
    request: AcquireCloudflareImgbedTokenRequest
  ) {
    if (isBrowserDemo()) return browserMock.acquireCloudflareImgbedToken(connectionId, request);
    return call<AcquireCloudflareImgbedTokenResult>("acquire_cloudflare_imgbed_token", {
      connectionId,
      request
    });
  },
  testCloudflareImgbedToken(connectionId: string, baseUrl: string) {
    if (isBrowserDemo()) return browserMock.testCloudflareImgbedToken(connectionId, baseUrl);
    return call<ImgBedConnectionTestResult>("test_cloudflare_imgbed_token", {
      connectionId,
      baseUrl
    });
  },
  cleanupBeforeExit() {
    if (isBrowserDemo() || !isTauri()) return Promise.resolve();
    return call<void>("cleanup_before_exit");
  },
  getPreviewStatus(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.getPreviewStatus(projectId, sessionGeneration);
    return call<PreviewServerView>("get_preview_status", { projectId, sessionGeneration });
  },
  startPreviewServer(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.startPreviewServer(projectId, sessionGeneration);
    return call<PreviewServerView>("start_preview_server", { projectId, sessionGeneration });
  },
  stopPreviewServer(projectId: string, sessionGeneration: number) {
    if (isBrowserDemo()) return browserMock.stopPreviewServer(projectId, sessionGeneration);
    return call<PreviewServerView>("stop_preview_server", { projectId, sessionGeneration });
  },
  resolveArticlePreviewUrl(projectId: string, sessionGeneration: number, articleId: string) {
    if (isBrowserDemo()) return browserMock.resolveArticlePreviewUrl(articleId);
    return call<string>("resolve_article_preview_url", { projectId, sessionGeneration, articleId });
  },
  async onPreviewStatus(handler: (view: PreviewServerView) => void): Promise<UnlistenFn> {
    if (isBrowserDemo()) return browserMock.onPreviewStatus(handler);
    if (!isTauri()) return () => undefined;
    return listen<PreviewServerView>("preview-status", ({ payload }) => handler(payload));
  },
  async onContentSyncStatus(handler: (view: ContentSyncView) => void): Promise<UnlistenFn> {
    if (!isTauri()) return () => undefined;
    return listen<ContentSyncView>("content-sync-status", ({ payload }) => handler(payload));
  },
  async onContentSyncPhase(handler: (event: ContentSyncEvent) => void): Promise<UnlistenFn> {
    if (!isTauri()) return () => undefined;
    return listen<ContentSyncEvent>("content-sync-phase", ({ payload }) => handler(payload));
  },
  async onProjectRescanned(handler: (project: ProjectRescanResult) => void): Promise<UnlistenFn> {
    if (!isTauri()) return () => undefined;
    return listen<ProjectRescanResult>("project-rescanned", ({ payload }) => handler(payload));
  },
  listTaskLogs() {
    if (isBrowserDemo()) return browserMock.listTaskLogs();
    return call<TaskLogSummary[]>("list_task_logs");
  },
  readTaskLog(taskId: string, cursor = 0, count = 300) {
    if (isBrowserDemo()) return browserMock.readTaskLog(taskId);
    return call<TaskLogPage>("read_task_log", { taskId, cursor, count });
  },
  deleteTaskLog(taskId: string) {
    if (isBrowserDemo()) return browserMock.deleteTaskLog(taskId);
    return call<void>("delete_task_log", { taskId });
  },
  clearTaskLogs() {
    if (isBrowserDemo()) return browserMock.clearTaskLogs();
    return call<void>("clear_task_logs");
  },
  runtimeInfo() {
    if (!isTauri()) {
      return Promise.resolve<RuntimeInfo>({
        version: "1.0.5",
        operatingSystem: navigator.platform,
        architecture: "browser preview",
        webview: navigator.userAgent
      });
    }
    return call<RuntimeInfo>("runtime_info");
  },
  openExternalTarget(
    target: "projectHomepage" | "license" | "releasePage" | "cloudflareDashboard" | "hexoPreview"
  ) {
    return call<void>("open_external_target", { target });
  },
  openMarkdownLink(url: string) {
    return call<void>("open_markdown_link", { url });
  },
  checkUpdate() {
    return call<UpdateCheckResult>("check_update");
  }
};
