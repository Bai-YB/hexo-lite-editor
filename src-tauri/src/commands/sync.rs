use crate::{
    app::AppState,
    data::load_config,
    domain::{AppError, AppResult},
    platform::{
        atomic_write, delete_webdav_credentials, set_webdav_credentials, webdav_credentials,
        webdav_status,
    },
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::Duration,
};
use tauri::Emitter;
use uuid::Uuid;
use wait_timeout::ChildExt;
use walkdir::WalkDir;

const MANIFEST: &str = ".hexo-lite-sync.json";
const WEBDAV_OBJECTS: &str = ".hexo-lite-objects";
const DEFAULT_BRANCH: &str = "hexo-lite-content";
const MAX_SYNC_FILE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_BACKUPS: usize = 10;
const GIT_TIMEOUT: Duration = Duration::from_secs(30);
const STARTUP_GIT_TIMEOUT: Duration = Duration::from_secs(3);
const WEBDAV_TIMEOUT: Duration = Duration::from_secs(30);
const STARTUP_WEBDAV_TIMEOUT: Duration = Duration::from_secs(3);
const REMOTE_AHEAD_MESSAGE: &str = "远端内容已前进；不会覆盖远端，请先处理远端更新。";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum ContentSyncProvider {
    #[default]
    Github,
    Webdav,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentSyncStatus {
    Off,
    Checking,
    Synced,
    LocalPending,
    RemoteAhead,
    Conflict,
    Offline,
    AuthRequired,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSyncView {
    pub enabled: bool,
    pub status: ContentSyncStatus,
    pub provider: ContentSyncProvider,
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub endpoint: Option<String>,
    pub remote_dir: Option<String>,
    pub visibility: Option<String>,
    pub message: Option<String>,
    pub conflicts: Vec<String>,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSyncEvent {
    pub phase: String,
    pub status: ContentSyncStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSyncCandidate {
    pub repository: String,
    pub source: String,
    pub pages_branch: Option<String>,
    pub visibility: String,
    pub default_branch: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSyncDetection {
    pub candidates: Vec<ContentSyncCandidate>,
    pub requires_selection: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSyncPreflight {
    pub candidate: ContentSyncCandidate,
    pub branch: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub remote_file_count: usize,
    pub remote_total_bytes: u64,
    pub local_only_count: usize,
    pub remote_only_count: usize,
    pub different_count: usize,
    pub remote_branch_exists: bool,
    pub remote_manifest_valid: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavContentSyncPreflight {
    pub endpoint: String,
    pub remote_dir: String,
    pub file_count: usize,
    pub total_bytes: u64,
    pub remote_file_count: usize,
    pub remote_total_bytes: u64,
    pub local_only_count: usize,
    pub remote_only_count: usize,
    pub different_count: usize,
    pub remote_exists: bool,
    pub remote_manifest_valid: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavConnectionTestResult {
    pub preflight: WebDavContentSyncPreflight,
    pub username: String,
    pub tested_at: String,
    pub sync: ContentSyncView,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSyncConflict {
    pub path: String,
    pub kind: String,
    pub local_hash: Option<String>,
    pub remote_hash: Option<String>,
    pub local_size: Option<u64>,
    pub remote_size: Option<u64>,
    pub local_text: Option<String>,
    pub remote_text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureContentSyncRequest {
    pub project_id: String,
    pub session_generation: u64,
    #[serde(default = "default_branch")]
    pub branch: String,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub initial_choice: Option<String>,
    #[serde(default)]
    pub confirm_public: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSyncPreflightRequest {
    pub project_id: String,
    pub session_generation: u64,
    pub repository: String,
    #[serde(default = "default_branch")]
    pub branch: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebDavContentSyncPreflightRequest {
    pub project_id: String,
    pub session_generation: u64,
    pub endpoint: String,
    #[serde(default = "default_remote_dir")]
    pub remote_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigureWebDavContentSyncRequest {
    pub project_id: String,
    pub session_generation: u64,
    pub endpoint: String,
    #[serde(default = "default_remote_dir")]
    pub remote_dir: String,
    #[serde(default)]
    pub initial_choice: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestWebDavContentSyncRequest {
    pub project_id: String,
    pub session_generation: u64,
    pub endpoint: String,
    #[serde(default = "default_remote_dir")]
    pub remote_dir: String,
    pub username: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWebDavContentSyncRequest {
    pub project_id: String,
    pub session_generation: u64,
    pub endpoint: String,
    #[serde(default = "default_remote_dir")]
    pub remote_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveContentSyncConflictsRequest {
    pub project_id: String,
    pub session_generation: u64,
    pub choices: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunContentSyncRequest {
    pub project_id: String,
    pub session_generation: u64,
    #[serde(default = "default_auto")]
    pub direction: String,
}

fn default_auto() -> String {
    "auto".to_string()
}

fn default_branch() -> String {
    DEFAULT_BRANCH.to_string()
}

fn default_remote_dir() -> String {
    DEFAULT_BRANCH.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncRecord {
    project_path: String,
    #[serde(default)]
    provider: ContentSyncProvider,
    repository: String,
    branch: String,
    image_dir: String,
    enabled: bool,
    visibility: String,
    status: ContentSyncStatus,
    #[serde(default)]
    base_files: BTreeMap<String, String>,
    #[serde(default)]
    conflicts: Vec<String>,
    #[serde(default)]
    conflict_remote_head: Option<String>,
    #[serde(default)]
    remote_etag: Option<String>,
    #[serde(default)]
    remote_manifest_exists: bool,
    #[serde(default)]
    message: Option<String>,
    #[serde(default)]
    last_synced_at: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SyncRegistry {
    records: Vec<SyncRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncManifest {
    schema_version: u8,
    image_dir: String,
    files: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyTransaction {
    backup_name: String,
    operations: BTreeSet<String>,
}

type Snapshot = BTreeMap<String, FileSnapshot>;

#[derive(Debug, Clone)]
struct FileSnapshot {
    hash: String,
}

#[derive(Debug)]
struct GitFailure {
    message: String,
    auth: bool,
    offline: bool,
}

#[derive(Debug)]
struct RemoteFetch {
    exists: bool,
    head: Option<String>,
    etag: Option<String>,
}

#[tauri::command]
pub fn detect_content_sync(
    project_id: String,
    session_generation: u64,
    state: tauri::State<'_, AppState>,
) -> AppResult<ContentSyncDetection> {
    let root = project_root(&state, &project_id, session_generation)?;
    let candidates = detect_candidates(&root);
    Ok(ContentSyncDetection {
        requires_selection: candidates.len() > 1,
        candidates,
    })
}

#[tauri::command]
pub fn preflight_content_sync(
    request: ContentSyncPreflightRequest,
    state: tauri::State<'_, AppState>,
) -> AppResult<ContentSyncPreflight> {
    let root = project_root(&state, &request.project_id, request.session_generation)?;
    let repository = normalize_github_remote(&request.repository)
        .ok_or_else(|| AppError::invalid("同步仓库不是可接受的 GitHub 地址。"))?;
    let mut candidate = detect_candidates(&root)
        .into_iter()
        .find(|item| item.repository == repository)
        .ok_or_else(|| AppError::invalid("同步仓库不在当前项目检测结果中。"))?;
    let branch = validate_branch(&request.branch)?;
    if candidate.pages_branch.as_deref() == Some(branch.as_str()) {
        return Err(AppError::invalid(
            "内容分支不能与 Hexo Pages 发布分支相同。",
        ));
    }
    let config = load_config(&state)?.config;
    let snapshot = local_snapshot(&root, &config.image_bed.local_image_dir)?;
    let total_bytes = snapshot
        .keys()
        .filter_map(|path| fs::metadata(root.join(path)).ok().map(|value| value.len()))
        .sum();
    let cache = state.sync_cache_dir.join(cache_key(&path_key(&root)));
    ensure_cache(&cache, &repository).map_err(git_failure_error)?;
    let remote_branch_exists = fetch_remote_branch(&cache, &branch).map_err(git_failure_error)?;
    candidate.visibility = detect_github_visibility(&candidate.repository);
    let mut remote_snapshot = None;
    let remote_manifest_valid = if remote_branch_exists {
        checkout_fetched_branch(&cache).map_err(git_failure_error)?;
        read_manifest(&cache).is_some_and(|manifest| {
            if manifest.schema_version != 1
                || manifest.image_dir != config.image_bed.local_image_dir
            {
                return false;
            }
            match snapshot_from_manifest(&cache, &manifest) {
                Ok(value) => {
                    remote_snapshot = Some(value);
                    true
                }
                Err(_) => false,
            }
        })
    } else {
        false
    };
    let remote_total_bytes = remote_snapshot
        .as_ref()
        .map(|remote| {
            remote
                .keys()
                .filter_map(|path| fs::metadata(cache.join(path)).ok().map(|value| value.len()))
                .sum()
        })
        .unwrap_or(0);
    let mut local_only_count = 0;
    let mut remote_only_count = 0;
    let mut different_count = 0;
    if let Some(remote) = remote_snapshot.as_ref() {
        for path in snapshot
            .keys()
            .chain(remote.keys())
            .collect::<BTreeSet<_>>()
        {
            match (snapshot.get(path), remote.get(path)) {
                (Some(local), Some(remote)) if local.hash != remote.hash => different_count += 1,
                (Some(_), None) => local_only_count += 1,
                (None, Some(_)) => remote_only_count += 1,
                _ => {}
            }
        }
    }
    Ok(ContentSyncPreflight {
        candidate,
        branch,
        file_count: snapshot.len(),
        total_bytes,
        remote_file_count: remote_snapshot.as_ref().map_or(0, BTreeMap::len),
        remote_total_bytes,
        local_only_count,
        remote_only_count,
        different_count,
        remote_branch_exists,
        remote_manifest_valid,
    })
}

#[tauri::command]
pub fn webdav_credential_status(endpoint: String) -> AppResult<crate::domain::CredentialStatus> {
    let endpoint = normalize_webdav_endpoint(&endpoint)?;
    Ok(webdav_status(&endpoint))
}

#[tauri::command]
pub fn webdav_credential_delete(endpoint: String) -> AppResult<crate::domain::CredentialStatus> {
    let endpoint = normalize_webdav_endpoint(&endpoint)?;
    delete_webdav_credentials(&endpoint)
}

#[tauri::command]
pub fn test_webdav_content_sync(
    request: TestWebDavContentSyncRequest,
    state: tauri::State<'_, AppState>,
) -> AppResult<WebDavConnectionTestResult> {
    test_webdav_content_sync_inner(request, &state)
}

fn test_webdav_content_sync_inner(
    request: TestWebDavContentSyncRequest,
    state: &AppState,
) -> AppResult<WebDavConnectionTestResult> {
    let root = project_root(state, &request.project_id, request.session_generation)?;
    let endpoint = normalize_webdav_endpoint(&request.endpoint)?;
    let remote_dir = validate_webdav_remote_dir(&request.remote_dir)?;
    let username = request.username.trim().to_string();
    if username.is_empty() {
        return Err(AppError::invalid("WebDAV 用户名不能为空。"));
    }
    let credentials = if request.password.is_empty() {
        let stored = webdav_credentials(&endpoint).map_err(|_| {
            AppError::new(
                "webdav_credentials_required",
                "请输入 WebDAV 密码；留空仅适用于已保存凭据。",
                true,
            )
        })?;
        if stored.username != username {
            return Err(AppError::new(
                "webdav_password_required",
                "用户名已更改，请同时输入新密码。",
                true,
            ));
        }
        stored
    } else {
        crate::platform::WebDavCredentials {
            username,
            password: request.password,
        }
    };
    test_webdav_connection(&endpoint, &remote_dir, &credentials)?;
    let preflight =
        webdav_preflight_for_credentials(state, &root, &endpoint, &remote_dir, &credentials)?;
    if preflight.remote_exists && !preflight.remote_manifest_valid {
        return Err(AppError::new(
            "sync_manifest_invalid",
            "WebDAV 远端目录的同步清单缺失、损坏或与当前项目范围不匹配。请修复清单或更换远端目录。",
            true,
        ));
    }
    set_webdav_credentials(&endpoint, &credentials.username, &credentials.password)?;
    let key = path_key(&root);
    let mut registry = load_registry(state)?;
    let mut changed = false;
    if let Some(record) = registry.records.iter_mut().find(|record| {
        record.project_path == key
            && record.enabled
            && record.provider == ContentSyncProvider::Webdav
            && record.repository == endpoint
            && record.branch == remote_dir
    }) {
        if matches!(
            record.status,
            ContentSyncStatus::AuthRequired | ContentSyncStatus::Offline | ContentSyncStatus::Error
        ) {
            record.status = ContentSyncStatus::LocalPending;
            record.message = Some("WebDAV 凭据和服务器连接已验证，可以重新同步。".to_string());
            changed = true;
        }
    }
    if changed {
        save_registry(state, &registry)?;
    }
    let sync = registry
        .records
        .iter()
        .find(|record| record.project_path == key && record.enabled)
        .map(view_from_record)
        .unwrap_or_else(off_view);
    Ok(WebDavConnectionTestResult {
        preflight,
        username: credentials.username,
        tested_at: Local::now().to_rfc3339(),
        sync,
    })
}

#[tauri::command]
pub fn update_webdav_content_sync(
    request: UpdateWebDavContentSyncRequest,
    state: tauri::State<'_, AppState>,
) -> AppResult<ContentSyncView> {
    update_webdav_content_sync_request_inner(request, &state)
}

fn update_webdav_content_sync_request_inner(
    request: UpdateWebDavContentSyncRequest,
    state: &AppState,
) -> AppResult<ContentSyncView> {
    let root = project_root(state, &request.project_id, request.session_generation)?;
    let endpoint = normalize_webdav_endpoint(&request.endpoint)?;
    let remote_dir = validate_webdav_remote_dir(&request.remote_dir)?;
    let credentials = webdav_credentials(&endpoint).map_err(|_| {
        AppError::new(
            "webdav_credentials_required",
            "请先成功测试并保存 WebDAV 用户名和密码。",
            true,
        )
    })?;
    test_webdav_connection(&endpoint, &remote_dir, &credentials)?;
    update_webdav_content_sync_inner(state, &root, endpoint, remote_dir, credentials)
}

fn update_webdav_content_sync_inner(
    state: &AppState,
    root: &Path,
    endpoint: String,
    remote_dir: String,
    credentials: crate::platform::WebDavCredentials,
) -> AppResult<ContentSyncView> {
    let config = load_config(state)?.config;
    let cache = webdav_cache_dir(state, root);
    let remote = fetch_webdav_remote(&cache, &endpoint, &remote_dir, &credentials, WEBDAV_TIMEOUT)
        .map_err(git_failure_error)?;
    if remote.exists {
        let manifest = read_manifest(&cache).ok_or_else(|| {
            AppError::new(
                "sync_manifest_missing",
                "WebDAV 远端目录已有内容，但没有合法同步清单。请更换远端目录。",
                true,
            )
        })?;
        if manifest.schema_version != 1
            || manifest.image_dir != config.image_bed.local_image_dir
            || snapshot_from_manifest(&cache, &manifest).is_err()
        {
            return Err(AppError::new(
                "sync_manifest_invalid",
                "WebDAV 远端目录的同步清单无效，请更换远端目录。",
                true,
            ));
        }
    }
    let key = path_key(root);
    let mut registry = load_registry(state)?;
    let Some(record) = registry.records.iter_mut().find(|record| {
        record.project_path == key
            && record.enabled
            && record.provider == ContentSyncProvider::Webdav
    }) else {
        return Err(AppError::new(
            "sync_not_enabled",
            "当前项目未启用 WebDAV 内容同步。",
            true,
        ));
    };
    record.repository = endpoint;
    record.branch = remote_dir;
    record.remote_etag = remote.etag;
    record.remote_manifest_exists = remote.exists;
    record.base_files.clear();
    record.conflicts.clear();
    record.conflict_remote_head = remote.head;
    record.last_synced_at = None;
    record.status = ContentSyncStatus::LocalPending;
    record.message = Some("WebDAV 连接设置已应用，请选择首次同步方向。".to_string());
    let view = view_from_record(record);
    save_registry(state, &registry)?;
    Ok(view)
}

#[tauri::command]
pub fn preflight_webdav_content_sync(
    request: WebDavContentSyncPreflightRequest,
    state: tauri::State<'_, AppState>,
) -> AppResult<WebDavContentSyncPreflight> {
    let root = project_root(&state, &request.project_id, request.session_generation)?;
    let endpoint = normalize_webdav_endpoint(&request.endpoint)?;
    let remote_dir = validate_webdav_remote_dir(&request.remote_dir)?;
    let credentials = webdav_credentials(&endpoint).map_err(|_| {
        AppError::new(
            "webdav_credentials_required",
            "请先保存 WebDAV 用户名和密码。",
            true,
        )
    })?;
    webdav_preflight_for_credentials(&state, &root, &endpoint, &remote_dir, &credentials)
}

fn webdav_preflight_for_credentials(
    state: &AppState,
    root: &Path,
    endpoint: &str,
    remote_dir: &str,
    credentials: &crate::platform::WebDavCredentials,
) -> AppResult<WebDavContentSyncPreflight> {
    let config = load_config(state)?.config;
    let snapshot = local_snapshot(root, &config.image_bed.local_image_dir)?;
    let total_bytes = snapshot_total_bytes(root, &snapshot);
    let cache = webdav_cache_dir(state, root);
    let fetched = fetch_webdav_remote(
        cache.as_path(),
        endpoint,
        remote_dir,
        credentials,
        WEBDAV_TIMEOUT,
    )
    .map_err(git_failure_error)?;
    let mut remote_snapshot = None;
    let remote_manifest_valid = if fetched.exists {
        read_manifest(&cache).is_some_and(|manifest| {
            if manifest.schema_version != 1
                || manifest.image_dir != config.image_bed.local_image_dir
            {
                return false;
            }
            match snapshot_from_manifest(&cache, &manifest) {
                Ok(value) => {
                    remote_snapshot = Some(value);
                    true
                }
                Err(_) => false,
            }
        })
    } else {
        false
    };
    let remote_total_bytes = remote_snapshot
        .as_ref()
        .map(|value| snapshot_total_bytes(&cache, value))
        .unwrap_or(0);
    let (local_only_count, remote_only_count, different_count) =
        snapshot_difference_counts(&snapshot, remote_snapshot.as_ref());
    Ok(WebDavContentSyncPreflight {
        endpoint: endpoint.to_string(),
        remote_dir: remote_dir.to_string(),
        file_count: snapshot.len(),
        total_bytes,
        remote_file_count: remote_snapshot.as_ref().map_or(0, BTreeMap::len),
        remote_total_bytes,
        local_only_count,
        remote_only_count,
        different_count,
        remote_exists: fetched.exists,
        remote_manifest_valid,
    })
}

#[tauri::command]
pub fn get_content_sync_status(
    project_id: String,
    session_generation: u64,
    state: tauri::State<'_, AppState>,
) -> AppResult<ContentSyncView> {
    let root = project_root(&state, &project_id, session_generation)?;
    let registry = load_registry(&state)?;
    Ok(registry
        .records
        .iter()
        .find(|record| record.project_path == path_key(&root))
        .map(view_from_record)
        .unwrap_or_else(off_view))
}

#[tauri::command]
pub fn enable_content_sync(
    request: ConfigureContentSyncRequest,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> AppResult<ContentSyncView> {
    let root = project_root(&state, &request.project_id, request.session_generation)?;
    let candidates = detect_candidates(&root);
    let candidate = match request.repository.as_deref() {
        Some(repository) => {
            let normalized = normalize_github_remote(repository)
                .ok_or_else(|| AppError::invalid("同步仓库不是可接受的 GitHub 地址。"))?;
            candidates
                .into_iter()
                .find(|item| item.repository == normalized)
        }
        None if candidates.len() == 1 => candidates.into_iter().next(),
        None if candidates.len() > 1 => {
            return Err(AppError::new(
                "sync_repository_selection_required",
                "检测到多个 Git deploy 仓库，请明确选择一个。",
                true,
            ));
        }
        None => None,
    }
    .ok_or_else(|| {
        AppError::new(
            "sync_repository_not_found",
            "没有检测到 GitHub Pages 或 GitHub deploy 仓库。",
            true,
        )
    })?;
    let branch = validate_branch(&request.branch)?;
    if candidate.pages_branch.as_deref() == Some(branch.as_str()) {
        return Err(AppError::invalid(
            "内容分支不能与 Hexo Pages 发布分支相同。",
        ));
    }
    if candidate.visibility != "private" && !request.confirm_public {
        return Err(AppError::new(
            "sync_public_confirmation_required",
            "目标 GitHub 仓库公开，内容分支也会公开。请先确认。",
            true,
        ));
    }
    let config = load_config(&state)?.config;
    let cache = state.sync_cache_dir.join(cache_key(&path_key(&root)));
    ensure_cache(&cache, &candidate.repository).map_err(git_failure_error)?;
    if fetch_remote_branch(&cache, &branch).map_err(git_failure_error)? {
        checkout_fetched_branch(&cache).map_err(git_failure_error)?;
        let manifest = read_manifest(&cache).ok_or_else(|| {
            AppError::new(
                "sync_manifest_missing",
                "远端已有同名分支但没有合法同步清单，请更换分支名。",
                true,
            )
        })?;
        if manifest.schema_version != 1
            || manifest.image_dir != config.image_bed.local_image_dir
            || snapshot_from_manifest(&cache, &manifest).is_err()
        {
            return Err(AppError::new(
                "sync_manifest_invalid",
                "远端已有同名分支但同步清单无效，请更换分支名。",
                true,
            ));
        }
    }
    let mut registry = load_registry(&state)?;
    let key = path_key(&root);
    let record = SyncRecord {
        project_path: key.clone(),
        provider: ContentSyncProvider::Github,
        repository: candidate.repository,
        branch,
        image_dir: config.image_bed.local_image_dir.clone(),
        enabled: true,
        visibility: candidate.visibility,
        status: ContentSyncStatus::LocalPending,
        base_files: BTreeMap::new(),
        conflicts: Vec::new(),
        conflict_remote_head: None,
        remote_etag: None,
        remote_manifest_exists: false,
        message: Some("同步已启用，等待首次选择同步方向。".to_string()),
        last_synced_at: None,
    };
    registry.records.retain(|item| item.project_path != key);
    registry.records.push(record);
    save_registry(&state, &registry)?;

    if let Some(choice) = request.initial_choice.as_deref() {
        let before = local_snapshot(&root, &config.image_bed.local_image_dir)
            .ok()
            .map(|value| hash_map(&value));
        emit_sync_phase(&app, "checking", ContentSyncStatus::Checking, None);
        let view = run_sync_for_root(&state, &root, choice);
        emit_rescan_if_changed(
            &app,
            &state,
            &root,
            &config.image_bed.local_image_dir,
            before,
        );
        let _ = app.emit("content-sync-status", &view);
        emit_sync_phase(
            &app,
            sync_phase_for_status(&view.status),
            view.status.clone(),
            view.message.clone(),
        );
        return Ok(view);
    }
    Ok(registry
        .records
        .iter()
        .find(|item| item.project_path == key)
        .map(view_from_record)
        .unwrap_or_else(off_view))
}

#[tauri::command]
pub fn enable_webdav_content_sync(
    request: ConfigureWebDavContentSyncRequest,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> AppResult<ContentSyncView> {
    let root = project_root(&state, &request.project_id, request.session_generation)?;
    let endpoint = normalize_webdav_endpoint(&request.endpoint)?;
    let remote_dir = validate_webdav_remote_dir(&request.remote_dir)?;
    let credentials = webdav_credentials(&endpoint).map_err(|_| {
        AppError::new(
            "webdav_credentials_required",
            "请先保存 WebDAV 用户名和密码。",
            true,
        )
    })?;
    test_webdav_connection(&endpoint, &remote_dir, &credentials)?;
    let config = load_config(&state)?.config;
    let cache = webdav_cache_dir(&state, &root);
    let remote = fetch_webdav_remote(&cache, &endpoint, &remote_dir, &credentials, WEBDAV_TIMEOUT)
        .map_err(git_failure_error)?;
    if remote.exists {
        let manifest = read_manifest(&cache).ok_or_else(|| {
            AppError::new(
                "sync_manifest_missing",
                "WebDAV 远端目录已有内容，但没有合法同步清单。请更换远端目录。",
                true,
            )
        })?;
        if manifest.schema_version != 1
            || manifest.image_dir != config.image_bed.local_image_dir
            || snapshot_from_manifest(&cache, &manifest).is_err()
        {
            return Err(AppError::new(
                "sync_manifest_invalid",
                "WebDAV 远端目录的同步清单无效，请更换远端目录。",
                true,
            ));
        }
    }
    let mut registry = load_registry(&state)?;
    let key = path_key(&root);
    let record = SyncRecord {
        project_path: key.clone(),
        provider: ContentSyncProvider::Webdav,
        repository: endpoint,
        branch: remote_dir,
        image_dir: config.image_bed.local_image_dir.clone(),
        enabled: true,
        visibility: "private".to_string(),
        status: ContentSyncStatus::LocalPending,
        base_files: BTreeMap::new(),
        conflicts: Vec::new(),
        conflict_remote_head: remote.head,
        remote_etag: remote.etag,
        remote_manifest_exists: remote.exists,
        message: Some("WebDAV 同步已启用，等待首次选择同步方向。".to_string()),
        last_synced_at: None,
    };
    registry.records.retain(|item| item.project_path != key);
    registry.records.push(record);
    save_registry(&state, &registry)?;
    if let Some(choice) = request.initial_choice.as_deref() {
        let before = local_snapshot(&root, &config.image_bed.local_image_dir)
            .ok()
            .map(|value| hash_map(&value));
        emit_sync_phase(&app, "checking", ContentSyncStatus::Checking, None);
        let view = run_sync_for_root(&state, &root, choice);
        emit_rescan_if_changed(
            &app,
            &state,
            &root,
            &config.image_bed.local_image_dir,
            before,
        );
        let _ = app.emit("content-sync-status", &view);
        emit_sync_phase(
            &app,
            sync_phase_for_status(&view.status),
            view.status.clone(),
            view.message.clone(),
        );
        return Ok(view);
    }
    Ok(registry
        .records
        .iter()
        .find(|item| item.project_path == key)
        .map(view_from_record)
        .unwrap_or_else(off_view))
}

#[tauri::command]
pub fn disable_content_sync(
    project_id: String,
    session_generation: u64,
    state: tauri::State<'_, AppState>,
) -> AppResult<ContentSyncView> {
    let root = project_root(&state, &project_id, session_generation)?;
    let mut registry = load_registry(&state)?;
    registry
        .records
        .retain(|item| item.project_path != path_key(&root));
    save_registry(&state, &registry)?;
    Ok(off_view())
}

#[tauri::command]
pub fn run_content_sync(
    request: RunContentSyncRequest,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> AppResult<ContentSyncView> {
    let root = project_root(&state, &request.project_id, request.session_generation)?;
    emit_sync_phase(&app, "checking", ContentSyncStatus::Checking, None);
    let config = load_config(&state)?.config;
    let before = local_snapshot(&root, &config.image_bed.local_image_dir)
        .ok()
        .map(|value| hash_map(&value));
    emit_sync_phase(&app, "checking", ContentSyncStatus::Checking, None);
    let view = run_sync_for_root(&state, &root, &request.direction);
    let _ = app.emit("content-sync-status", &view);
    emit_sync_phase(
        &app,
        sync_phase_for_status(&view.status),
        view.status.clone(),
        view.message.clone(),
    );
    emit_rescan_if_changed(
        &app,
        &state,
        &root,
        &config.image_bed.local_image_dir,
        before,
    );
    Ok(view)
}

#[tauri::command]
pub fn get_content_sync_conflicts(
    project_id: String,
    session_generation: u64,
    state: tauri::State<'_, AppState>,
) -> AppResult<Vec<ContentSyncConflict>> {
    let root = project_root(&state, &project_id, session_generation)?;
    let key = path_key(&root);
    let registry = load_registry(&state)?;
    let record = registry
        .records
        .iter()
        .find(|item| item.project_path == key && item.enabled)
        .ok_or_else(|| AppError::new("sync_not_enabled", "当前项目未启用内容同步。", true))?;
    let local = local_snapshot(&root, &record.image_dir)?;
    let cache = record_cache_dir(&state, &root, record);
    prepare_remote_cache(&cache, record).map_err(git_failure_error)?;
    let fetched = fetch_record_remote(&cache, record, WEBDAV_TIMEOUT.max(GIT_TIMEOUT))
        .map_err(git_failure_error)?;
    if !fetched.exists {
        return Ok(Vec::new());
    }
    if record.conflict_remote_head != fetched.head {
        return Err(AppError::new(
            "sync_remote_changed",
            "冲突确认期间远端内容已经前进，请重新检查后再选择。",
            true,
        ));
    }
    let manifest = read_manifest(&cache)
        .ok_or_else(|| AppError::new("sync_manifest_invalid", "远端同步清单无效。", true))?;
    let remote = snapshot_from_manifest(&cache, &manifest)?;
    record
        .conflicts
        .iter()
        .map(|path| conflict_view(&root, &cache, path, &local, &remote))
        .collect()
}

#[tauri::command]
pub fn resolve_content_sync_conflicts(
    request: ResolveContentSyncConflictsRequest,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> AppResult<ContentSyncView> {
    let root = project_root(&state, &request.project_id, request.session_generation)?;
    let key = path_key(&root);
    let sync_lock = state.content_sync_lock(&key)?;
    let _guard = sync_lock
        .try_lock()
        .map_err(|_| AppError::new("sync_busy", "此项目已有同步任务正在运行。", true))?;
    let mut registry = load_registry(&state)?;
    let record = registry
        .records
        .iter_mut()
        .find(|item| item.project_path == key && item.enabled)
        .ok_or_else(|| AppError::new("sync_not_enabled", "当前项目未启用内容同步。", true))?;
    if record.conflicts.is_empty()
        || record.conflicts.iter().any(|path| {
            !matches!(
                request.choices.get(path).map(String::as_str),
                Some("local" | "remote")
            )
        })
        || request
            .choices
            .keys()
            .any(|path| !record.conflicts.contains(path))
    {
        return Err(AppError::invalid("请为每个冲突文件选择本地或远端版本。"));
    }
    let cache = record_cache_dir(&state, &root, record);
    prepare_remote_cache(&cache, record).map_err(git_failure_error)?;
    let fetched = fetch_record_remote(&cache, record, WEBDAV_TIMEOUT.max(GIT_TIMEOUT))
        .map_err(git_failure_error)?;
    if !fetched.exists {
        return Err(AppError::new(
            "sync_remote_changed",
            "远端内容分支已变化，请重新检查。",
            true,
        ));
    }
    if record.conflict_remote_head != fetched.head {
        return Err(AppError::new(
            "sync_remote_changed",
            "冲突确认期间远端内容已经变化，请重新检查。",
            true,
        ));
    }
    record.remote_etag = fetched.etag;
    record.remote_manifest_exists = fetched.exists;
    let manifest = read_manifest(&cache)
        .ok_or_else(|| AppError::new("sync_manifest_invalid", "远端同步清单无效。", true))?;
    let remote = snapshot_from_manifest(&cache, &manifest)?;
    let local = local_snapshot(&root, &record.image_dir)?;
    let before = Some(hash_map(&local));
    let image_dir = record.image_dir.clone();
    apply_conflict_choices(
        &state,
        &root,
        &cache,
        record,
        &local,
        &remote,
        &request.choices,
    )?;
    let snapshot = local_snapshot(&root, &record.image_dir)?;
    let view = finish_local_push(
        &state,
        &root,
        &cache,
        record.clone(),
        snapshot,
        &mut registry,
    );
    emit_rescan_if_changed(&app, &state, &root, &image_dir, before);
    let _ = app.emit("content-sync-status", &view);
    emit_sync_phase(
        &app,
        sync_phase_for_status(&view.status),
        view.status.clone(),
        view.message.clone(),
    );
    Ok(view)
}

#[tauri::command]
pub fn open_content_sync_backups(
    project_id: String,
    session_generation: u64,
    state: tauri::State<'_, AppState>,
) -> AppResult<()> {
    let root = project_root(&state, &project_id, session_generation)?;
    let directory = state.sync_backup_dir.join(cache_key(&path_key(&root)));
    fs::create_dir_all(&directory).map_err(|error| AppError::io("打开同步备份目录失败", error))?;
    open::that_detached(&directory).map_err(|error| AppError::io("打开同步备份目录失败", error))
}

#[tauri::command]
pub fn reconnect_content_sync(
    project_id: String,
    session_generation: u64,
    state: tauri::State<'_, AppState>,
) -> AppResult<ContentSyncView> {
    let root = project_root(&state, &project_id, session_generation)?;
    let key = path_key(&root);
    let mut registry = load_registry(&state)?;
    let record = registry
        .records
        .iter_mut()
        .find(|record| record.project_path == key && record.enabled)
        .ok_or_else(|| AppError::new("sync_not_enabled", "当前项目未启用内容同步。", true))?;
    let cache = record_cache_dir(&state, &root, record);
    prepare_remote_cache(&cache, record).map_err(git_failure_error)?;
    let result = match record.provider {
        ContentSyncProvider::Github => {
            let refspec = format!("refs/heads/{}", record.branch);
            git(&cache, &["ls-remote", "--heads", "origin", &refspec], true).map(|_| ())
        }
        ContentSyncProvider::Webdav => {
            fetch_record_remote(&cache, record, WEBDAV_TIMEOUT).map(|_| ())
        }
    };
    if let Err(error) = result {
        record.status = if error.auth {
            ContentSyncStatus::AuthRequired
        } else if error.offline {
            ContentSyncStatus::Offline
        } else {
            ContentSyncStatus::Error
        };
        record.message = Some(error.message);
    } else {
        record.status = ContentSyncStatus::LocalPending;
        record.message = Some(match record.provider {
            ContentSyncProvider::Github => "系统 Git 认证可用，可以重新同步。".to_string(),
            ContentSyncProvider::Webdav => {
                "WebDAV 凭据和服务器连接可用，可以重新同步。".to_string()
            }
        });
    }
    let view = view_from_record(record);
    save_registry(&state, &registry)?;
    Ok(view)
}

pub fn sync_before_open(state: &AppState, root: &Path) {
    let Ok(registry) = load_registry(state) else {
        return;
    };
    if registry
        .records
        .iter()
        .any(|record| record.enabled && record.project_path == path_key(root))
    {
        let _ = run_sync_for_root(state, root, "startup");
    }
}

pub fn schedule_sync_after_save(app: tauri::AppHandle, root: PathBuf) {
    use tauri::Manager;
    let state = app.state::<AppState>();
    let key = path_key(&root);
    if let Some(view) = mark_local_pending(&state, &key) {
        let _ = app.emit("content-sync-status", view);
        emit_sync_phase(
            &app,
            "waiting",
            ContentSyncStatus::LocalPending,
            Some("等待保存后 30 秒空闲窗口。".to_string()),
        );
    }
    let (cancel, cancelled) = tokio::sync::oneshot::channel();
    if let Ok(mut schedules) = state.sync_schedules.lock() {
        if let Some(previous) = schedules.insert(key.clone(), cancel) {
            let _ = previous.send(());
        }
    }
    tauri::async_runtime::spawn(async move {
        let timeout = tokio::time::sleep(std::time::Duration::from_secs(30));
        tokio::pin!(timeout);
        tokio::select! {
            _ = &mut timeout => {
                let app_for_task = app.clone();
                let root_for_task = root.clone();
                let _ = tauri::async_runtime::spawn_blocking(move || {
                    let state_for_task = app_for_task.state::<AppState>();
                    let view = run_sync_for_root(&state_for_task, &root_for_task, "push");
                    let _ = app_for_task.emit("content-sync-status", &view);
                    emit_sync_phase(
                        &app_for_task,
                        sync_phase_for_status(&view.status),
                        view.status.clone(),
                        view.message.clone(),
                    );
                    view
                }).await;
            }
            _ = cancelled => {}
        }
        let state = app.state::<AppState>();
        if let Ok(mut schedules) = state.sync_schedules.lock() {
            schedules.remove(&key);
        };
    });
}

fn emit_sync_phase(
    app: &tauri::AppHandle,
    phase: &str,
    status: ContentSyncStatus,
    message: Option<String>,
) {
    let _ = app.emit(
        "content-sync-phase",
        ContentSyncEvent {
            phase: phase.to_string(),
            status,
            message,
        },
    );
}

fn sync_phase_for_status(status: &ContentSyncStatus) -> &'static str {
    match status {
        ContentSyncStatus::Checking => "checking",
        ContentSyncStatus::LocalPending => "waiting",
        ContentSyncStatus::RemoteAhead | ContentSyncStatus::Conflict => "attention",
        ContentSyncStatus::Offline | ContentSyncStatus::AuthRequired | ContentSyncStatus::Error => {
            "failed"
        }
        ContentSyncStatus::Off | ContentSyncStatus::Synced => "completed",
    }
}

fn mark_local_pending(state: &AppState, key: &str) -> Option<ContentSyncView> {
    let mut registry = load_registry(state).ok()?;
    let record = registry
        .records
        .iter_mut()
        .find(|record| record.project_path == key && record.enabled)?;
    if !matches!(
        record.status,
        ContentSyncStatus::Conflict | ContentSyncStatus::RemoteAhead
    ) {
        record.status = ContentSyncStatus::LocalPending;
        record.message = Some("本地内容已保存，将在空闲 30 秒后推送。".to_string());
    }
    let view = view_from_record(record);
    let _ = save_registry(state, &registry);
    Some(view)
}

fn project_root(state: &AppState, project_id: &str, generation: u64) -> AppResult<PathBuf> {
    state.with_project(project_id, Some(generation), |project| {
        Ok(project.root.clone())
    })
}

fn run_sync_for_root(state: &AppState, root: &Path, direction: &str) -> ContentSyncView {
    let key = path_key(root);
    let sync_lock = match state.content_sync_lock(&key) {
        Ok(lock) => lock,
        Err(error) => return error_view(error.message),
    };
    let Ok(_guard) = sync_lock.try_lock() else {
        return current_view_for_root(state, root).unwrap_or_else(|| ContentSyncView {
            enabled: true,
            status: ContentSyncStatus::Checking,
            provider: ContentSyncProvider::Github,
            repository: None,
            branch: None,
            endpoint: None,
            remote_dir: None,
            visibility: None,
            message: Some("此项目已有同步任务正在运行。".to_string()),
            conflicts: Vec::new(),
            last_synced_at: None,
        });
    };
    run_sync_for_root_locked(state, root, direction)
}

fn run_sync_for_root_locked(state: &AppState, root: &Path, direction: &str) -> ContentSyncView {
    let key = path_key(root);
    if let Err(error) = recover_pending_transaction(state, root) {
        return error_view(error.message);
    }
    let mut registry = match load_registry(state) {
        Ok(value) => value,
        Err(error) => return error_view(error.message),
    };
    let Some(record) = registry
        .records
        .iter_mut()
        .find(|item| item.project_path == key)
    else {
        return off_view();
    };
    record.status = ContentSyncStatus::Checking;
    record.message = Some(match record.provider {
        ContentSyncProvider::Github => "正在检查 GitHub 内容分支。".to_string(),
        ContentSyncProvider::Webdav => "正在检查 WebDAV 远端目录。".to_string(),
    });
    let snapshot = match local_snapshot(root, &record.image_dir) {
        Ok(value) => value,
        Err(error) => return update_error(&mut registry, state, &key, error.message),
    };
    let cache = record_cache_dir(state, root, record);
    if let Err(error) = prepare_remote_cache(&cache, record) {
        return update_git_error(&mut registry, state, &key, error);
    }

    let remote_fetch = match fetch_record_remote(
        &cache,
        record,
        if direction == "startup" {
            match record.provider {
                ContentSyncProvider::Github => STARTUP_GIT_TIMEOUT,
                ContentSyncProvider::Webdav => STARTUP_WEBDAV_TIMEOUT,
            }
        } else {
            match record.provider {
                ContentSyncProvider::Github => GIT_TIMEOUT,
                ContentSyncProvider::Webdav => WEBDAV_TIMEOUT,
            }
        },
    ) {
        Ok(value) => value,
        Err(error) => return update_git_error(&mut registry, state, &key, error),
    };
    record.remote_etag = remote_fetch.etag.clone();
    record.remote_manifest_exists = remote_fetch.exists;
    if !remote_fetch.exists {
        if direction != "local" {
            record.status = ContentSyncStatus::LocalPending;
            record.message = Some(match record.provider {
                ContentSyncProvider::Github => {
                    "远端内容分支不存在，请确认创建并上传本地文章。".to_string()
                }
                ContentSyncProvider::Webdav => {
                    "WebDAV 远端目录尚未初始化，请确认创建并上传本地文章。".to_string()
                }
            });
            let view = view_from_record(record);
            let _ = save_registry(state, &registry);
            return view;
        }
        if record.provider == ContentSyncProvider::Github {
            if let Err(error) = prepare_orphan_cache(&cache) {
                return update_git_error(&mut registry, state, &key, error);
            }
        }
        return finish_local_push(state, root, &cache, record.clone(), snapshot, &mut registry);
    }

    let Some(manifest) = read_manifest(&cache) else {
        record.status = ContentSyncStatus::Error;
        record.message = Some("远端分支没有 Hexo Lite Editor 同步清单，已拒绝接管。".to_string());
        let view = view_from_record(record);
        let _ = save_registry(state, &registry);
        return view;
    };
    if manifest.schema_version != 1 {
        record.status = ContentSyncStatus::Error;
        record.message = Some("远端同步清单版本不受支持。".to_string());
        let view = view_from_record(record);
        let _ = save_registry(state, &registry);
        return view;
    }
    if manifest.image_dir != record.image_dir {
        record.status = ContentSyncStatus::Error;
        record.message = Some("远端同步清单的图片目录与当前配置不一致。".to_string());
        let view = view_from_record(record);
        let _ = save_registry(state, &registry);
        return view;
    }
    let remote = match snapshot_from_manifest(&cache, &manifest) {
        Ok(value) => value,
        Err(error) => return update_error(&mut registry, state, &key, error.message),
    };

    if record.base_files.is_empty() {
        if direction == "remote" {
            let local_base = hash_map(&snapshot);
            if let Err(error) = apply_remote(state, root, &cache, &snapshot, &remote, &local_base) {
                return update_error(&mut registry, state, &key, error.message);
            }
            record.base_files = hash_map(&remote);
            record.status = ContentSyncStatus::Synced;
            record.message = Some("已使用远端内容初始化本地项目。".to_string());
            record.last_synced_at = Some(Local::now().to_rfc3339());
        } else if direction == "local" {
            return finish_local_push(state, root, &cache, record.clone(), snapshot, &mut registry);
        } else {
            record.status = ContentSyncStatus::LocalPending;
            record.message = Some("远端分支已有内容，请选择使用远端或上传本地。".to_string());
        }
        let view = view_from_record(record);
        let _ = save_registry(state, &registry);
        return view;
    }

    let base = &record.base_files;
    let mut conflicts = Vec::new();
    let mut remote_only = BTreeSet::new();
    let mut local_only = BTreeSet::new();
    let paths = base
        .keys()
        .chain(snapshot.keys())
        .chain(remote.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for path in paths {
        let local_hash = snapshot.get(&path).map(|item| item.hash.as_str());
        let remote_hash = remote.get(&path).map(|item| item.hash.as_str());
        let base_hash = base.get(&path).map(String::as_str);
        let local_changed = local_hash != base_hash;
        let remote_changed = remote_hash != base_hash;
        if local_changed && remote_changed && local_hash != remote_hash {
            conflicts.push(path);
        } else if remote_changed && !local_changed {
            remote_only.insert(path);
        } else if local_changed && !remote_changed {
            local_only.insert(path);
        }
    }
    if !conflicts.is_empty() {
        record.status = ContentSyncStatus::Conflict;
        record.conflicts = conflicts;
        record.conflict_remote_head = remote_fetch.head;
        record.remote_etag = remote_fetch.etag;
        record.message = Some("本地和远端同时修改了相同文件，需要逐文件确认。".to_string());
        let view = view_from_record(record);
        let _ = save_registry(state, &registry);
        return view;
    }
    if !matches!(direction, "startup" | "remote") && !remote_only.is_empty() {
        record.status = ContentSyncStatus::RemoteAhead;
        record.message =
            Some("远端内容已前进；写作期间不会自动拉取，请手动处理远端更新。".to_string());
        let view = view_from_record(record);
        let _ = save_registry(state, &registry);
        return view;
    }
    if !remote_only.is_empty() {
        let remote_changes = remote_only
            .iter()
            .filter_map(|path| remote.get(path).map(|item| (path.clone(), item.clone())))
            .collect::<Snapshot>();
        let remote_base = remote_only
            .iter()
            .map(|path| (path.clone(), base.get(path).cloned().unwrap_or_default()))
            .collect::<BTreeMap<_, _>>();
        if let Err(error) = apply_remote(
            state,
            root,
            &cache,
            &snapshot,
            &remote_changes,
            &remote_base,
        ) {
            return update_error(&mut registry, state, &key, error.message);
        }
    }
    if !local_only.is_empty() {
        let after = local_snapshot(root, &record.image_dir).unwrap_or(snapshot);
        return finish_local_push(state, root, &cache, record.clone(), after, &mut registry);
    }
    record.base_files = hash_map(&remote);
    record.conflicts.clear();
    record.conflict_remote_head = None;
    record.status = ContentSyncStatus::Synced;
    record.message = Some("本地与远端内容已经同步。".to_string());
    record.last_synced_at = Some(Local::now().to_rfc3339());
    let view = view_from_record(record);
    let _ = save_registry(state, &registry);
    view
}

fn current_view_for_root(state: &AppState, root: &Path) -> Option<ContentSyncView> {
    let key = path_key(root);
    load_registry(state)
        .ok()?
        .records
        .iter()
        .find(|record| record.project_path == key)
        .map(view_from_record)
}

pub fn content_sync_view_for_root(state: &AppState, root: &Path) -> Option<ContentSyncView> {
    current_view_for_root(state, root)
}

fn emit_rescan_if_changed(
    app: &tauri::AppHandle,
    state: &AppState,
    root: &Path,
    image_dir: &str,
    before: Option<BTreeMap<String, String>>,
) {
    let after = local_snapshot(root, image_dir)
        .ok()
        .map(|value| hash_map(&value));
    if before != after {
        if let Ok(project) = super::project::rescan_project_after_sync(state, root) {
            let _ = app.emit("project-rescanned", project);
        }
    }
}

fn finish_local_push(
    state: &AppState,
    root: &Path,
    cache: &Path,
    mut record: SyncRecord,
    snapshot: Snapshot,
    registry: &mut SyncRegistry,
) -> ContentSyncView {
    let remote_existed = record.remote_manifest_exists;
    let copied = match record.provider {
        ContentSyncProvider::Github => {
            copy_snapshot_to_cache(root, cache, &snapshot, &record.image_dir)
        }
        ContentSyncProvider::Webdav => copy_snapshot_to_plain_cache(root, cache, &snapshot),
    };
    if let Err(error) = copied {
        return update_error(registry, state, &record.project_path, error.message);
    }
    let manifest = SyncManifest {
        schema_version: 1,
        image_dir: record.image_dir.clone(),
        files: hash_map(&snapshot),
    };
    let published = write_manifest(cache, &manifest).and_then(|_| match record.provider {
        ContentSyncProvider::Github => commit_and_push(cache, &record.branch),
        ContentSyncProvider::Webdav => publish_webdav_remote(
            cache,
            &record.repository,
            &record.branch,
            &manifest,
            remote_existed,
            record.remote_etag.as_deref(),
        ),
    });
    if let Err(error) = published {
        return update_git_error(registry, state, &record.project_path, error);
    }
    record.base_files = manifest.files;
    record.remote_manifest_exists = true;
    record.conflicts.clear();
    record.conflict_remote_head = None;
    record.status = ContentSyncStatus::Synced;
    record.message = Some(match record.provider {
        ContentSyncProvider::Github => "本地文章已推送到内容分支。".to_string(),
        ContentSyncProvider::Webdav => "本地文章已上传到 WebDAV 远端目录。".to_string(),
    });
    record.last_synced_at = Some(Local::now().to_rfc3339());
    let view = view_from_record(&record);
    if let Some(stored) = registry
        .records
        .iter_mut()
        .find(|item| item.project_path == record.project_path)
    {
        *stored = record;
    }
    let _ = save_registry(state, registry);
    view
}

fn apply_remote(
    state: &AppState,
    root: &Path,
    cache: &Path,
    local: &Snapshot,
    remote: &Snapshot,
    base: &BTreeMap<String, String>,
) -> Result<(), GitFailure> {
    let backup =
        backup_local_files(state, root, local, remote, base).map_err(|error| GitFailure {
            message: error.message,
            auth: false,
            offline: false,
        })?;
    let transaction = state
        .sync_cache_dir
        .join(cache_key(&path_key(root)))
        .join("apply-transaction.json");
    let operations = remote
        .keys()
        .chain(base.keys().filter(|path| !remote.contains_key(*path)))
        .cloned()
        .collect::<BTreeSet<_>>();
    let backup_name = backup
        .as_ref()
        .and_then(|path| path.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    let journal = ApplyTransaction {
        backup_name,
        operations: operations.clone(),
    };
    atomic_write(
        &transaction,
        &serde_json::to_vec_pretty(&journal).unwrap_or_default(),
    )
    .map_err(|error| GitFailure {
        message: error.message,
        auth: false,
        offline: false,
    })?;
    let applied = apply_remote_operations(root, cache, local, remote, base);
    if let Err(error) = applied {
        if let Some(backup) = backup.as_deref() {
            let _ = restore_backup(root, backup, &operations);
        }
        let _ = fs::remove_file(&transaction);
        return Err(error);
    }
    let _ = fs::remove_file(&transaction);
    Ok(())
}

fn recover_pending_transaction(state: &AppState, root: &Path) -> AppResult<()> {
    let cache_key = cache_key(&path_key(root));
    let transaction = state
        .sync_cache_dir
        .join(&cache_key)
        .join("apply-transaction.json");
    if !transaction.is_file() {
        return Ok(());
    }
    let journal: ApplyTransaction = serde_json::from_slice(
        &fs::read(&transaction).map_err(|error| AppError::io("读取同步恢复日志失败", error))?,
    )
    .map_err(|_| {
        AppError::new(
            "sync_transaction_invalid",
            "同步恢复日志已损坏，请打开备份目录手动恢复。",
            true,
        )
    })?;
    if journal.backup_name.is_empty()
        || journal.backup_name.contains(['/', '\\'])
        || journal.backup_name == "."
        || journal.backup_name == ".."
    {
        return Err(AppError::new(
            "sync_transaction_invalid",
            "同步恢复日志包含无效备份路径。",
            true,
        ));
    }
    let backup = state
        .sync_backup_dir
        .join(cache_key)
        .join(journal.backup_name);
    if !backup.is_dir() {
        return Err(AppError::new(
            "sync_backup_missing",
            "同步事务未完成且恢复备份不存在。",
            true,
        ));
    }
    restore_backup(root, &backup, &journal.operations)?;
    fs::remove_file(transaction).map_err(|error| AppError::io("清理同步恢复日志失败", error))
}

fn apply_remote_operations(
    root: &Path,
    cache: &Path,
    local: &Snapshot,
    remote: &Snapshot,
    base: &BTreeMap<String, String>,
) -> Result<(), GitFailure> {
    for path in remote.keys() {
        let source = cache.join(path);
        let target = root.join(path);
        ensure_safe_apply_target(root, &target)?;
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| GitFailure {
                message: format!("创建同步目录失败：{error}"),
                auth: false,
                offline: false,
            })?;
        }
        fs::copy(source, target).map_err(|error| GitFailure {
            message: format!("应用远端文件失败：{error}"),
            auth: false,
            offline: false,
        })?;
    }
    for path in base.keys() {
        if !remote.contains_key(path)
            && local.get(path).map(|item| item.hash.as_str()) == base.get(path).map(String::as_str)
        {
            let target = root.join(path);
            ensure_safe_apply_target(root, &target)?;
            if target.is_file() {
                fs::remove_file(target).map_err(|error| GitFailure {
                    message: format!("删除远端已删除文件失败：{error}"),
                    auth: false,
                    offline: false,
                })?;
            }
        }
    }
    Ok(())
}

fn ensure_safe_apply_target(root: &Path, target: &Path) -> Result<(), GitFailure> {
    let relative = target.strip_prefix(root).map_err(|_| GitFailure {
        message: "同步目标路径超出了当前项目。".to_string(),
        auth: false,
        offline: false,
    })?;
    let mut cursor = root.to_path_buf();
    for component in relative.components() {
        cursor.push(component);
        match fs::symlink_metadata(&cursor) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(GitFailure {
                    message: "同步目标路径包含符号链接，已拒绝写入。".to_string(),
                    auth: false,
                    offline: false,
                });
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => {
                return Err(GitFailure {
                    message: format!("验证同步目标路径失败：{error}"),
                    auth: false,
                    offline: false,
                });
            }
        }
    }
    Ok(())
}

fn backup_local_files(
    state: &AppState,
    root: &Path,
    local: &Snapshot,
    remote: &Snapshot,
    base: &BTreeMap<String, String>,
) -> AppResult<Option<PathBuf>> {
    let timestamp = Local::now().format("%Y%m%d-%H%M%S-%3f");
    let target_root = state
        .sync_backup_dir
        .join(cache_key(&path_key(root)))
        .join(timestamp.to_string());
    fs::create_dir_all(&target_root)
        .map_err(|error| AppError::io("创建同步备份目录失败", error))?;
    for path in remote.keys().chain(base.keys()) {
        if local.contains_key(path) {
            let source = root.join(path);
            if source.is_file() {
                let target = target_root.join(path);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|error| AppError::io("创建同步备份目录失败", error))?;
                }
                fs::copy(source, target)
                    .map_err(|error| AppError::io("创建同步备份失败", error))?;
            }
        }
    }
    prune_backups(target_root.parent().unwrap_or(&target_root))?;
    Ok(Some(target_root))
}

fn restore_backup(root: &Path, backup: &Path, operations: &BTreeSet<String>) -> AppResult<()> {
    for path in operations {
        let saved = backup.join(path);
        let target = root.join(path);
        if saved.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| AppError::io("恢复同步备份失败", error))?;
            }
            fs::copy(saved, target).map_err(|error| AppError::io("恢复同步备份失败", error))?;
        } else if target.is_file() {
            fs::remove_file(target).map_err(|error| AppError::io("回滚新增同步文件失败", error))?;
        }
    }
    Ok(())
}

fn prune_backups(project_backup_dir: &Path) -> AppResult<()> {
    let mut backups = fs::read_dir(project_backup_dir)
        .map_err(|error| AppError::io("读取同步备份目录失败", error))?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .collect::<Vec<_>>();
    backups.sort_by_key(|entry| entry.file_name());
    let remove_count = backups.len().saturating_sub(MAX_BACKUPS);
    for entry in backups.into_iter().take(remove_count) {
        fs::remove_dir_all(entry.path())
            .map_err(|error| AppError::io("清理旧同步备份失败", error))?;
    }
    Ok(())
}

fn conflict_view(
    root: &Path,
    cache: &Path,
    path: &str,
    local: &Snapshot,
    remote: &Snapshot,
) -> AppResult<ContentSyncConflict> {
    let markdown = Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md"));
    let local_path = root.join(path);
    let remote_path = cache.join(path);
    let local_size = fs::metadata(&local_path).ok().map(|value| value.len());
    let remote_size = fs::metadata(&remote_path).ok().map(|value| value.len());
    Ok(ContentSyncConflict {
        path: path.to_string(),
        kind: if markdown { "markdown" } else { "binary" }.to_string(),
        local_hash: local.get(path).map(|item| item.hash.clone()),
        remote_hash: remote.get(path).map(|item| item.hash.clone()),
        local_size,
        remote_size,
        local_text: if markdown && local_path.is_file() {
            fs::read_to_string(local_path).ok()
        } else {
            None
        },
        remote_text: if markdown && remote_path.is_file() {
            fs::read_to_string(remote_path).ok()
        } else {
            None
        },
    })
}

fn apply_conflict_choices(
    state: &AppState,
    root: &Path,
    cache: &Path,
    record: &SyncRecord,
    local: &Snapshot,
    remote: &Snapshot,
    choices: &BTreeMap<String, String>,
) -> AppResult<()> {
    let paths = record
        .base_files
        .keys()
        .chain(local.keys())
        .chain(remote.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let affected_remote = paths
        .into_iter()
        .filter(|path| {
            let local_hash = local.get(path).map(|item| item.hash.as_str());
            let remote_hash = remote.get(path).map(|item| item.hash.as_str());
            let base_hash = record.base_files.get(path).map(String::as_str);
            let local_changed = local_hash != base_hash;
            let remote_changed = remote_hash != base_hash;
            remote_changed
                && (!local_changed
                    || (local_hash != remote_hash
                        && choices.get(path).is_some_and(|choice| choice == "remote")))
        })
        .collect::<BTreeSet<_>>();
    if affected_remote.is_empty() {
        return Ok(());
    }
    let mut selected_remote = Snapshot::new();
    for path in &affected_remote {
        if let Some(item) = remote.get(path) {
            selected_remote.insert(path.clone(), item.clone());
        }
    }
    let selected_base = affected_remote
        .iter()
        .map(|path| {
            let local_hash = local.get(path).map(|item| item.hash.as_str());
            let base_hash = record.base_files.get(path).map(String::as_str);
            (
                path.clone(),
                if local_hash == base_hash {
                    record.base_files.get(path).cloned().unwrap_or_default()
                } else {
                    local
                        .get(path)
                        .map(|item| item.hash.clone())
                        .unwrap_or_default()
                },
            )
        })
        .collect();
    apply_remote(state, root, cache, local, &selected_remote, &selected_base)
        .map_err(git_failure_error)
}

fn local_snapshot(root: &Path, image_dir: &str) -> AppResult<Snapshot> {
    validate_sync_image_dir(image_dir)?;
    let mut paths = BTreeSet::new();
    let posts_root = root.join("source/_posts");
    if posts_root.exists() {
        for entry in WalkDir::new(&posts_root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if entry.file_type().is_symlink()
                || !entry.file_type().is_file()
                || !entry
                    .path()
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
            {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(root)
                .map_err(|_| AppError::new("sync_path_escape", "文章不属于当前项目。", false))?
                .to_string_lossy()
                .replace('\\', "/");
            paths.insert(relative);
        }
    }
    collect_scope(
        root,
        &root.join(image_dir.replace('/', std::path::MAIN_SEPARATOR_STR)),
        &mut paths,
    )?;
    for post in paths
        .clone()
        .into_iter()
        .filter(|path| path.starts_with("source/_posts/") && path.ends_with(".md"))
    {
        let stem = Path::new(&post).with_extension("");
        collect_scope(root, &root.join(stem), &mut paths)?;
    }
    snapshot_paths(root, paths)
}

fn validate_sync_image_dir(image_dir: &str) -> AppResult<()> {
    let normalized = image_dir.trim().replace('\\', "/");
    if matches!(normalized.as_str(), "source/_posts" | "source/_drafts")
        || normalized.starts_with("source/_posts/")
        || normalized.starts_with("source/_drafts/")
    {
        return Err(AppError::new(
            "sync_image_scope_invalid",
            "同步图片目录不能位于文章或草稿目录内。",
            true,
        ));
    }
    Ok(())
}

fn collect_scope(root: &Path, directory: &Path, paths: &mut BTreeSet<String>) -> AppResult<()> {
    if !directory.exists() {
        return Ok(());
    }
    for entry in WalkDir::new(directory)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_symlink() || !entry.file_type().is_file() {
            continue;
        }
        let metadata =
            fs::metadata(entry.path()).map_err(|error| AppError::io("读取同步文件失败", error))?;
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| AppError::new("sync_path_escape", "同步文件不属于当前项目。", false))?
            .to_string_lossy()
            .replace('\\', "/");
        if metadata.len() > MAX_SYNC_FILE_BYTES {
            return Err(AppError::new(
                "sync_file_too_large",
                format!("同步文件超过 GitHub 单文件限制：{relative}"),
                true,
            ));
        }
        if relative != MANIFEST && !is_excluded_sync_path(&relative) {
            paths.insert(relative);
        }
    }
    Ok(())
}

fn snapshot_paths(root: &Path, paths: BTreeSet<String>) -> AppResult<Snapshot> {
    let mut snapshot = Snapshot::new();
    for relative in paths {
        let bytes = fs::read(root.join(&relative))
            .map_err(|error| AppError::io("读取同步文件失败", error))?;
        snapshot.insert(
            relative,
            FileSnapshot {
                hash: hash_bytes(&bytes),
            },
        );
    }
    Ok(snapshot)
}

fn snapshot_from_manifest(cache: &Path, manifest: &SyncManifest) -> AppResult<Snapshot> {
    validate_manifest_paths(&manifest.files, &manifest.image_dir)?;
    let mut paths = BTreeSet::new();
    for path in manifest.files.keys() {
        let candidate = cache.join(path);
        let file_type = fs::symlink_metadata(&candidate)
            .map_err(|_| {
                AppError::new(
                    "sync_manifest_invalid",
                    "远端同步清单引用了缺失文件。",
                    true,
                )
            })?
            .file_type();
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(AppError::new(
                "sync_manifest_invalid",
                "远端同步分支包含符号链接或非普通文件。",
                true,
            ));
        }
        let canonical = candidate.canonicalize().map_err(|_| {
            AppError::new(
                "sync_manifest_invalid",
                "远端同步清单引用了缺失文件。",
                true,
            )
        })?;
        let canonical_cache = cache
            .canonicalize()
            .map_err(|error| AppError::io("验证同步缓存失败", error))?;
        if !canonical.starts_with(&canonical_cache) || !canonical.is_file() {
            return Err(AppError::new(
                "sync_manifest_invalid",
                "远端同步清单包含无效路径。",
                true,
            ));
        }
        paths.insert(path.clone());
    }
    let snapshot = snapshot_paths(cache, paths)?;
    if snapshot
        .iter()
        .any(|(path, item)| manifest.files.get(path) != Some(&item.hash))
    {
        return Err(AppError::new(
            "sync_manifest_invalid",
            "远端同步清单哈希不匹配。",
            true,
        ));
    }
    Ok(snapshot)
}

fn validate_manifest_paths(files: &BTreeMap<String, String>, image_dir: &str) -> AppResult<()> {
    let article_resource_prefixes = files
        .keys()
        .filter(|path| {
            path.starts_with("source/_posts/") && path.to_ascii_lowercase().ends_with(".md")
        })
        .map(|path| format!("{}/", path[..path.len() - 3].trim_end_matches('.')))
        .collect::<Vec<_>>();
    for (path, hash) in files {
        validate_manifest_path_shape(path)?;
        if hash.len() != 64 || !hash.bytes().all(|value| value.is_ascii_hexdigit()) {
            return Err(AppError::new(
                "sync_manifest_invalid",
                "远端同步清单包含无效文件哈希。",
                true,
            ));
        }
        if is_excluded_sync_path(path) {
            return Err(AppError::new(
                "sync_manifest_scope",
                "远端同步清单包含备份、隐藏目录或敏感文件。",
                true,
            ));
        }
        let image_prefix = image_dir.trim_matches('/');
        let is_article =
            path.starts_with("source/_posts/") && path.to_ascii_lowercase().ends_with(".md");
        let is_article_resource = article_resource_prefixes
            .iter()
            .any(|prefix| path.starts_with(prefix));
        let is_configured_image = !image_prefix.is_empty()
            && (path == image_prefix || path.starts_with(&format!("{image_prefix}/")));
        if !is_article && !is_article_resource && !is_configured_image {
            return Err(AppError::new(
                "sync_manifest_scope",
                "远端同步清单包含文章、文章资源或图片目录以外的文件。",
                true,
            ));
        }
    }
    Ok(())
}

fn validate_manifest_path_shape(path: &str) -> AppResult<()> {
    let relative = Path::new(path);
    if path.is_empty()
        || path.contains('\\')
        || relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(AppError::new(
            "sync_manifest_invalid",
            "远端同步清单包含危险路径。",
            true,
        ));
    }
    Ok(())
}

fn is_excluded_sync_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    let name = lower.rsplit('/').next().unwrap_or_default();
    let hidden_component = lower
        .split('/')
        .skip(1)
        .any(|component| component.starts_with('.'));
    hidden_component
        || name.starts_with(".env")
        || matches!(
            name,
            "id_rsa" | "id_ed25519" | "credentials" | "credentials.json"
        )
        || name.ends_with('~')
        || [
            ".bak", ".backup", ".old", ".orig", ".tmp", ".swp", ".key", ".pem", ".p12", ".pfx",
        ]
        .iter()
        .any(|suffix| name.ends_with(suffix))
}

fn copy_snapshot_to_cache(
    root: &Path,
    cache: &Path,
    snapshot: &Snapshot,
    image_dir: &str,
) -> Result<(), GitFailure> {
    let _ = image_dir;
    let _ = git_output(cache, &["rm", "-rf", "--ignore-unmatch", "."], false)?;
    git(cache, &["clean", "-fdx"], false)?;
    for path in snapshot.keys() {
        let target = cache.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|error| GitFailure {
                message: format!("创建缓存目录失败：{error}"),
                auth: false,
                offline: false,
            })?;
        }
        fs::copy(root.join(path), target).map_err(|error| GitFailure {
            message: format!("写入同步缓存失败：{error}"),
            auth: false,
            offline: false,
        })?;
    }
    Ok(())
}

fn copy_snapshot_to_plain_cache(
    root: &Path,
    cache: &Path,
    snapshot: &Snapshot,
) -> Result<(), GitFailure> {
    clear_plain_cache(cache)?;
    for path in snapshot.keys() {
        let target = cache.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(local_sync_failure("创建 WebDAV 缓存目录失败"))?;
        }
        fs::copy(root.join(path), target)
            .map_err(local_sync_failure("写入 WebDAV 同步缓存失败"))?;
    }
    Ok(())
}

fn clear_plain_cache(cache: &Path) -> Result<(), GitFailure> {
    if cache.exists() {
        for entry in fs::read_dir(cache).map_err(local_sync_failure("读取同步缓存失败"))? {
            let entry = entry.map_err(local_sync_failure("读取同步缓存失败"))?;
            let kind = entry
                .file_type()
                .map_err(local_sync_failure("读取同步缓存失败"))?;
            if kind.is_dir() {
                fs::remove_dir_all(entry.path()).map_err(local_sync_failure("清理同步缓存失败"))?;
            } else {
                fs::remove_file(entry.path()).map_err(local_sync_failure("清理同步缓存失败"))?;
            }
        }
    } else {
        fs::create_dir_all(cache).map_err(local_sync_failure("创建同步缓存失败"))?;
    }
    Ok(())
}

fn local_sync_failure(context: &'static str) -> impl FnOnce(std::io::Error) -> GitFailure {
    move |error| GitFailure {
        message: format!("{context}：{error}"),
        auth: false,
        offline: false,
    }
}

fn write_manifest(cache: &Path, manifest: &SyncManifest) -> Result<(), GitFailure> {
    let bytes = serde_json::to_vec_pretty(manifest).map_err(|error| GitFailure {
        message: error.to_string(),
        auth: false,
        offline: false,
    })?;
    atomic_write(&cache.join(MANIFEST), &bytes).map_err(|error| GitFailure {
        message: error.message,
        auth: false,
        offline: false,
    })
}

fn commit_and_push(cache: &Path, branch: &str) -> Result<(), GitFailure> {
    git(cache, &["add", "-A"], false)?;
    let status = git_output(cache, &["diff", "--cached", "--quiet"], false)?;
    if status.status.success() {
        return Ok(());
    }
    git(
        cache,
        &[
            "-c",
            "user.name=Hexo Lite Editor Sync",
            "-c",
            "user.email=hexo-lite-editor-sync@localhost",
            "commit",
            "-m",
            "Sync Hexo content",
        ],
        false,
    )?;
    let refspec = format!("HEAD:refs/heads/{branch}");
    git(cache, &["push", "origin", &refspec], false)?;
    Ok(())
}

fn prepare_orphan_cache(cache: &Path) -> Result<(), GitFailure> {
    let name = format!("hlex-sync-{}", uuid::Uuid::new_v4());
    git(cache, &["checkout", "--orphan", &name], false)?;
    let _ = git_output(cache, &["rm", "-rf", "--ignore-unmatch", "."], false)?;
    git(cache, &["clean", "-fdx"], false)?;
    Ok(())
}

fn ensure_cache(cache: &Path, repository: &str) -> Result<(), GitFailure> {
    fs::create_dir_all(cache).map_err(|error| GitFailure {
        message: format!("创建同步缓存失败：{error}"),
        auth: false,
        offline: false,
    })?;
    if !cache.join(".git").is_dir() {
        git(cache, &["init"], false)?;
    }
    let remote = git_output(cache, &["remote", "get-url", "origin"], false)?;
    if !remote.status.success() {
        git(cache, &["remote", "add", "origin", repository], false)?;
    } else if String::from_utf8_lossy(&remote.stdout).trim() != repository {
        git(cache, &["remote", "set-url", "origin", repository], false)?;
    }
    Ok(())
}

fn prepare_remote_cache(cache: &Path, record: &SyncRecord) -> Result<(), GitFailure> {
    match record.provider {
        ContentSyncProvider::Github => ensure_cache(cache, &record.repository),
        ContentSyncProvider::Webdav => {
            fs::create_dir_all(cache).map_err(local_sync_failure("创建 WebDAV 同步缓存失败"))
        }
    }
}

fn fetch_record_remote(
    cache: &Path,
    record: &SyncRecord,
    timeout: Duration,
) -> Result<RemoteFetch, GitFailure> {
    match record.provider {
        ContentSyncProvider::Github => {
            let exists = fetch_remote_branch_with_timeout(cache, &record.branch, timeout)?;
            if !exists {
                return Ok(RemoteFetch {
                    exists: false,
                    head: None,
                    etag: None,
                });
            }
            checkout_fetched_branch(cache)?;
            Ok(RemoteFetch {
                exists: true,
                head: git(cache, &["rev-parse", "HEAD"], false).ok(),
                etag: None,
            })
        }
        ContentSyncProvider::Webdav => {
            let credentials = webdav_credentials(&record.repository).map_err(|_| GitFailure {
                message: "WebDAV 凭据缺失，请在内容同步设置中重新保存用户名和密码。".to_string(),
                auth: true,
                offline: false,
            })?;
            fetch_webdav_remote(
                cache,
                &record.repository,
                &record.branch,
                &credentials,
                timeout,
            )
        }
    }
}

fn webdav_client(timeout: Duration) -> Result<reqwest::blocking::Client, GitFailure> {
    reqwest::blocking::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::none())
        .user_agent("Hexo-Lite-Editor/1.0.5 WebDAV-Sync")
        .build()
        .map_err(|error| GitFailure {
            message: format!("无法初始化 WebDAV 客户端：{error}"),
            auth: false,
            offline: true,
        })
}

fn fetch_webdav_remote(
    cache: &Path,
    endpoint: &str,
    remote_dir: &str,
    credentials: &crate::platform::WebDavCredentials,
    timeout: Duration,
) -> Result<RemoteFetch, GitFailure> {
    clear_plain_cache(cache)?;
    let client = webdav_client(timeout)?;
    let manifest_url = webdav_resource_url(endpoint, remote_dir, Some(MANIFEST))?;
    let response = client
        .get(manifest_url)
        .basic_auth(&credentials.username, Some(&credentials.password))
        .send()
        .map_err(webdav_transport_failure)?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        let has_content =
            webdav_collection_has_content(&client, endpoint, remote_dir, credentials)?;
        return Ok(RemoteFetch {
            exists: has_content,
            head: None,
            etag: None,
        });
    }
    if !response.status().is_success() {
        return Err(webdav_status_failure(
            response.status(),
            "读取 WebDAV 同步清单失败",
        ));
    }
    let etag = response
        .headers()
        .get(reqwest::header::ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let manifest_bytes = read_webdav_body(response, 5 * 1024 * 1024, "WebDAV 同步清单")?;
    let manifest: SyncManifest =
        serde_json::from_slice(&manifest_bytes).map_err(|_| GitFailure {
            message: "WebDAV 远端同步清单不是有效的 JSON。".to_string(),
            auth: false,
            offline: false,
        })?;
    validate_manifest_paths(&manifest.files, &manifest.image_dir).map_err(|error| GitFailure {
        message: error.message,
        auth: false,
        offline: false,
    })?;
    for (path, hash) in &manifest.files {
        let object_path = format!("{WEBDAV_OBJECTS}/{hash}");
        let file_url = webdav_resource_url(endpoint, remote_dir, Some(&object_path))?;
        let response = client
            .get(file_url)
            .basic_auth(&credentials.username, Some(&credentials.password))
            .send()
            .map_err(webdav_transport_failure)?;
        if !response.status().is_success() {
            return Err(webdav_status_failure(
                response.status(),
                &format!("读取 WebDAV 文件失败：{path}"),
            ));
        }
        let bytes = read_webdav_body(response, MAX_SYNC_FILE_BYTES, path)?;
        let target = cache.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(local_sync_failure("创建 WebDAV 缓存目录失败"))?;
        }
        atomic_write(&target, &bytes).map_err(|error| GitFailure {
            message: error.message,
            auth: false,
            offline: false,
        })?;
    }
    atomic_write(&cache.join(MANIFEST), &manifest_bytes).map_err(|error| GitFailure {
        message: error.message,
        auth: false,
        offline: false,
    })?;
    Ok(RemoteFetch {
        exists: true,
        head: Some(hash_bytes(&manifest_bytes)),
        etag,
    })
}

fn webdav_collection_has_content(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    remote_dir: &str,
    credentials: &crate::platform::WebDavCredentials,
) -> Result<bool, GitFailure> {
    let method = reqwest::Method::from_bytes(b"PROPFIND").expect("valid WebDAV method");
    let url = webdav_resource_url(endpoint, remote_dir, None)?;
    let response = client
        .request(method, url.clone())
        .header("Depth", "1")
        .basic_auth(&credentials.username, Some(&credentials.password))
        .send()
        .map_err(webdav_transport_failure)?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(false);
    }
    if response.status().is_success() || response.status() == reqwest::StatusCode::MULTI_STATUS {
        let body = response.text().map_err(webdav_transport_failure)?;
        return Ok(webdav_listing_has_user_content(&body, &url));
    }
    Err(webdav_status_failure(
        response.status(),
        "检查 WebDAV 远端目录失败",
    ))
}

fn test_webdav_connection(
    endpoint: &str,
    remote_dir: &str,
    credentials: &crate::platform::WebDavCredentials,
) -> AppResult<()> {
    let client = webdav_client(WEBDAV_TIMEOUT).map_err(git_failure_error)?;
    let collection_method = reqwest::Method::from_bytes(b"PROPFIND").expect("valid WebDAV method");
    let mut collection_url =
        url::Url::parse(endpoint).map_err(|_| AppError::invalid("WebDAV 服务器地址无效。"))?;
    let collection_path = format!("{}/", collection_url.path().trim_end_matches('/'));
    collection_url.set_path(&collection_path);
    let collection_response = client
        .request(collection_method, collection_url)
        .header("Depth", "0")
        .basic_auth(&credentials.username, Some(&credentials.password))
        .send()
        .map_err(webdav_transport_failure)
        .map_err(git_failure_error)?;
    if !collection_response.status().is_success()
        && collection_response.status() != reqwest::StatusCode::MULTI_STATUS
    {
        return Err(git_failure_error(webdav_status_failure(
            collection_response.status(),
            "WebDAV PROPFIND 探针失败",
        )));
    }
    ensure_webdav_collections(&client, endpoint, remote_dir, None, credentials)
        .map_err(webdav_probe_error)?;

    let probe_name = format!(".hexo-lite-probe-{}", Uuid::new_v4());
    let probe_url =
        webdav_resource_url(endpoint, remote_dir, Some(&probe_name)).map_err(git_failure_error)?;
    let probe_bytes = format!("hexo-lite-editor:{}", Uuid::new_v4()).into_bytes();
    let put_response = client
        .put(probe_url.clone())
        .basic_auth(&credentials.username, Some(&credentials.password))
        .header(reqwest::header::IF_NONE_MATCH, "*")
        .header(reqwest::header::CONTENT_TYPE, "text/plain")
        .body(probe_bytes.clone())
        .send()
        .map_err(webdav_transport_failure)
        .map_err(git_failure_error)?;
    if !put_response.status().is_success() {
        return Err(webdav_probe_error(webdav_status_failure(
            put_response.status(),
            "WebDAV 写入探针失败",
        )));
    }

    let probe_result = (|| -> AppResult<()> {
        let get_response = client
            .get(probe_url.clone())
            .basic_auth(&credentials.username, Some(&credentials.password))
            .send()
            .map_err(webdav_transport_failure)
            .map_err(git_failure_error)?;
        if !get_response.status().is_success() {
            return Err(git_failure_error(webdav_status_failure(
                get_response.status(),
                "WebDAV 读取探针失败",
            )));
        }
        let received =
            read_webdav_body(get_response, 1024, "WebDAV 连接探针").map_err(git_failure_error)?;
        if received != probe_bytes {
            return Err(AppError::new(
                "webdav_probe_mismatch",
                "WebDAV 服务器返回的探针内容与写入内容不一致。",
                true,
            ));
        }
        Ok(())
    })();

    let delete_response = client
        .delete(probe_url)
        .basic_auth(&credentials.username, Some(&credentials.password))
        .send()
        .map_err(|error| {
            AppError::new(
                "webdav_probe_cleanup_failed",
                format!(
                    "WebDAV 连接探针清理失败：{}",
                    webdav_transport_failure(error).message
                ),
                true,
            )
        })?;
    if !delete_response.status().is_success()
        && delete_response.status() != reqwest::StatusCode::NOT_FOUND
    {
        return Err(AppError::new(
            "webdav_probe_cleanup_failed",
            format!(
                "WebDAV 连接探针无法删除（HTTP {}），请检查删除权限。",
                delete_response.status().as_u16()
            ),
            true,
        ));
    }
    probe_result
}

fn webdav_probe_error(error: GitFailure) -> AppError {
    if error.auth || error.offline {
        return git_failure_error(error);
    }
    AppError::new(
        "webdav_write_required",
        format!(
            "{} 请确认账号拥有创建目录、上传、下载和删除权限。",
            error.message
        ),
        true,
    )
}

fn webdav_listing_has_user_content(body: &str, collection_url: &url::Url) -> bool {
    let collection_path = collection_url.path().trim_end_matches('/');
    let objects_path = format!("{collection_path}/{WEBDAV_OBJECTS}");
    let href_pattern = regex::Regex::new(
        r"(?is)<(?:[A-Za-z_][A-Za-z0-9_.-]*:)?href(?:\s[^>]*)?>(.*?)</(?:[A-Za-z_][A-Za-z0-9_.-]*:)?href>",
    )
    .expect("valid WebDAV href pattern");
    let has_content = href_pattern.captures_iter(body).any(|capture| {
        let href = capture.get(1).map_or("", |value| value.as_str()).trim();
        let path = url::Url::parse(href)
            .ok()
            .map(|value| value.path().to_string())
            .or_else(|| {
                collection_url
                    .join(href)
                    .ok()
                    .map(|value| value.path().to_string())
            })
            .unwrap_or_else(|| href.to_string());
        let path = percent_encoding::percent_decode_str(&path)
            .decode_utf8_lossy()
            .trim_end_matches('/')
            .to_string();
        path != collection_path && path != objects_path
    });
    has_content
}

fn publish_webdav_remote(
    cache: &Path,
    endpoint: &str,
    remote_dir: &str,
    manifest: &SyncManifest,
    remote_existed: bool,
    previous_etag: Option<&str>,
) -> Result<(), GitFailure> {
    let credentials = webdav_credentials(endpoint).map_err(|_| GitFailure {
        message: "WebDAV 凭据缺失，请重新保存用户名和密码。".to_string(),
        auth: true,
        offline: false,
    })?;
    publish_webdav_remote_with_credentials(
        cache,
        endpoint,
        remote_dir,
        manifest,
        remote_existed,
        previous_etag,
        &credentials,
    )
}

fn publish_webdav_remote_with_credentials(
    cache: &Path,
    endpoint: &str,
    remote_dir: &str,
    manifest: &SyncManifest,
    remote_existed: bool,
    previous_etag: Option<&str>,
    credentials: &crate::platform::WebDavCredentials,
) -> Result<(), GitFailure> {
    if remote_existed && previous_etag.is_none() {
        return Err(GitFailure {
            message: "WebDAV 服务器没有为同步清单提供 ETag，无法安全覆盖远端内容。".to_string(),
            auth: false,
            offline: false,
        });
    }
    let client = webdav_client(WEBDAV_TIMEOUT)?;
    ensure_webdav_collections(&client, endpoint, remote_dir, None, credentials)?;
    if !manifest.files.is_empty() {
        ensure_webdav_collections(
            &client,
            endpoint,
            remote_dir,
            Some(Path::new(WEBDAV_OBJECTS)),
            credentials,
        )?;
        for (path, hash) in &manifest.files {
            let bytes =
                fs::read(cache.join(path)).map_err(local_sync_failure("读取待上传文件失败"))?;
            let object_path = format!("{WEBDAV_OBJECTS}/{hash}");
            let response = client
                .put(webdav_resource_url(
                    endpoint,
                    remote_dir,
                    Some(&object_path),
                )?)
                .basic_auth(&credentials.username, Some(&credentials.password))
                .header(reqwest::header::IF_NONE_MATCH, "*")
                .body(bytes)
                .send()
                .map_err(webdav_transport_failure)?;
            if !response.status().is_success()
                && response.status() != reqwest::StatusCode::PRECONDITION_FAILED
            {
                return Err(webdav_status_failure(
                    response.status(),
                    &format!("上传 WebDAV 文件失败：{path}"),
                ));
            }
        }
    }
    let manifest_bytes =
        fs::read(cache.join(MANIFEST)).map_err(local_sync_failure("读取待上传同步清单失败"))?;
    let mut request = client
        .put(webdav_resource_url(endpoint, remote_dir, Some(MANIFEST))?)
        .basic_auth(&credentials.username, Some(&credentials.password))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(manifest_bytes);
    request = if let Some(etag) = previous_etag {
        request.header(reqwest::header::IF_MATCH, etag)
    } else if !remote_existed {
        request.header(reqwest::header::IF_NONE_MATCH, "*")
    } else {
        request
    };
    let response = request.send().map_err(webdav_transport_failure)?;
    if response.status() == reqwest::StatusCode::PRECONDITION_FAILED {
        return Err(GitFailure {
            message: REMOTE_AHEAD_MESSAGE.to_string(),
            auth: false,
            offline: false,
        });
    }
    if !response.status().is_success() {
        return Err(webdav_status_failure(
            response.status(),
            "上传 WebDAV 同步清单失败",
        ));
    }
    Ok(())
}

fn ensure_webdav_collections(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    remote_dir: &str,
    parent: Option<&Path>,
    credentials: &crate::platform::WebDavCredentials,
) -> Result<(), GitFailure> {
    let mut segments = remote_dir
        .split('/')
        .map(str::to_string)
        .collect::<Vec<_>>();
    if let Some(parent) = parent {
        segments.extend(parent.components().filter_map(|part| match part {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        }));
    }
    let method = reqwest::Method::from_bytes(b"MKCOL").expect("valid WebDAV method");
    for length in 1..=segments.len() {
        let path = segments[..length].join("/");
        let response = client
            .request(method.clone(), webdav_resource_url(endpoint, &path, None)?)
            .basic_auth(&credentials.username, Some(&credentials.password))
            .send()
            .map_err(webdav_transport_failure)?;
        if response.status().is_success()
            || response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED
        {
            continue;
        }
        return Err(webdav_status_failure(
            response.status(),
            &format!("创建 WebDAV 目录失败：{path}"),
        ));
    }
    Ok(())
}

fn read_webdav_body(
    response: reqwest::blocking::Response,
    limit: u64,
    name: &str,
) -> Result<Vec<u8>, GitFailure> {
    if response
        .content_length()
        .is_some_and(|length| length > limit)
    {
        return Err(GitFailure {
            message: format!("WebDAV 文件超过大小限制：{name}"),
            auth: false,
            offline: false,
        });
    }
    let mut bytes = Vec::new();
    response
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| GitFailure {
            message: format!("读取 WebDAV 响应失败：{error}"),
            auth: false,
            offline: true,
        })?;
    if bytes.len() as u64 > limit {
        return Err(GitFailure {
            message: format!("WebDAV 文件超过大小限制：{name}"),
            auth: false,
            offline: false,
        });
    }
    Ok(bytes)
}

fn webdav_transport_failure(error: reqwest::Error) -> GitFailure {
    GitFailure {
        message: if error.is_timeout() {
            "WebDAV 连接超时，请检查服务器地址和网络。".to_string()
        } else {
            format!("无法连接 WebDAV 服务器：{error}")
        },
        auth: false,
        offline: true,
    }
}

fn webdav_status_failure(status: reqwest::StatusCode, context: &str) -> GitFailure {
    let auth = status == reqwest::StatusCode::UNAUTHORIZED;
    GitFailure {
        message: if auth {
            "WebDAV 认证失败，请检查用户名和密码。".to_string()
        } else if status == reqwest::StatusCode::FORBIDDEN {
            format!("WebDAV 权限不足：{context}（HTTP 403）。")
        } else {
            format!("{context}（HTTP {}）。", status.as_u16())
        },
        auth,
        offline: false,
    }
}

fn fetch_remote_branch(cache: &Path, branch: &str) -> Result<bool, GitFailure> {
    fetch_remote_branch_with_timeout(cache, branch, GIT_TIMEOUT)
}

fn fetch_remote_branch_with_timeout(
    cache: &Path,
    branch: &str,
    timeout: Duration,
) -> Result<bool, GitFailure> {
    let refspec = format!("refs/heads/{branch}:refs/remotes/origin/{branch}");
    let output = git_output_with_timeout(
        cache,
        &["fetch", "--depth", "1", "origin", &refspec],
        false,
        timeout,
    )?;
    if output.status.success() {
        return Ok(true);
    }
    let text = output_text(&output);
    if text.contains("couldn't find remote ref") || text.contains("did not match any") {
        return Ok(false);
    }
    Err(classify_git_failure(text))
}

fn checkout_fetched_branch(cache: &Path) -> Result<(), GitFailure> {
    git(cache, &["checkout", "--detach", "FETCH_HEAD"], false).map(|_| ())
}

fn read_manifest(cache: &Path) -> Option<SyncManifest> {
    serde_json::from_slice(&fs::read(cache.join(MANIFEST)).ok()?).ok()
}

fn git(root: &Path, args: &[&str], interactive: bool) -> Result<String, GitFailure> {
    let output = git_output(root, args, interactive)?;
    if !output.status.success() {
        return Err(classify_git_failure(output_text(&output)));
    }
    Ok(output_text(&output))
}

fn git_output(root: &Path, args: &[&str], interactive: bool) -> Result<Output, GitFailure> {
    git_output_with_timeout(root, args, interactive, GIT_TIMEOUT)
}

fn git_output_with_timeout(
    root: &Path,
    args: &[&str],
    interactive: bool,
    timeout: Duration,
) -> Result<Output, GitFailure> {
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    if !interactive {
        command
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "Never");
    } else {
        command
            .env("GIT_TERMINAL_PROMPT", "1")
            .env("GCM_INTERACTIVE", "Always");
    }
    let mut child = command.spawn().map_err(|error| GitFailure {
        message: format!("无法执行 Git：{error}"),
        auth: false,
        offline: false,
    })?;
    let stdout = child.stdout.take().map(|mut stream| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stream.read_to_end(&mut bytes);
            bytes
        })
    });
    let stderr = child.stderr.take().map(|mut stream| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = stream.read_to_end(&mut bytes);
            bytes
        })
    });
    match child.wait_timeout(timeout) {
        Ok(Some(status)) => Ok(Output {
            status,
            stdout: stdout
                .and_then(|reader| reader.join().ok())
                .unwrap_or_default(),
            stderr: stderr
                .and_then(|reader| reader.join().ok())
                .unwrap_or_default(),
        }),
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(GitFailure {
                message: "GitHub 同步超时，本地内容保持不变。".to_string(),
                auth: false,
                offline: true,
            })
        }
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(GitFailure {
                message: format!("等待 Git 结束失败：{error}"),
                auth: false,
                offline: false,
            })
        }
    }
}

fn classify_git_failure(message: String) -> GitFailure {
    let lower = message.to_ascii_lowercase();
    let remote_ahead = lower.contains("non-fast-forward")
        || lower.contains("fetch first")
        || lower.contains("failed to push some refs");
    GitFailure {
        auth: lower.contains("authentication")
            || lower.contains("permission denied")
            || lower.contains("could not read username")
            || lower.contains("denied"),
        offline: lower.contains("could not resolve")
            || lower.contains("network")
            || lower.contains("timed out")
            || lower.contains("unable to access"),
        message: if remote_ahead {
            REMOTE_AHEAD_MESSAGE.to_string()
        } else {
            "GitHub 同步未完成，请检查网络、仓库权限或系统 Git 认证。".to_string()
        },
    }
}

fn git_failure_error(error: GitFailure) -> AppError {
    let code = if error.message == REMOTE_AHEAD_MESSAGE {
        "sync_remote_ahead"
    } else if error.auth {
        "sync_auth_required"
    } else if error.offline {
        "sync_offline"
    } else if error.message.contains("WebDAV 权限不足") {
        "sync_permission_denied"
    } else if error.message.contains("同步清单") {
        "sync_manifest_invalid"
    } else {
        "sync_git_error"
    };
    AppError::new(code, error.message, true)
}

fn update_git_error(
    registry: &mut SyncRegistry,
    state: &AppState,
    key: &str,
    error: GitFailure,
) -> ContentSyncView {
    if let Some(record) = registry
        .records
        .iter_mut()
        .find(|item| item.project_path == key)
    {
        record.status = if error.message == REMOTE_AHEAD_MESSAGE {
            ContentSyncStatus::RemoteAhead
        } else if error.auth {
            ContentSyncStatus::AuthRequired
        } else if error.offline {
            ContentSyncStatus::Offline
        } else {
            ContentSyncStatus::Error
        };
        record.message = Some(error.message);
        let view = view_from_record(record);
        let _ = save_registry(state, registry);
        return view;
    }
    error_view(error.message)
}

fn update_error(
    registry: &mut SyncRegistry,
    state: &AppState,
    key: &str,
    message: String,
) -> ContentSyncView {
    if let Some(record) = registry
        .records
        .iter_mut()
        .find(|item| item.project_path == key)
    {
        record.status = ContentSyncStatus::Error;
        record.message = Some(message);
        let view = view_from_record(record);
        let _ = save_registry(state, registry);
        return view;
    }
    error_view(message)
}

fn detect_candidates(root: &Path) -> Vec<ContentSyncCandidate> {
    let mut candidates = Vec::new();
    if let Ok(config) = fs::read_to_string(root.join("_config.yml")) {
        if let Ok(yaml) = serde_yaml::from_str::<YamlValue>(&config) {
            if let Some(deploy) = yaml.get("deploy") {
                match deploy {
                    YamlValue::Mapping(mapping) => push_deploy_candidate(mapping, &mut candidates),
                    YamlValue::Sequence(items) => {
                        for item in items {
                            if let Some(mapping) = item.as_mapping() {
                                push_deploy_candidate(mapping, &mut candidates);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let mut unique = BTreeSet::new();
    candidates.retain(|candidate| unique.insert(candidate.repository.clone()));
    if candidates.is_empty() {
        if let Ok(remote) = git_output(root, &["remote", "get-url", "origin"], false) {
            if remote.status.success() {
                if let Some(repository) =
                    normalize_github_remote(&String::from_utf8_lossy(&remote.stdout))
                {
                    candidates.push(ContentSyncCandidate {
                        repository,
                        source: "Git origin".to_string(),
                        pages_branch: None,
                        visibility: "unknown".to_string(),
                        default_branch: None,
                    });
                }
            }
        }
    }
    candidates
}

fn push_deploy_candidate(value: &serde_yaml::Mapping, candidates: &mut Vec<ContentSyncCandidate>) {
    let type_value = value
        .get(YamlValue::String("type".to_string()))
        .and_then(YamlValue::as_str)
        .unwrap_or_default();
    if !type_value.eq_ignore_ascii_case("git") {
        return;
    }
    for key in ["repo", "repository"] {
        if let Some(repo) = value
            .get(YamlValue::String(key.to_string()))
            .and_then(YamlValue::as_str)
            .and_then(normalize_github_remote)
        {
            let branch = value
                .get(YamlValue::String("branch".to_string()))
                .and_then(YamlValue::as_str)
                .map(str::to_string);
            candidates.push(ContentSyncCandidate {
                repository: repo,
                source: "Hexo deploy 配置".to_string(),
                pages_branch: branch,
                visibility: "unknown".to_string(),
                default_branch: None,
            });
        }
    }
}

fn normalize_github_remote(raw: &str) -> Option<String> {
    let value = raw.trim().trim_end_matches('/');
    let (normalized, ssh) = if let Some(rest) = value.strip_prefix("git@github.com:") {
        (format!("https://github.com/{rest}"), true)
    } else if let Some(rest) = value.strip_prefix("ssh://git@github.com/") {
        (format!("https://github.com/{rest}"), true)
    } else {
        (value.to_string(), false)
    };
    let url = url::Url::parse(&normalized).ok()?;
    if url.scheme() != "https"
        || !url.host_str()?.eq_ignore_ascii_case("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let path = url.path().trim_matches('/').trim_end_matches(".git");
    let parts = path.split('/').collect::<Vec<_>>();
    if parts.len() != 2
        || parts
            .iter()
            .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return None;
    }
    if ssh {
        Some(format!("git@github.com:{}/{}.git", parts[0], parts[1]))
    } else {
        Some(format!("https://github.com/{}/{}.git", parts[0], parts[1]))
    }
}

fn normalize_webdav_endpoint(raw: &str) -> AppResult<String> {
    let mut url =
        url::Url::parse(raw.trim()).map_err(|_| AppError::invalid("WebDAV 服务器地址无效。"))?;
    let local_debug = local_http_webdav_allowed(url.scheme(), url.host_str());
    if (url.scheme() != "https" && !local_debug)
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(AppError::invalid(
            "WebDAV 服务器必须使用不含凭据、查询参数或片段的 HTTPS 地址。",
        ));
    }
    url.set_query(None);
    url.set_fragment(None);
    let path = url.path().trim_end_matches('/').to_string();
    url.set_path(if path.is_empty() { "/" } else { &path });
    Ok(url.to_string().trim_end_matches('/').to_string())
}

fn local_http_webdav_allowed(scheme: &str, host: Option<&str>) -> bool {
    if !cfg!(debug_assertions) || scheme != "http" {
        return false;
    }
    if matches!(host, Some("localhost" | "127.0.0.1" | "[::1]")) {
        return true;
    }
    cfg!(test)
        && std::env::var_os("HLEX_REAL_WEBDAV_TEST_ALLOW_HTTP").is_some_and(|value| value == "1")
}

fn validate_webdav_remote_dir(raw: &str) -> AppResult<String> {
    let value = raw.trim().trim_matches('/').replace('\\', "/");
    if value.is_empty()
        || value.len() > 512
        || value.split('/').any(|segment| {
            segment.is_empty()
                || matches!(segment, "." | "..")
                || segment.chars().any(char::is_control)
        })
    {
        return Err(AppError::invalid(
            "WebDAV 远端目录必须是不含空段或路径穿越的相对路径。",
        ));
    }
    Ok(value)
}

fn webdav_resource_url(
    endpoint: &str,
    remote_dir: &str,
    relative: Option<&str>,
) -> Result<url::Url, GitFailure> {
    let mut url = url::Url::parse(endpoint).map_err(|_| GitFailure {
        message: "WebDAV 服务器地址无效。".to_string(),
        auth: false,
        offline: false,
    })?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| GitFailure {
            message: "WebDAV 服务器地址不能作为目录使用。".to_string(),
            auth: false,
            offline: false,
        })?;
        segments.pop_if_empty();
        for segment in remote_dir.split('/') {
            segments.push(segment);
        }
        if let Some(relative) = relative {
            for segment in relative.split('/') {
                segments.push(segment);
            }
        } else {
            segments.push("");
        }
    }
    Ok(url)
}

fn webdav_cache_dir(state: &AppState, root: &Path) -> PathBuf {
    state
        .sync_cache_dir
        .join(format!("{}-webdav", cache_key(&path_key(root))))
}

fn record_cache_dir(state: &AppState, root: &Path, record: &SyncRecord) -> PathBuf {
    match record.provider {
        ContentSyncProvider::Github => state.sync_cache_dir.join(cache_key(&path_key(root))),
        ContentSyncProvider::Webdav => webdav_cache_dir(state, root),
    }
}

fn snapshot_total_bytes(root: &Path, snapshot: &Snapshot) -> u64 {
    snapshot
        .keys()
        .filter_map(|path| fs::metadata(root.join(path)).ok().map(|value| value.len()))
        .sum()
}

fn snapshot_difference_counts(
    local: &Snapshot,
    remote: Option<&Snapshot>,
) -> (usize, usize, usize) {
    let Some(remote) = remote else {
        return (local.len(), 0, 0);
    };
    let mut local_only = 0;
    let mut remote_only = 0;
    let mut different = 0;
    for path in local.keys().chain(remote.keys()).collect::<BTreeSet<_>>() {
        match (local.get(path), remote.get(path)) {
            (Some(local), Some(remote)) if local.hash != remote.hash => different += 1,
            (Some(_), None) => local_only += 1,
            (None, Some(_)) => remote_only += 1,
            _ => {}
        }
    }
    (local_only, remote_only, different)
}

fn github_owner_repo(repository: &str) -> Option<(String, String)> {
    let path = if let Some(rest) = repository.strip_prefix("git@github.com:") {
        rest.to_string()
    } else {
        let url = url::Url::parse(repository).ok()?;
        if !url.host_str()?.eq_ignore_ascii_case("github.com") {
            return None;
        }
        url.path().trim_matches('/').to_string()
    };
    let path = path.trim_end_matches(".git");
    let (owner, repo) = path.split_once('/')?;
    if owner.is_empty() || repo.is_empty() || repo.contains('/') {
        return None;
    }
    Some((owner.to_string(), repo.to_string()))
}

fn detect_github_visibility(repository: &str) -> String {
    let Some((owner, repo)) = github_owner_repo(repository) else {
        return "unknown".to_string();
    };
    let endpoint = format!("https://api.github.com/repos/{owner}/{repo}");
    let response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .user_agent("Hexo-Lite-Editor/1.0.5")
        .build()
        .and_then(|client| client.get(endpoint).send());
    match response {
        Ok(response) if response.status().is_success() => "public".to_string(),
        _ => "unknown".to_string(),
    }
}

fn validate_branch(value: &str) -> AppResult<String> {
    let branch = value.trim();
    if branch.is_empty()
        || branch.len() > 120
        || branch.starts_with(['-', '.', '/'])
        || branch.ends_with(['.', '/'])
        || branch.ends_with(".lock")
        || branch.contains("..")
        || branch.contains("@{")
        || branch.contains("//")
        || branch.chars().any(|character| {
            character.is_control()
                || character.is_whitespace()
                || matches!(character, '~' | '^' | ':' | '?' | '*' | '[' | '\\')
        })
    {
        return Err(AppError::invalid("内容分支名称无效。"));
    }
    Ok(branch.to_string())
}

fn load_registry(state: &AppState) -> AppResult<SyncRegistry> {
    if !state.sync_registry_path.exists() {
        return Ok(SyncRegistry::default());
    }
    serde_json::from_slice(
        &fs::read(&state.sync_registry_path)
            .map_err(|error| AppError::io("读取内容同步设置失败", error))?,
    )
    .map_err(|error| AppError::new("sync_registry_corrupt", error.to_string(), true))
}

fn save_registry(state: &AppState, registry: &SyncRegistry) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(registry)
        .map_err(|error| AppError::new("sync_registry_serialize", error.to_string(), false))?;
    atomic_write(&state.sync_registry_path, &bytes)
}

fn view_from_record(record: &SyncRecord) -> ContentSyncView {
    ContentSyncView {
        enabled: record.enabled,
        status: record.status.clone(),
        provider: record.provider,
        repository: (record.provider == ContentSyncProvider::Github)
            .then(|| record.repository.clone()),
        branch: (record.provider == ContentSyncProvider::Github).then(|| record.branch.clone()),
        endpoint: (record.provider == ContentSyncProvider::Webdav)
            .then(|| record.repository.clone()),
        remote_dir: (record.provider == ContentSyncProvider::Webdav).then(|| record.branch.clone()),
        visibility: (record.provider == ContentSyncProvider::Github)
            .then(|| record.visibility.clone()),
        message: record.message.clone(),
        conflicts: record.conflicts.clone(),
        last_synced_at: record.last_synced_at.clone(),
    }
}

fn off_view() -> ContentSyncView {
    ContentSyncView {
        enabled: false,
        status: ContentSyncStatus::Off,
        provider: ContentSyncProvider::Github,
        repository: None,
        branch: None,
        endpoint: None,
        remote_dir: None,
        visibility: None,
        message: None,
        conflicts: Vec::new(),
        last_synced_at: None,
    }
}
fn error_view(message: String) -> ContentSyncView {
    ContentSyncView {
        enabled: true,
        status: ContentSyncStatus::Error,
        provider: ContentSyncProvider::Github,
        repository: None,
        branch: None,
        endpoint: None,
        remote_dir: None,
        visibility: None,
        message: Some(message),
        conflicts: Vec::new(),
        last_synced_at: None,
    }
}
fn path_key(path: &Path) -> String {
    path.canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/")
}
fn cache_key(value: &str) -> String {
    hex_hash(value.as_bytes())[..24].to_string()
}
fn hash_bytes(bytes: &[u8]) -> String {
    hex_hash(bytes)
}
fn hex_hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn hash_map(snapshot: &Snapshot) -> BTreeMap<String, String> {
    snapshot
        .iter()
        .map(|(path, item)| (path.clone(), item.hash.clone()))
        .collect()
}
fn output_text(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        stderr.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_webdav_environment() -> Option<(String, String, String)> {
        Some((
            std::env::var("HLEX_REAL_WEBDAV_ENDPOINT").ok()?,
            std::env::var("HLEX_REAL_WEBDAV_USERNAME").ok()?,
            std::env::var("HLEX_REAL_WEBDAV_PASSWORD").ok()?,
        ))
    }

    fn real_webdav_credential_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap()
    }

    struct TestWebDavServer {
        endpoint: String,
        stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    #[derive(Clone, Copy)]
    struct TestWebDavBehavior {
        require_auth: bool,
        forbidden_method: Option<&'static str>,
    }

    impl TestWebDavServer {
        fn start() -> Self {
            Self::start_with_behavior(TestWebDavBehavior {
                require_auth: true,
                forbidden_method: None,
            })
        }

        fn start_with_forbidden_method(method: &'static str) -> Self {
            Self::start_with_behavior(TestWebDavBehavior {
                require_auth: true,
                forbidden_method: Some(method),
            })
        }

        fn start_with_behavior(behavior: TestWebDavBehavior) -> Self {
            use std::sync::{atomic::AtomicBool, Arc, Mutex};
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let address = listener.local_addr().unwrap();
            let stop = Arc::new(AtomicBool::new(false));
            let files = Arc::new(Mutex::new(BTreeMap::<String, Vec<u8>>::new()));
            let stop_for_thread = stop.clone();
            let thread = std::thread::spawn(move || {
                while !stop_for_thread.load(std::sync::atomic::Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let _ = stream.set_nonblocking(false);
                            handle_test_webdav_request(&mut stream, &files, behavior);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                endpoint: format!("http://{address}/dav"),
                stop,
                thread: Some(thread),
            }
        }
    }

    impl Drop for TestWebDavServer {
        fn drop(&mut self) {
            self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
            let _ = std::net::TcpStream::connect(
                self.endpoint
                    .trim_start_matches("http://")
                    .split('/')
                    .next()
                    .unwrap(),
            );
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn handle_test_webdav_request(
        stream: &mut std::net::TcpStream,
        files: &std::sync::Mutex<BTreeMap<String, Vec<u8>>>,
        behavior: TestWebDavBehavior,
    ) {
        use std::io::{Read, Write};
        let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
        let mut request = Vec::new();
        let mut chunk = [0_u8; 4096];
        let header_end = loop {
            let Ok(read) = stream.read(&mut chunk) else {
                return;
            };
            if read == 0 {
                return;
            }
            request.extend_from_slice(&chunk[..read]);
            if let Some(index) = request.windows(4).position(|value| value == b"\r\n\r\n") {
                break index + 4;
            }
        };
        let headers = String::from_utf8_lossy(&request[..header_end]).into_owned();
        let mut request_line = headers
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        let method = request_line.next().unwrap_or_default();
        let path = request_line.next().unwrap_or_default().to_string();
        let content_length = headers
            .lines()
            .find_map(|line| {
                line.split_once(':').and_then(|(name, value)| {
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
            })
            .unwrap_or(0);
        while request.len() < header_end + content_length {
            let Ok(read) = stream.read(&mut chunk) else {
                return;
            };
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
        }
        let body = request[header_end..request.len().min(header_end + content_length)].to_vec();
        let authenticated = headers
            .lines()
            .any(|line| line.eq_ignore_ascii_case("Authorization: Basic d3JpdGVyOnNlY3JldA=="));
        if behavior.require_auth && !authenticated {
            let response = "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"test\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(response.as_bytes());
            return;
        }
        if behavior.forbidden_method == Some(method) {
            let response =
                "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(response.as_bytes());
            return;
        }
        let (status, response_body, etag) = match method {
            "MKCOL" => (201, Vec::new(), None),
            "PUT" => {
                let already_exists = files.lock().unwrap().contains_key(&path);
                let create_only = headers
                    .lines()
                    .any(|line| line.eq_ignore_ascii_case("If-None-Match: *"));
                if already_exists && create_only {
                    (412, Vec::new(), None)
                } else {
                    files.lock().unwrap().insert(path.clone(), body);
                    (
                        201,
                        Vec::new(),
                        path.ends_with(MANIFEST).then_some("\"v1\""),
                    )
                }
            }
            "GET" => match files.lock().unwrap().get(&path).cloned() {
                Some(value) => (200, value, path.ends_with(MANIFEST).then_some("\"v1\"")),
                None => (404, Vec::new(), None),
            },
            "PROPFIND" => (
                207,
                b"<d:multistatus xmlns:d=\"DAV:\"></d:multistatus>".to_vec(),
                None,
            ),
            "DELETE" => {
                files.lock().unwrap().remove(&path);
                (204, Vec::new(), None)
            }
            _ => (405, Vec::new(), None),
        };
        let reason = match status {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            207 => "Multi-Status",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            412 => "Precondition Failed",
            _ => "Unknown",
        };
        let etag_header = etag.map_or(String::new(), |value| format!("ETag: {value}\r\n"));
        let response = format!(
            "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\n{etag_header}Connection: close\r\n\r\n",
            response_body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(&response_body);
    }

    #[test]
    fn normalizes_github_https_and_ssh_remotes() {
        assert_eq!(
            normalize_github_remote("git@github.com:owner/blog.git").as_deref(),
            Some("git@github.com:owner/blog.git")
        );
        assert_eq!(
            normalize_github_remote("https://github.com/owner/blog").as_deref(),
            Some("https://github.com/owner/blog.git")
        );
        assert!(normalize_github_remote("https://user:pass@github.com/owner/blog").is_none());
        assert!(normalize_github_remote("https://example.com/owner/blog").is_none());
    }

    #[test]
    fn validates_webdav_endpoint_remote_directory_and_empty_listing() {
        assert_eq!(
            normalize_webdav_endpoint("https://dav.example.com/root/").unwrap(),
            "https://dav.example.com/root"
        );
        assert!(normalize_webdav_endpoint("https://user:secret@dav.example.com/root").is_err());
        assert!(normalize_webdav_endpoint("http://dav.example.com/root").is_err());
        assert!(!local_http_webdav_allowed("http", Some("192.0.2.1")));
        assert_eq!(
            validate_webdav_remote_dir("/hexo/my-blog/").unwrap(),
            "hexo/my-blog"
        );
        assert!(validate_webdav_remote_dir("hexo/../secret").is_err());

        let collection = url::Url::parse("https://dav.example.com/root/hexo/").unwrap();
        let empty = r#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>/root/hexo/</d:href></d:response><d:response><d:href>/root/hexo/.hexo-lite-objects/</d:href></d:response></d:multistatus>"#;
        assert!(!webdav_listing_has_user_content(empty, &collection));
        let occupied = r#"<d:multistatus xmlns:d="DAV:"><d:response><d:href>/root/hexo/</d:href></d:response><d:response><d:href>/root/hexo/other.txt</d:href></d:response></d:multistatus>"#;
        assert!(webdav_listing_has_user_content(occupied, &collection));
    }

    #[test]
    fn webdav_probe_requires_real_basic_auth_and_checks_read_write_delete() {
        let server = TestWebDavServer::start();
        let valid = crate::platform::WebDavCredentials {
            username: "writer".to_string(),
            password: "secret".to_string(),
        };
        test_webdav_connection(&server.endpoint, "probe-content", &valid).unwrap();

        let invalid = crate::platform::WebDavCredentials {
            username: "writer".to_string(),
            password: "wrong".to_string(),
        };
        let error =
            test_webdav_connection(&server.endpoint, "probe-content", &invalid).unwrap_err();
        assert_eq!(error.code, "sync_auth_required");
    }

    #[test]
    fn webdav_probe_reports_permission_write_and_cleanup_failures_separately() {
        let credentials = crate::platform::WebDavCredentials {
            username: "writer".to_string(),
            password: "secret".to_string(),
        };

        let propfind_forbidden = TestWebDavServer::start_with_forbidden_method("PROPFIND");
        let error =
            test_webdav_connection(&propfind_forbidden.endpoint, "probe-content", &credentials)
                .unwrap_err();
        assert_eq!(error.code, "sync_permission_denied");

        let put_forbidden = TestWebDavServer::start_with_forbidden_method("PUT");
        let error = test_webdav_connection(&put_forbidden.endpoint, "probe-content", &credentials)
            .unwrap_err();
        assert_eq!(error.code, "webdav_write_required");
        assert!(error.message.contains("上传、下载和删除权限"));

        let delete_forbidden = TestWebDavServer::start_with_forbidden_method("DELETE");
        let error =
            test_webdav_connection(&delete_forbidden.endpoint, "probe-content", &credentials)
                .unwrap_err();
        assert_eq!(error.code, "webdav_probe_cleanup_failed");
    }

    #[test]
    fn invalid_webdav_manifest_does_not_replace_stored_credentials() {
        use crate::app::ProjectSession;
        use std::collections::HashMap;

        let _credential_guard = real_webdav_credential_test_guard();
        let server = TestWebDavServer::start();
        let endpoint = server.endpoint.clone();
        let remote_dir = format!("invalid-manifest/{}", Uuid::new_v4());
        let manifest_url = webdav_resource_url(&endpoint, &remote_dir, Some(MANIFEST)).unwrap();
        let response = reqwest::blocking::Client::new()
            .put(manifest_url)
            .basic_auth("writer", Some("secret"))
            .body(b"{not-valid-json".to_vec())
            .send()
            .unwrap();
        assert!(response.status().is_success());
        set_webdav_credentials(&endpoint, "stored-user", "stored-password").unwrap();

        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("project");
        let config = temp.path().join("config");
        fs::create_dir_all(root.join("source/_posts")).unwrap();
        fs::create_dir_all(&config).unwrap();
        let state = AppState::new(&config);
        *state.project.write().unwrap() = Some(ProjectSession {
            id: "invalid-manifest-project".to_string(),
            generation: 1,
            name: "Invalid manifest".to_string(),
            root,
            warnings: Vec::new(),
            article_summaries: Vec::new(),
            articles: HashMap::new(),
            assets: HashMap::new(),
            remote_assets: HashMap::new(),
        });

        let error = test_webdav_content_sync_inner(
            TestWebDavContentSyncRequest {
                project_id: "invalid-manifest-project".to_string(),
                session_generation: 1,
                endpoint: endpoint.clone(),
                remote_dir,
                username: "writer".to_string(),
                password: "secret".to_string(),
            },
            &state,
        )
        .unwrap_err();
        assert_eq!(error.code, "sync_manifest_invalid");
        let stored = webdav_credentials(&endpoint).unwrap();
        assert_eq!(stored.username, "stored-user");
        assert_eq!(stored.password, "stored-password");
        delete_webdav_credentials(&endpoint).unwrap();
    }

    #[test]
    fn real_webdav_tauri_ipc_keeps_last_valid_credentials_after_bad_password() {
        use crate::app::ProjectSession;
        use std::collections::HashMap;
        use tauri::Manager;

        let Some((endpoint, username, password)) = real_webdav_environment() else {
            eprintln!("real WebDAV environment is not configured; integration test skipped");
            return;
        };
        let _credential_guard = real_webdav_credential_test_guard();
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("project");
        let config = temp.path().join("config");
        fs::create_dir_all(root.join("source/_posts")).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(root.join("source/_posts/ipc.md"), "real WebDAV IPC test").unwrap();
        let state = AppState::new(&config);
        let current_remote_dir = format!("integration-current/{}", Uuid::new_v4());
        save_registry(
            &state,
            &SyncRegistry {
                records: vec![SyncRecord {
                    project_path: path_key(&root),
                    provider: ContentSyncProvider::Webdav,
                    repository: endpoint.clone(),
                    branch: current_remote_dir,
                    image_dir: "source/images".to_string(),
                    enabled: true,
                    visibility: "private".to_string(),
                    status: ContentSyncStatus::AuthRequired,
                    base_files: BTreeMap::new(),
                    conflicts: Vec::new(),
                    conflict_remote_head: None,
                    remote_etag: None,
                    remote_manifest_exists: false,
                    message: Some("expired credentials".to_string()),
                    last_synced_at: Some("2026-07-25T00:00:00+08:00".to_string()),
                }],
            },
        )
        .unwrap();
        *state.project.write().unwrap() = Some(ProjectSession {
            id: "webdav-ipc-project".to_string(),
            generation: 1,
            name: "WebDAV IPC".to_string(),
            root: root.clone(),
            warnings: Vec::new(),
            article_summaries: Vec::new(),
            articles: HashMap::new(),
            assets: HashMap::new(),
            remote_assets: HashMap::new(),
        });
        let app = tauri::test::mock_builder()
            .manage(state)
            .invoke_handler(crate::webdav_invoke_handler())
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let webview = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
            .build()
            .unwrap();
        let remote_dir = format!("integration/{}", Uuid::new_v4());
        let invoke = |command: &str, body: serde_json::Value| {
            tauri::test::get_ipc_response(
                &webview,
                tauri::webview::InvokeRequest {
                    cmd: command.to_string(),
                    callback: tauri::ipc::CallbackFn(0),
                    error: tauri::ipc::CallbackFn(1),
                    url: "http://tauri.localhost".parse().unwrap(),
                    body: tauri::ipc::InvokeBody::Json(body),
                    headers: Default::default(),
                    invoke_key: tauri::test::INVOKE_KEY.to_string(),
                },
            )
        };
        let request = |candidate_password: &str| {
            serde_json::json!({
                "projectId": "webdav-ipc-project",
                "sessionGeneration": 1,
                "endpoint": endpoint,
                "remoteDir": remote_dir,
                "username": username,
                "password": candidate_password,
            })
        };
        let request_body = |request| serde_json::json!({ "request": request });
        let valid = invoke("test_webdav_content_sync", request_body(request(&password)))
            .unwrap()
            .deserialize::<serde_json::Value>()
            .unwrap();
        assert_eq!(valid["username"], username);
        assert_eq!(valid["preflight"]["remoteDir"], remote_dir);
        assert!(valid.get("password").is_none());
        let invalid = invoke(
            "test_webdav_content_sync",
            request_body(request("definitely-wrong")),
        )
        .unwrap_err();
        assert_eq!(invalid["code"], "sync_auth_required");
        let reused = invoke("test_webdav_content_sync", request_body(request("")))
            .unwrap()
            .deserialize::<serde_json::Value>()
            .unwrap();
        assert_eq!(reused["username"], username);
        let status = invoke(
            "webdav_credential_status",
            serde_json::json!({ "endpoint": endpoint }),
        )
        .unwrap()
        .deserialize::<serde_json::Value>()
        .unwrap();
        assert_eq!(status["configured"], true);
        assert_eq!(status["username"], username);
        assert!(status.get("password").is_none());
        let updated = invoke(
            "update_webdav_content_sync",
            request_body(serde_json::json!({
                "projectId": "webdav-ipc-project",
                "sessionGeneration": 1,
                "endpoint": endpoint,
                "remoteDir": remote_dir,
            })),
        )
        .unwrap()
        .deserialize::<serde_json::Value>()
        .unwrap();
        assert_eq!(updated["status"], "localPending");
        assert_eq!(updated["remoteDir"], remote_dir);
        assert!(updated["lastSyncedAt"].is_null());
        let registry = load_registry(&app.state::<AppState>()).unwrap();
        assert_eq!(registry.records[0].branch, remote_dir);
        assert!(registry.records[0].base_files.is_empty());
        let managed = app.state::<AppState>();
        assert_eq!(
            managed.project.read().unwrap().as_ref().unwrap().id,
            "webdav-ipc-project"
        );
        delete_webdav_credentials(&endpoint).unwrap();
    }

    #[test]
    fn real_webdav_auth_recovery_and_connection_update_are_revalidated() {
        use crate::app::ProjectSession;
        use std::collections::HashMap;

        let Some((endpoint, username, password)) = real_webdav_environment() else {
            eprintln!("real WebDAV environment is not configured; integration test skipped");
            return;
        };
        let _credential_guard = real_webdav_credential_test_guard();
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("project");
        let config = temp.path().join("config");
        fs::create_dir_all(root.join("source/_posts")).unwrap();
        fs::create_dir_all(&config).unwrap();
        fs::write(root.join("source/_posts/update.md"), "connection update").unwrap();
        let state = AppState::new(&config);
        *state.project.write().unwrap() = Some(ProjectSession {
            id: "webdav-update-project".to_string(),
            generation: 1,
            name: "WebDAV update".to_string(),
            root: root.clone(),
            warnings: Vec::new(),
            article_summaries: Vec::new(),
            articles: HashMap::new(),
            assets: HashMap::new(),
            remote_assets: HashMap::new(),
        });
        let current_dir = format!("auth-recovery/{}", Uuid::new_v4());
        let next_dir = format!("connection-update/{}", Uuid::new_v4());
        save_registry(
            &state,
            &SyncRegistry {
                records: vec![SyncRecord {
                    project_path: path_key(&root),
                    provider: ContentSyncProvider::Webdav,
                    repository: endpoint.clone(),
                    branch: current_dir.clone(),
                    image_dir: "source/images".to_string(),
                    enabled: true,
                    visibility: "private".to_string(),
                    status: ContentSyncStatus::AuthRequired,
                    base_files: BTreeMap::new(),
                    conflicts: Vec::new(),
                    conflict_remote_head: None,
                    remote_etag: None,
                    remote_manifest_exists: false,
                    message: Some("expired credentials".to_string()),
                    last_synced_at: Some("2026-07-25T00:00:00+08:00".to_string()),
                }],
            },
        )
        .unwrap();

        let test_request =
            |remote_dir: &str, candidate_password: &str| TestWebDavContentSyncRequest {
                project_id: "webdav-update-project".to_string(),
                session_generation: 1,
                endpoint: endpoint.clone(),
                remote_dir: remote_dir.to_string(),
                username: username.clone(),
                password: candidate_password.to_string(),
            };
        let recovered =
            test_webdav_content_sync_inner(test_request(&current_dir, &password), &state).unwrap();
        assert!(matches!(
            recovered.sync.status,
            ContentSyncStatus::LocalPending
        ));
        assert_eq!(
            recovered.sync.last_synced_at.as_deref(),
            Some("2026-07-25T00:00:00+08:00")
        );

        test_webdav_content_sync_inner(test_request(&next_dir, &password), &state).unwrap();
        set_webdav_credentials(&endpoint, &username, "definitely-wrong").unwrap();
        let update_request = || UpdateWebDavContentSyncRequest {
            project_id: "webdav-update-project".to_string(),
            session_generation: 1,
            endpoint: endpoint.clone(),
            remote_dir: next_dir.clone(),
        };
        let failed =
            update_webdav_content_sync_request_inner(update_request(), &state).unwrap_err();
        assert_eq!(failed.code, "sync_auth_required");
        let unchanged = load_registry(&state).unwrap();
        assert_eq!(unchanged.records[0].branch, current_dir);

        test_webdav_content_sync_inner(test_request(&next_dir, &password), &state).unwrap();
        let applied = update_webdav_content_sync_request_inner(update_request(), &state).unwrap();
        assert!(matches!(applied.status, ContentSyncStatus::LocalPending));
        assert_eq!(applied.remote_dir.as_deref(), Some(next_dir.as_str()));
        assert!(applied.last_synced_at.is_none());
        delete_webdav_credentials(&endpoint).unwrap();
    }

    #[test]
    fn real_webdav_standard_server_round_trip_and_etag_guard() {
        let Some((endpoint, username, password)) = real_webdav_environment() else {
            eprintln!("real WebDAV environment is not configured; integration test skipped");
            return;
        };
        let credentials = crate::platform::WebDavCredentials { username, password };
        let temp = tempfile::TempDir::new().unwrap();
        let upload = temp.path().join("upload");
        let download = temp.path().join("download");
        fs::create_dir_all(upload.join("source/_posts")).unwrap();
        fs::write(
            upload.join("source/_posts/round-trip.md"),
            "WsgiDAV round trip",
        )
        .unwrap();
        let snapshot = local_snapshot(&upload, "source/images").unwrap();
        let manifest = SyncManifest {
            schema_version: 1,
            image_dir: "source/images".to_string(),
            files: hash_map(&snapshot),
        };
        write_manifest(&upload, &manifest).unwrap();
        let remote_dir = format!("round-trip/{}", Uuid::new_v4());
        publish_webdav_remote_with_credentials(
            &upload,
            &endpoint,
            &remote_dir,
            &manifest,
            false,
            None,
            &credentials,
        )
        .unwrap();
        let fetched = fetch_webdav_remote(
            &download,
            &endpoint,
            &remote_dir,
            &credentials,
            WEBDAV_TIMEOUT,
        )
        .unwrap();
        assert!(fetched.exists);
        assert!(fetched.etag.is_some());
        assert_eq!(
            fs::read_to_string(download.join("source/_posts/round-trip.md")).unwrap(),
            "WsgiDAV round trip"
        );
        let stale = publish_webdav_remote_with_credentials(
            &upload,
            &endpoint,
            &remote_dir,
            &manifest,
            true,
            Some("\"stale-etag\""),
            &credentials,
        )
        .unwrap_err();
        assert_eq!(stale.message, REMOTE_AHEAD_MESSAGE);
    }

    #[test]
    fn webdav_content_addressed_round_trip_uploads_manifest_last_and_downloads_snapshot() {
        let server = TestWebDavServer::start();
        let temp = tempfile::TempDir::new().unwrap();
        let upload = temp.path().join("upload");
        let download = temp.path().join("download");
        fs::create_dir_all(upload.join("source/_posts")).unwrap();
        fs::write(upload.join("source/_posts/hello.md"), "hello from WebDAV").unwrap();
        let snapshot = local_snapshot(&upload, "source/images").unwrap();
        let manifest = SyncManifest {
            schema_version: 1,
            image_dir: "source/images".to_string(),
            files: hash_map(&snapshot),
        };
        write_manifest(&upload, &manifest).unwrap();
        let credentials = crate::platform::WebDavCredentials {
            username: "writer".to_string(),
            password: "secret".to_string(),
        };

        publish_webdav_remote_with_credentials(
            &upload,
            &server.endpoint,
            "blog-content",
            &manifest,
            false,
            None,
            &credentials,
        )
        .unwrap();
        let fetched = fetch_webdav_remote(
            &download,
            &server.endpoint,
            "blog-content",
            &credentials,
            Duration::from_secs(2),
        )
        .unwrap();

        assert!(fetched.exists);
        assert_eq!(fetched.etag.as_deref(), Some("\"v1\""));
        let downloaded_manifest = read_manifest(&download).unwrap();
        assert_eq!(downloaded_manifest.files, manifest.files);
        assert_eq!(
            fs::read_to_string(download.join("source/_posts/hello.md")).unwrap(),
            "hello from WebDAV"
        );
        let stale_push = publish_webdav_remote_with_credentials(
            &upload,
            &server.endpoint,
            "blog-content",
            &manifest,
            false,
            None,
            &credentials,
        )
        .unwrap_err();
        assert_eq!(stale_push.message, REMOTE_AHEAD_MESSAGE);
    }

    #[test]
    fn validates_branch_names() {
        assert_eq!(validate_branch(DEFAULT_BRANCH).unwrap(), DEFAULT_BRANCH);
        assert!(validate_branch("bad..branch").is_err());
        assert!(validate_branch("bad branch").is_err());
    }

    #[test]
    fn classifies_git_auth_offline_and_non_fast_forward_failures() {
        let auth = classify_git_failure("fatal: Authentication failed".to_string());
        assert!(auth.auth);
        assert!(!auth.offline);
        let offline = classify_git_failure("Could not resolve host: github.com".to_string());
        assert!(offline.offline);
        assert!(!offline.auth);
        let ahead = classify_git_failure(
            "rejected (non-fast-forward); failed to push some refs".to_string(),
        );
        assert_eq!(ahead.message, REMOTE_AHEAD_MESSAGE);
        assert!(!ahead.auth);
        assert!(!ahead.offline);
    }

    #[test]
    fn detects_multiple_git_deploy_repositories_without_falling_back_to_origin() {
        let temp = tempfile::TempDir::new().unwrap();
        fs::write(
            temp.path().join("_config.yml"),
            "deploy:\n  - type: git\n    repo: https://github.com/owner/pages.git\n    branch: gh-pages\n  - type: git\n    repository: git@github.com:owner/mirror.git\n    branch: main\n",
        )
        .unwrap();
        let candidates = detect_candidates(temp.path());
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].pages_branch.as_deref(), Some("gh-pages"));
    }

    #[test]
    fn validates_manifest_paths_and_excludes_drafts() {
        let hash = "a".repeat(64);
        let valid = BTreeMap::from([
            ("source/_posts/hello.md".to_string(), hash.clone()),
            ("source/_posts/hello/a.png".to_string(), hash.clone()),
            ("source/images/a.png".to_string(), hash.clone()),
        ]);
        assert!(validate_manifest_paths(&valid, "source/images").is_ok());
        let draft = BTreeMap::from([("source/_drafts/private.md".to_string(), hash.clone())]);
        assert!(validate_manifest_paths(&draft, "source/images").is_err());
        let orphan = BTreeMap::from([("source/_posts/orphan/a.png".to_string(), hash)]);
        assert!(validate_manifest_paths(&orphan, "source/images").is_err());
        assert!(validate_manifest_path_shape("../secret").is_err());
        assert!(validate_sync_image_dir("source/_drafts").is_err());
    }

    #[test]
    fn local_snapshot_only_contains_posts_resources_and_configured_images() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("source/_posts/hello")).unwrap();
        fs::create_dir_all(root.join("source/_drafts")).unwrap();
        fs::create_dir_all(root.join("source/images")).unwrap();
        fs::write(root.join("source/_posts/hello.md"), "post").unwrap();
        fs::write(root.join("source/_posts/hello/a.png"), "asset").unwrap();
        fs::write(root.join("source/_drafts/private.md"), "secret").unwrap();
        fs::write(root.join("source/images/site.png"), "image").unwrap();
        fs::write(root.join("source/images/site.png.bak"), "backup").unwrap();
        fs::write(root.join("source/images/id_rsa"), "private key").unwrap();
        fs::write(root.join("_config.yml"), "token: secret").unwrap();
        let snapshot = local_snapshot(root, "source/images").unwrap();
        assert!(snapshot.contains_key("source/_posts/hello.md"));
        assert!(snapshot.contains_key("source/_posts/hello/a.png"));
        assert!(snapshot.contains_key("source/images/site.png"));
        assert!(!snapshot.contains_key("source/images/site.png.bak"));
        assert!(!snapshot.contains_key("source/images/id_rsa"));
        assert!(!snapshot.contains_key("source/_drafts/private.md"));
        assert!(!snapshot.contains_key("_config.yml"));
    }

    #[test]
    fn applies_only_remote_changed_paths_and_preserves_local_changes() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("project");
        let cache = temp.path().join("cache");
        let config = temp.path().join("config");
        fs::create_dir_all(root.join("source/_posts")).unwrap();
        fs::create_dir_all(cache.join("source/_posts")).unwrap();
        fs::write(root.join("source/_posts/local.md"), "local changed").unwrap();
        fs::write(root.join("source/_posts/remote.md"), "base").unwrap();
        fs::write(cache.join("source/_posts/local.md"), "base").unwrap();
        fs::write(cache.join("source/_posts/remote.md"), "remote changed").unwrap();
        let local = local_snapshot(&root, "source/images").unwrap();
        let remote = local_snapshot(&cache, "source/images").unwrap();
        let selected_remote = Snapshot::from([(
            "source/_posts/remote.md".to_string(),
            remote["source/_posts/remote.md"].clone(),
        )]);
        let selected_base =
            BTreeMap::from([("source/_posts/remote.md".to_string(), hash_bytes(b"base"))]);
        let state = AppState::new(&config);
        apply_remote(
            &state,
            &root,
            &cache,
            &local,
            &selected_remote,
            &selected_base,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("source/_posts/local.md")).unwrap(),
            "local changed"
        );
        assert_eq!(
            fs::read_to_string(root.join("source/_posts/remote.md")).unwrap(),
            "remote changed"
        );
    }

    #[test]
    fn remote_delete_only_removes_tracked_unmodified_files() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("project");
        let cache = temp.path().join("cache");
        let state = AppState::new(&temp.path().join("config"));
        fs::create_dir_all(root.join("source/_posts")).unwrap();
        fs::create_dir_all(&cache).unwrap();
        fs::write(root.join("source/_posts/tracked.md"), "base").unwrap();
        fs::write(root.join("source/_posts/modified.md"), "local change").unwrap();
        fs::write(root.join("source/_posts/untracked.md"), "keep").unwrap();
        let local = local_snapshot(&root, "source/images").unwrap();
        let base = BTreeMap::from([
            ("source/_posts/tracked.md".to_string(), hash_bytes(b"base")),
            ("source/_posts/modified.md".to_string(), hash_bytes(b"base")),
        ]);
        apply_remote(&state, &root, &cache, &local, &Snapshot::new(), &base).unwrap();
        assert!(!root.join("source/_posts/tracked.md").exists());
        assert!(root.join("source/_posts/modified.md").exists());
        assert!(root.join("source/_posts/untracked.md").exists());
    }

    #[test]
    fn backup_pruning_keeps_the_latest_ten_directories() {
        let temp = tempfile::TempDir::new().unwrap();
        for index in 0..12 {
            fs::create_dir(temp.path().join(format!("backup-{index:02}"))).unwrap();
        }
        fs::write(temp.path().join("README.txt"), "not a backup directory").unwrap();
        prune_backups(temp.path()).unwrap();
        let remaining = fs::read_dir(temp.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect::<BTreeSet<_>>();
        assert_eq!(remaining.len(), MAX_BACKUPS);
        assert!(!remaining.contains("backup-00"));
        assert!(!remaining.contains("backup-01"));
        assert!(remaining.contains("backup-11"));
        assert!(temp.path().join("README.txt").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_to_apply_through_a_symbolic_link() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("project");
        let cache = temp.path().join("cache");
        let outside = temp.path().join("outside");
        fs::create_dir_all(root.join("source/_posts")).unwrap();
        fs::create_dir_all(cache.join("source/_posts/link")).unwrap();
        fs::create_dir_all(&outside).unwrap();
        symlink(&outside, root.join("source/_posts/link")).unwrap();
        fs::write(cache.join("source/_posts/link/a.png"), "remote").unwrap();
        let remote = Snapshot::from([(
            "source/_posts/link/a.png".to_string(),
            FileSnapshot {
                hash: hash_bytes(b"remote"),
            },
        )]);
        let state = AppState::new(&temp.path().join("config"));
        let result = apply_remote(
            &state,
            &root,
            &cache,
            &Snapshot::new(),
            &remote,
            &BTreeMap::new(),
        );
        assert!(result.is_err());
        assert!(!outside.join("a.png").exists());
    }

    #[test]
    fn bare_repository_round_trip_keeps_content_branch_isolated() {
        let temp = tempfile::TempDir::new().unwrap();
        let remote = temp.path().join("remote.git");
        let cache = temp.path().join("cache");
        let project = temp.path().join("project");
        fs::create_dir_all(project.join("source/_posts")).unwrap();
        fs::create_dir_all(project.join("source/images")).unwrap();
        fs::write(project.join("source/_posts/hello.md"), "first").unwrap();
        assert!(Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote)
            .status()
            .unwrap()
            .success());
        ensure_cache(&cache, &remote.to_string_lossy()).unwrap();
        prepare_orphan_cache(&cache).unwrap();
        let first = local_snapshot(&project, "source/images").unwrap();
        copy_snapshot_to_cache(&project, &cache, &first, "source/images").unwrap();
        write_manifest(
            &cache,
            &SyncManifest {
                schema_version: 1,
                image_dir: "source/images".to_string(),
                files: hash_map(&first),
            },
        )
        .unwrap();
        commit_and_push(&cache, DEFAULT_BRANCH).unwrap();

        fs::write(project.join("source/_posts/hello.md"), "second").unwrap();
        fs::write(project.join("source/_posts/new.md"), "new").unwrap();
        let second = local_snapshot(&project, "source/images").unwrap();
        copy_snapshot_to_cache(&project, &cache, &second, "source/images").unwrap();
        write_manifest(
            &cache,
            &SyncManifest {
                schema_version: 1,
                image_dir: "source/images".to_string(),
                files: hash_map(&second),
            },
        )
        .unwrap();
        commit_and_push(&cache, DEFAULT_BRANCH).unwrap();

        let refs = git(&cache, &["ls-remote", "--heads", "origin"], false).unwrap();
        assert!(refs.contains(&format!("refs/heads/{DEFAULT_BRANCH}")));
        assert_eq!(refs.lines().count(), 1);
        let tree = git(&cache, &["ls-tree", "-r", "--name-only", "HEAD"], false).unwrap();
        assert!(tree.contains(MANIFEST));
        assert!(tree.contains("source/_posts/hello.md"));
        assert!(tree.contains("source/_posts/new.md"));
        assert!(!tree.contains("_config.yml"));
    }

    #[test]
    fn bare_repository_startup_pull_and_remote_ahead_keep_pages_ref_unchanged() {
        let temp = tempfile::TempDir::new().unwrap();
        let remote = temp.path().join("remote.git");
        let pages = temp.path().join("pages");
        let project = temp.path().join("project");
        let config = temp.path().join("config");
        fs::create_dir_all(project.join("source/_posts")).unwrap();
        fs::create_dir_all(project.join("source/images")).unwrap();
        fs::write(project.join("source/_posts/local.md"), "base local").unwrap();
        fs::write(project.join("source/_posts/remote.md"), "base remote").unwrap();
        git_test_init_bare(&remote);

        fs::create_dir_all(&pages).unwrap();
        git(&pages, &["init"], false).unwrap();
        fs::write(pages.join("index.html"), "published pages").unwrap();
        git(&pages, &["add", "index.html"], false).unwrap();
        git_test_commit(&pages, "pages");
        git(
            &pages,
            &["remote", "add", "origin", &remote.to_string_lossy()],
            false,
        )
        .unwrap();
        git(
            &pages,
            &["push", "origin", "HEAD:refs/heads/gh-pages"],
            false,
        )
        .unwrap();
        let pages_before = git(
            &pages,
            &["ls-remote", "origin", "refs/heads/gh-pages"],
            false,
        )
        .unwrap();

        let state = AppState::new(&config);
        let key = path_key(&project);
        let record = SyncRecord {
            project_path: key.clone(),
            provider: ContentSyncProvider::Github,
            repository: remote.to_string_lossy().to_string(),
            branch: DEFAULT_BRANCH.to_string(),
            image_dir: "source/images".to_string(),
            enabled: true,
            visibility: "private".to_string(),
            status: ContentSyncStatus::LocalPending,
            base_files: BTreeMap::new(),
            conflicts: Vec::new(),
            conflict_remote_head: None,
            remote_etag: None,
            remote_manifest_exists: false,
            message: None,
            last_synced_at: None,
        };
        save_registry(
            &state,
            &SyncRegistry {
                records: vec![record],
            },
        )
        .unwrap();
        assert!(matches!(
            run_sync_for_root(&state, &project, "local").status,
            ContentSyncStatus::Synced
        ));

        let cache = state.sync_cache_dir.join(cache_key(&key));
        fs::write(project.join("source/_posts/local.md"), "local changed").unwrap();
        fs::write(cache.join("source/_posts/remote.md"), "remote changed").unwrap();
        let remote_snapshot = local_snapshot(&cache, "source/images").unwrap();
        write_manifest(
            &cache,
            &SyncManifest {
                schema_version: 1,
                image_dir: "source/images".to_string(),
                files: hash_map(&remote_snapshot),
            },
        )
        .unwrap();
        commit_and_push(&cache, DEFAULT_BRANCH).unwrap();

        assert!(matches!(
            run_sync_for_root(&state, &project, "startup").status,
            ContentSyncStatus::Synced
        ));
        assert_eq!(
            fs::read_to_string(project.join("source/_posts/local.md")).unwrap(),
            "local changed"
        );
        assert_eq!(
            fs::read_to_string(project.join("source/_posts/remote.md")).unwrap(),
            "remote changed"
        );

        fs::write(cache.join("source/_posts/remote.md"), "remote ahead").unwrap();
        let remote_snapshot = local_snapshot(&cache, "source/images").unwrap();
        write_manifest(
            &cache,
            &SyncManifest {
                schema_version: 1,
                image_dir: "source/images".to_string(),
                files: hash_map(&remote_snapshot),
            },
        )
        .unwrap();
        commit_and_push(&cache, DEFAULT_BRANCH).unwrap();
        assert!(matches!(
            run_sync_for_root(&state, &project, "push").status,
            ContentSyncStatus::RemoteAhead
        ));
        assert_eq!(
            fs::read_to_string(project.join("source/_posts/remote.md")).unwrap(),
            "remote changed"
        );
        let pages_after = git(
            &pages,
            &["ls-remote", "origin", "refs/heads/gh-pages"],
            false,
        )
        .unwrap();
        assert_eq!(pages_before, pages_after);
    }

    fn git_test_init_bare(remote: &Path) {
        assert!(Command::new("git")
            .args(["init", "--bare"])
            .arg(remote)
            .status()
            .unwrap()
            .success());
    }

    fn git_test_commit(root: &Path, message: &str) {
        git(
            root,
            &[
                "-c",
                "user.name=Hexo Lite Editor Test",
                "-c",
                "user.email=test@localhost",
                "commit",
                "-m",
                message,
            ],
            false,
        )
        .unwrap();
    }

    #[test]
    fn conflict_resolution_also_applies_non_conflicting_remote_changes() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("project");
        let cache = temp.path().join("cache");
        let config = temp.path().join("config");
        fs::create_dir_all(root.join("source/_posts")).unwrap();
        fs::create_dir_all(cache.join("source/_posts")).unwrap();
        fs::write(root.join("source/_posts/conflict.md"), "local").unwrap();
        fs::write(root.join("source/_posts/remote.md"), "base").unwrap();
        fs::write(cache.join("source/_posts/conflict.md"), "remote").unwrap();
        fs::write(cache.join("source/_posts/remote.md"), "remote-only").unwrap();
        let local = local_snapshot(&root, "source/images").unwrap();
        let remote = local_snapshot(&cache, "source/images").unwrap();
        let mut base = BTreeMap::new();
        base.insert("source/_posts/conflict.md".to_string(), hash_bytes(b"base"));
        base.insert("source/_posts/remote.md".to_string(), hash_bytes(b"base"));
        let record = SyncRecord {
            project_path: path_key(&root),
            provider: ContentSyncProvider::Github,
            repository: String::new(),
            branch: DEFAULT_BRANCH.to_string(),
            image_dir: "source/images".to_string(),
            enabled: true,
            visibility: "private".to_string(),
            status: ContentSyncStatus::Conflict,
            base_files: base,
            conflicts: vec!["source/_posts/conflict.md".to_string()],
            conflict_remote_head: None,
            remote_etag: None,
            remote_manifest_exists: false,
            message: None,
            last_synced_at: None,
        };
        let state = AppState::new(&config);
        fs::create_dir_all(&state.sync_cache_dir).unwrap();
        let choices =
            BTreeMap::from([("source/_posts/conflict.md".to_string(), "local".to_string())]);
        apply_conflict_choices(&state, &root, &cache, &record, &local, &remote, &choices).unwrap();
        assert_eq!(
            fs::read_to_string(root.join("source/_posts/conflict.md")).unwrap(),
            "local"
        );
        assert_eq!(
            fs::read_to_string(root.join("source/_posts/remote.md")).unwrap(),
            "remote-only"
        );
    }

    #[test]
    fn recovers_an_interrupted_apply_from_transaction_backup() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("project");
        let state = AppState::new(&temp.path().join("config"));
        fs::create_dir_all(root.join("source/_posts")).unwrap();
        let key = cache_key(&path_key(&root));
        let backup = state.sync_backup_dir.join(&key).join("backup-1");
        let transaction = state
            .sync_cache_dir
            .join(&key)
            .join("apply-transaction.json");
        fs::create_dir_all(backup.join("source/_posts")).unwrap();
        fs::create_dir_all(transaction.parent().unwrap()).unwrap();
        fs::write(root.join("source/_posts/hello.md"), "partial remote").unwrap();
        fs::write(backup.join("source/_posts/hello.md"), "local before sync").unwrap();
        let journal = ApplyTransaction {
            backup_name: "backup-1".to_string(),
            operations: BTreeSet::from(["source/_posts/hello.md".to_string()]),
        };
        fs::write(&transaction, serde_json::to_vec(&journal).unwrap()).unwrap();

        recover_pending_transaction(&state, &root).unwrap();

        assert_eq!(
            fs::read_to_string(root.join("source/_posts/hello.md")).unwrap(),
            "local before sync"
        );
        assert!(!transaction.exists());
    }
}
