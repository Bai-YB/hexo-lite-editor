use crate::domain::{
    AppError, AppResult, ArticleSummary, PreviewServerView, ProjectSessionView, RemoteAssetKind,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex, RwLock,
    },
    time::SystemTime,
};
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ArticleRecord {
    pub id: String,
    pub canonical_path: PathBuf,
    pub revision: u64,
}

#[derive(Debug, Clone)]
pub enum AssetSource {
    Disk(PathBuf),
    Memory(Arc<Vec<u8>>),
}

#[derive(Debug, Clone)]
pub struct AssetRecord {
    pub source: AssetSource,
    pub mime: String,
    pub generation: u64,
    pub expires_at: SystemTime,
}

#[derive(Debug, Clone)]
pub struct RemoteAssetRecord {
    pub delete_key: String,
    pub kind: RemoteAssetKind,
}

pub struct PreviewRuntime {
    pub view: PreviewServerView,
    pub cancellation: Option<oneshot::Sender<()>>,
}

#[derive(Debug, Clone)]
pub struct ProjectSession {
    pub id: String,
    pub generation: u64,
    pub name: String,
    pub root: PathBuf,
    pub warnings: Vec<String>,
    pub article_summaries: Vec<ArticleSummary>,
    pub articles: HashMap<String, ArticleRecord>,
    pub assets: HashMap<String, AssetRecord>,
    pub remote_assets: HashMap<String, RemoteAssetRecord>,
}

impl ProjectSession {
    pub fn view(&self) -> ProjectSessionView {
        ProjectSessionView {
            project_id: self.id.clone(),
            generation: self.generation,
            name: self.name.clone(),
            display_path: self.root.display().to_string(),
            warnings: self.warnings.clone(),
        }
    }

    pub fn require_identity(&self, project_id: &str, generation: Option<u64>) -> AppResult<()> {
        if self.id != project_id || generation.is_some_and(|value| value != self.generation) {
            return Err(AppError::session_expired());
        }
        Ok(())
    }

    pub fn article(&self, article_id: &str) -> AppResult<&ArticleRecord> {
        self.articles.get(article_id).ok_or_else(|| {
            AppError::new(
                "article_not_found",
                "文章不属于当前项目会话，可能已经被移动或删除。",
                true,
            )
        })
    }
}

pub struct AppState {
    pub project: RwLock<Option<ProjectSession>>,
    pub config_path: PathBuf,
    pub v2_config_path: PathBuf,
    pub legacy_config_path: PathBuf,
    pub recent_path: PathBuf,
    pub task_log_dir: PathBuf,
    pub config_write_lock: Mutex<()>,
    pub task_log_write_lock: Mutex<()>,
    pub save_locks: Mutex<HashMap<String, std::sync::Arc<Mutex<()>>>>,
    pub task_cancellations: Mutex<HashMap<String, oneshot::Sender<()>>>,
    pub preview: Mutex<Option<PreviewRuntime>>,
    pub shutdown_started: AtomicBool,
    generation: AtomicU64,
}

impl AppState {
    pub fn new(config_dir: &Path) -> Self {
        Self {
            project: RwLock::new(None),
            config_path: config_dir.join("config-v3.json"),
            v2_config_path: config_dir.join("config-v2.json"),
            legacy_config_path: config_dir.join("app-config.json"),
            recent_path: config_dir.join("recent-project.json"),
            task_log_dir: config_dir.join("task-logs"),
            config_write_lock: Mutex::new(()),
            task_log_write_lock: Mutex::new(()),
            save_locks: Mutex::new(HashMap::new()),
            task_cancellations: Mutex::new(HashMap::new()),
            preview: Mutex::new(None),
            shutdown_started: AtomicBool::new(false),
            generation: AtomicU64::new(0),
        }
    }

    pub fn next_generation(&self) -> u64 {
        self.generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    pub fn new_project_id() -> String {
        Uuid::new_v4().to_string()
    }

    pub fn with_project<T>(
        &self,
        project_id: &str,
        generation: Option<u64>,
        callback: impl FnOnce(&ProjectSession) -> AppResult<T>,
    ) -> AppResult<T> {
        let guard = self
            .project
            .read()
            .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
        let project = guard
            .as_ref()
            .ok_or_else(|| AppError::new("project_not_open", "请先打开一个 Hexo 项目。", true))?;
        project.require_identity(project_id, generation)?;
        callback(project)
    }

    pub fn article_save_lock(&self, article_id: &str) -> AppResult<std::sync::Arc<Mutex<()>>> {
        let mut locks = self
            .save_locks
            .lock()
            .map_err(|_| AppError::new("state_poisoned", "保存队列不可用。", false))?;
        Ok(locks
            .entry(article_id.to_string())
            .or_insert_with(|| std::sync::Arc::new(Mutex::new(())))
            .clone())
    }
}
