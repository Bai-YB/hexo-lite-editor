use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
            details: None,
        }
    }

    pub fn io(context: &str, error: impl std::fmt::Display) -> Self {
        Self::new("io_error", format!("{context}: {error}"), true)
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new("invalid_request", message, true)
    }

    pub fn session_expired() -> Self {
        Self::new(
            "project_session_expired",
            "项目会话已经变化，请重新打开项目后再试。",
            true,
        )
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSessionView {
    pub project_id: String,
    pub generation: u64,
    pub name: String,
    pub display_path: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProjectResult {
    pub session: ProjectSessionView,
    pub articles: Vec<ArticleSummary>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateArticleRequest {
    pub project_id: String,
    pub session_generation: u64,
    pub title: String,
    pub file_name: String,
    pub kind: ArticleKind,
    pub date: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleSummary {
    pub article_id: String,
    pub relative_path: String,
    pub title: String,
    pub kind: ArticleKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_matter_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    pub modified_at: String,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
    pub cover: ArticleCover,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ArticleCover {
    pub source: ArticleCoverSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    pub alt: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ArticleCoverSource {
    Cover,
    TopImg,
    Banner,
    Thumbnail,
    IndexImg,
    FirstImage,
    Placeholder,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ArticleKind {
    Post,
    Draft,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSnapshot {
    pub project_id: String,
    pub article_id: String,
    pub content: String,
    pub revision: u64,
    pub session_generation: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDocumentRequest {
    pub project_id: String,
    pub article_id: String,
    pub content: String,
    pub revision: u64,
    pub session_generation: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveDocumentResult {
    pub article_id: String,
    pub accepted_revision: u64,
    pub saved_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontMatterResult {
    pub attributes: Value,
    pub body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfigV3 {
    pub schema_version: u8,
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub editor: EditorConfig,
    #[serde(default)]
    pub article_list: ArticleListConfig,
    pub layout: LayoutConfig,
    pub hexo: HexoConfig,
    pub image_bed: ImageBedConfig,
    pub publish: PublishConfig,
    #[serde(default)]
    pub diagnostics: DiagnosticsConfig,
    pub update: UpdateConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArticleListConfig {
    pub show_cover: bool,
}

impl Default for ArticleListConfig {
    fn default() -> Self {
        Self { show_cover: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralConfig {
    pub open_recent_project_on_start: bool,
    pub auto_save: bool,
    pub auto_save_delay_ms: u64,
    pub backup_before_save: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceConfig {
    pub theme_mode: ThemeMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    Light,
    Dark,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorConfig {
    pub font_size: u8,
    pub line_height: f32,
    pub show_line_numbers: bool,
    pub line_wrapping: bool,
    pub highlight_active_line: bool,
    pub tab_size: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutConfig {
    pub article_list_width: u16,
    pub preview_width: u16,
    pub preview_visible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HexoConfig {
    pub preview_port: u16,
    #[serde(default)]
    pub auto_start_preview: bool,
    #[serde(default = "default_true")]
    pub preview_drafts: bool,
    #[serde(default)]
    pub default_preview_mode: PreviewMode,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PreviewMode {
    #[default]
    Markdown,
    Theme,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageBedConfig {
    pub default_provider: ImageProvider,
    pub cloudflare_api_url: String,
    pub auto_insert_markdown: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ImageProvider {
    Local,
    CloudflareImgbed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishConfig {
    pub save_before_run: bool,
    pub clean_before_generate: bool,
    pub generate_before_deploy: bool,
    pub git_push_after_deploy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsConfig {
    #[serde(default = "default_log_retention_days")]
    pub log_retention_days: u8,
    #[serde(default = "default_log_storage_mb")]
    pub max_log_storage_mb: u16,
}

impl Default for DiagnosticsConfig {
    fn default() -> Self {
        Self {
            log_retention_days: default_log_retention_days(),
            max_log_storage_mb: default_log_storage_mb(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_log_retention_days() -> u8 {
    14
}

fn default_log_storage_mb() -> u16 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConfig {
    pub check_on_start: bool,
}

impl Default for AppConfigV3 {
    fn default() -> Self {
        Self {
            schema_version: 3,
            general: GeneralConfig {
                open_recent_project_on_start: true,
                auto_save: true,
                auto_save_delay_ms: 2_000,
                backup_before_save: false,
            },
            appearance: AppearanceConfig {
                theme_mode: ThemeMode::System,
            },
            editor: EditorConfig {
                font_size: 15,
                line_height: 1.65,
                show_line_numbers: true,
                line_wrapping: true,
                highlight_active_line: true,
                tab_size: 2,
            },
            article_list: ArticleListConfig::default(),
            layout: LayoutConfig {
                article_list_width: 280,
                preview_width: 380,
                preview_visible: true,
            },
            hexo: HexoConfig {
                preview_port: 4_000,
                auto_start_preview: false,
                preview_drafts: true,
                default_preview_mode: PreviewMode::Markdown,
            },
            image_bed: ImageBedConfig {
                default_provider: ImageProvider::Local,
                cloudflare_api_url: String::new(),
                auto_insert_markdown: true,
            },
            publish: PublishConfig {
                save_before_run: true,
                clean_before_generate: false,
                generate_before_deploy: true,
                git_push_after_deploy: false,
            },
            diagnostics: DiagnosticsConfig {
                log_retention_days: 14,
                max_log_storage_mb: 20,
            },
            update: UpdateConfig {
                check_on_start: true,
            },
        }
    }
}

impl AppConfigV3 {
    pub fn validate(&self) -> AppResult<()> {
        if self.schema_version != 3 {
            return Err(AppError::invalid("配置 schemaVersion 必须为 3。"));
        }
        if !(12..=28).contains(&self.editor.font_size) {
            return Err(AppError::invalid("编辑器字号必须在 12–28 之间。"));
        }
        if !(1.2..=2.2).contains(&self.editor.line_height) {
            return Err(AppError::invalid("编辑器行高必须在 1.2–2.2 之间。"));
        }
        if ![2, 4, 8].contains(&self.editor.tab_size) {
            return Err(AppError::invalid("Tab 宽度只能是 2、4 或 8。"));
        }
        if !(220..=420).contains(&self.layout.article_list_width)
            || !(280..=720).contains(&self.layout.preview_width)
        {
            return Err(AppError::invalid("编辑器栏宽超出允许范围。"));
        }
        if !(300..=65_535).contains(&self.hexo.preview_port) {
            return Err(AppError::invalid("Hexo 预览端口必须在 300–65535 之间。"));
        }
        if !(500..=30_000).contains(&self.general.auto_save_delay_ms) {
            return Err(AppError::invalid("自动保存延迟必须在 500–30000 毫秒之间。"));
        }
        if ![7, 14, 30].contains(&self.diagnostics.log_retention_days) {
            return Err(AppError::invalid("日志保留时间只能是 7、14 或 30 天。"));
        }
        if ![10, 20, 50].contains(&self.diagnostics.max_log_storage_mb) {
            return Err(AppError::invalid("日志空间上限只能是 10、20 或 50 MB。"));
        }
        let image_api = self.image_bed.cloudflare_api_url.trim();
        if !image_api.is_empty() {
            let url = url::Url::parse(image_api)
                .map_err(|_| AppError::invalid("Cloudflare-ImgBed API 地址无效。"))?;
            let local_debug = cfg!(debug_assertions)
                && url.scheme() == "http"
                && matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
            if (url.scheme() != "https" && !local_debug)
                || !url.username().is_empty()
                || url.password().is_some()
            {
                return Err(AppError::invalid(
                    "Cloudflare-ImgBed API 必须使用不含凭据的 HTTPS 地址。",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigLoadResult {
    pub config: AppConfigV3,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    pub configured: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskType {
    Clean,
    Generate,
    Deploy,
    Publish,
    GitStatus,
    ServerStart,
    ServerStop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvent {
    pub task_id: String,
    pub project_id: String,
    pub sequence: u64,
    pub kind: TaskEventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<TaskStream>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskEventKind {
    Queued,
    StepStarted,
    Log,
    StepFinished,
    Finished,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskReceipt {
    pub task_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalImage {
    pub image_id: String,
    pub name: String,
    pub relative_path: String,
    pub mime: String,
    pub size: u64,
    pub preview_url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorImageInput {
    pub name: String,
    pub mime: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageImportResult {
    pub file_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAssetItem {
    pub asset_id: String,
    pub kind: RemoteAssetKind,
    pub name: String,
    pub file_name: String,
    pub directory: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    pub can_preview: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RemoteAssetKind {
    Folder,
    Image,
    Archive,
    Document,
    Audio,
    Video,
    File,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAssetPage {
    pub current_directory: String,
    pub breadcrumbs: Vec<RemoteAssetBreadcrumb>,
    pub items: Vec<RemoteAssetItem>,
    pub total_count: usize,
    pub returned_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAssetBreadcrumb {
    pub name: String,
    pub directory: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PreviewServerState {
    Starting,
    Running,
    Stopping,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewServerView {
    pub project_id: String,
    pub session_generation: u64,
    pub state: PreviewServerState,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    pub drafts_enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskLogSummary {
    pub task_id: String,
    pub project_name: String,
    pub task_type: TaskType,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    pub size: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskLogPage {
    pub events: Vec<TaskEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProjectView {
    pub recent_id: String,
    pub name: String,
    pub display_path: String,
    pub last_opened_at: String,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResult {
    pub url: String,
    pub markdown: String,
    pub file_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub version: String,
    pub operating_system: String,
    pub architecture: String,
    pub webview: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub has_update: bool,
    pub release_notes: Option<String>,
    pub release_page_url: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExternalTarget {
    ProjectHomepage,
    License,
    ReleasePage,
    CloudflareDashboard,
    HexoPreview,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_config_ranges() {
        let mut config = AppConfigV3::default();
        config.editor.font_size = 40;
        assert_eq!(config.validate().unwrap_err().code, "invalid_request");

        let mut config = AppConfigV3::default();
        config.image_bed.cloudflare_api_url = "http://example.com/upload".to_string();
        assert!(config.validate().is_err());
    }
}
