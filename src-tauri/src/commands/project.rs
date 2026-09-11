use crate::{
    app::{AppState, ArticleRecord, ProjectSession},
    data::load_config,
    domain::{
        AppError, AppResult, ArticleKind, ArticleSummary, CreateArticleRequest, DocumentSnapshot,
        FrontMatterResult, OpenProjectResult, ProjectRescanResult, ProjectSessionView,
        RecentProjectView, RenameArticleRequest, SaveDocumentRequest, SaveDocumentResult,
    },
    engine::{
        article_asset_folder, article_target, parse_front_matter, scan_articles, summarize_article,
        validate_hexo_root,
    },
    platform::atomic_write,
};
use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value as YamlValue};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecentProjectRecord {
    recent_id: String,
    name: String,
    path: PathBuf,
    last_opened_at: String,
}

#[tauri::command]
pub fn pick_project(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Option<OpenProjectResult>> {
    let Some(selection) = app
        .dialog()
        .file()
        .set_title("打开 Hexo 项目")
        .blocking_pick_folder()
    else {
        return Ok(None);
    };
    let path = selection
        .into_path()
        .map_err(|error| AppError::invalid(error.to_string()))?;
    let result = open_project_path(&state, &path)?;
    super::sync::schedule_sync_after_open(
        app,
        result.session.project_id.clone(),
        result.session.generation,
    );
    Ok(Some(result))
}

#[tauri::command]
pub fn reopen_recent_project(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Option<OpenProjectResult>> {
    let records = load_recent_records(&state)?;
    let Some(record) = records
        .into_iter()
        .find(|record| recent_available(&record.path))
    else {
        return Ok(None);
    };
    match open_project_path(&state, &record.path) {
        Ok(project) => {
            super::sync::schedule_sync_after_open(
                app,
                project.session.project_id.clone(),
                project.session.generation,
            );
            Ok(Some(project))
        }
        Err(error) if error.recoverable => Ok(None),
        Err(error) => Err(error),
    }
}

#[tauri::command]
pub fn list_recent_projects(state: State<'_, AppState>) -> AppResult<Vec<RecentProjectView>> {
    Ok(load_recent_records(&state)?
        .into_iter()
        .map(|record| RecentProjectView {
            recent_id: record.recent_id,
            name: record.name,
            display_path: record.path.display().to_string(),
            last_opened_at: record.last_opened_at,
            available: recent_available(&record.path),
        })
        .collect())
}

#[tauri::command]
pub fn open_recent_project(
    recent_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<OpenProjectResult> {
    let record = load_recent_records(&state)?
        .into_iter()
        .find(|record| record.recent_id == recent_id)
        .ok_or_else(|| AppError::new("recent_project_not_found", "最近项目记录不存在。", true))?;
    if !recent_available(&record.path) {
        return Err(AppError::new(
            "recent_project_unavailable",
            "项目目录已移动或不可用，请重新选择项目文件夹。",
            true,
        ));
    }
    let result = open_project_path(&state, &record.path)?;
    super::sync::schedule_sync_after_open(
        app,
        result.session.project_id.clone(),
        result.session.generation,
    );
    Ok(result)
}

#[tauri::command]
pub fn remove_recent_project(recent_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let mut records = load_recent_records(&state)?;
    records.retain(|record| record.recent_id != recent_id);
    save_recent_records(&state, &records)
}

#[tauri::command]
pub fn clear_recent_projects(state: State<'_, AppState>) -> AppResult<()> {
    save_recent_records(&state, &[])
}

#[tauri::command]
pub fn current_project(state: State<'_, AppState>) -> AppResult<Option<ProjectSessionView>> {
    let guard = state
        .project
        .read()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    Ok(guard.as_ref().map(ProjectSession::view))
}

#[tauri::command]
pub fn close_project(state: State<'_, AppState>) -> AppResult<()> {
    cancel_project_work(&state);
    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    *guard = None;
    Ok(())
}

#[tauri::command]
pub fn list_articles(
    project_id: String,
    session_generation: u64,
    state: State<'_, AppState>,
) -> AppResult<Vec<ArticleSummary>> {
    state.with_project(&project_id, Some(session_generation), |project| {
        Ok(project.article_summaries.clone())
    })
}

#[tauri::command]
pub fn load_document(
    project_id: String,
    article_id: String,
    session_generation: u64,
    state: State<'_, AppState>,
) -> AppResult<DocumentSnapshot> {
    state.with_project(&project_id, Some(session_generation), |project| {
        let article = project.article(&article_id)?;
        let content = fs::read_to_string(&article.canonical_path)
            .map_err(|error| AppError::io("读取文章失败", error))?;
        Ok(DocumentSnapshot {
            project_id: project.id.clone(),
            article_id: article.id.clone(),
            content,
            revision: article.revision,
            session_generation: project.generation,
        })
    })
}

#[tauri::command]
pub fn parse_document_front_matter(content: String) -> FrontMatterResult {
    parse_front_matter(&content)
}

#[tauri::command]
pub fn save_document(
    request: SaveDocumentRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<SaveDocumentResult> {
    let root_for_lock = state.with_project(
        &request.project_id,
        Some(request.session_generation),
        |project| Ok(project.root.clone()),
    )?;
    let file_lock = state.project_file_lock(&root_for_lock)?;
    let _file_guard = file_lock.lock().map_err(|_| AppError::session_expired())?;
    let save_lock = state.article_save_lock(&request.article_id)?;
    let _save_guard = save_lock
        .lock()
        .map_err(|_| AppError::new("save_queue_poisoned", "文章保存队列不可用。", false))?;

    let (path, current_revision) = state.with_project(
        &request.project_id,
        Some(request.session_generation),
        |project| {
            let article = project.article(&request.article_id)?;
            Ok((article.canonical_path.clone(), article.revision))
        },
    )?;
    if request.revision < current_revision {
        return Err(AppError::new(
            "stale_revision",
            "收到过期的保存请求，当前编辑内容未被覆盖。",
            true,
        ));
    }

    let config = load_config(&state)?.config;
    if config.general.backup_before_save && path.exists() {
        backup_article(&path)?;
    }
    atomic_write(&path, request.content.as_bytes())?;

    let (root, kind) = state.with_project(
        &request.project_id,
        Some(request.session_generation),
        |project| {
            let kind = project
                .article_summaries
                .iter()
                .find(|summary| summary.article_id == request.article_id)
                .map(|summary| summary.kind)
                .ok_or_else(|| {
                    AppError::new("article_not_found", "文章已离开当前项目会话。", true)
                })?;
            Ok((project.root.clone(), kind))
        },
    )?;
    let (updated_summary, cover_asset) =
        summarize_article(&root, &path, kind, &request.article_id)?;

    let mut project_guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    let project = project_guard
        .as_mut()
        .ok_or_else(AppError::session_expired)?;
    project.require_identity(&request.project_id, Some(request.session_generation))?;
    let article = project
        .articles
        .get_mut(&request.article_id)
        .ok_or_else(|| AppError::new("article_not_found", "文章已离开当前项目会话。", true))?;
    article.revision = article.revision.max(request.revision);
    if let Some(summary) = project
        .article_summaries
        .iter_mut()
        .find(|summary| summary.article_id == request.article_id)
    {
        *summary = updated_summary;
    }
    if let Some((token, mut asset)) = cover_asset {
        asset.generation = request.session_generation;
        project.assets.insert(token, asset);
    }
    let result = SaveDocumentResult {
        article_id: request.article_id,
        accepted_revision: request.revision,
        saved_at: Local::now().to_rfc3339(),
    };
    drop(project_guard);
    drop(_file_guard);
    super::sync::schedule_sync_after_save(app, root);
    Ok(result)
}

#[tauri::command]
pub fn create_article(
    request: CreateArticleRequest,
    state: State<'_, AppState>,
) -> AppResult<ArticleSummary> {
    let (root, project_id, generation) = state.with_project(
        &request.project_id,
        Some(request.session_generation),
        |project| Ok((project.root.clone(), project.id.clone(), project.generation)),
    )?;
    let file_lock = state.project_file_lock(&root)?;
    let _file_guard = file_lock.lock().map_err(|_| AppError::session_expired())?;
    state.with_project(&project_id, Some(generation), |_| Ok(()))?;
    let target = article_target(&root, request.kind, &request.file_name)?;
    let date = request.date.trim();
    if date.is_empty() {
        return Err(AppError::invalid("文章日期不能为空。"));
    }
    let mut attributes = Mapping::new();
    attributes.insert(
        YamlValue::String("title".to_string()),
        YamlValue::String(request.title.trim().to_string()),
    );
    attributes.insert(
        YamlValue::String("imgbed_folder".to_string()),
        YamlValue::String(article_asset_folder(&request.title)),
    );
    attributes.insert(
        YamlValue::String("date".to_string()),
        YamlValue::String(date.to_string()),
    );
    attributes.insert(
        YamlValue::String("tags".to_string()),
        YamlValue::Sequence(
            clean_labels(&request.tags)
                .into_iter()
                .map(YamlValue::String)
                .collect(),
        ),
    );
    attributes.insert(
        YamlValue::String("categories".to_string()),
        YamlValue::Sequence(
            clean_labels(&request.categories)
                .into_iter()
                .map(YamlValue::String)
                .collect(),
        ),
    );
    let yaml = serde_yaml::to_string(&attributes)
        .map_err(|error| AppError::new("front_matter_serialize", error.to_string(), false))?;
    let content = format!("---\n{}---\n\n", yaml.trim_start_matches("---\n"));
    atomic_write(&target, content.as_bytes())?;
    let canonical = target
        .canonicalize()
        .map_err(|error| AppError::io("验证新文章失败", error))?;
    if !canonical.starts_with(&root) {
        return Err(AppError::new("path_escape", "新文章路径越出项目。", false));
    }
    let article_id = Uuid::new_v4().to_string();
    let (summary, cover_asset) = summarize_article(&root, &canonical, request.kind, &article_id)?;
    let remote_asset_folder = summary.asset_folder.clone();
    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    let project = guard.as_mut().ok_or_else(AppError::session_expired)?;
    project.require_identity(&project_id, Some(generation))?;
    project.articles.insert(
        article_id.clone(),
        ArticleRecord {
            id: article_id,
            canonical_path: canonical,
            revision: 0,
            remote_asset_folder,
        },
    );
    project.article_summaries.insert(0, summary.clone());
    if let Some((token, mut asset)) = cover_asset {
        asset.generation = generation;
        project.assets.insert(token, asset);
    }
    Ok(summary)
}

#[tauri::command]
pub fn rename_article(
    request: RenameArticleRequest,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<ArticleSummary> {
    ensure_article_action_available(&state)?;
    let new_title = request.new_title.trim();
    if new_title.is_empty() {
        return Err(AppError::invalid("文章标题不能为空。"));
    }
    let root_for_lock = state.with_project(
        &request.project_id,
        Some(request.session_generation),
        |project| Ok(project.root.clone()),
    )?;
    let file_lock = state.project_file_lock(&root_for_lock)?;
    let _file_guard = file_lock.lock().map_err(|_| AppError::session_expired())?;
    let save_lock = state.article_save_lock(&request.article_id)?;
    let _save_guard = save_lock
        .lock()
        .map_err(|_| AppError::new("save_queue_poisoned", "文章保存队列不可用。", false))?;
    let (root, source, kind) = state.with_project(
        &request.project_id,
        Some(request.session_generation),
        |project| {
            let article = project.article(&request.article_id)?;
            let kind = project
                .article_summaries
                .iter()
                .find(|summary| summary.article_id == request.article_id)
                .map(|summary| summary.kind)
                .ok_or_else(|| {
                    AppError::new("article_not_found", "文章已离开当前项目会话。", true)
                })?;
            Ok((project.root.clone(), article.canonical_path.clone(), kind))
        },
    )?;
    let original =
        fs::read_to_string(&source).map_err(|error| AppError::io("读取文章失败", error))?;
    let parsed = parse_front_matter(&original);
    if let Some(error) = parsed.error {
        return Err(AppError::new("front_matter_invalid", error, true));
    }
    let mut attributes = parsed.attributes;
    let object = attributes
        .as_object_mut()
        .ok_or_else(|| AppError::new("front_matter_invalid", "Front Matter 必须是对象。", true))?;
    object.insert("title".into(), serde_json::Value::String(new_title.into()));
    object.insert(
        "imgbed_folder".into(),
        serde_json::Value::String(article_asset_folder(new_title)),
    );
    let yaml = serde_yaml::to_string(&attributes)
        .map_err(|error| AppError::new("front_matter_serialize", error.to_string(), false))?;
    let content = format!(
        "---\n{}---\n{}",
        yaml.trim_start_matches("---\n"),
        parsed.body
    );
    let target = request
        .new_file_name
        .as_deref()
        .map(|name| article_target(&root, kind, name))
        .transpose()?
        .unwrap_or_else(|| source.clone());
    if target != source && target.exists() {
        return Err(AppError::new(
            "article_exists",
            "目标位置已有同名文章。",
            true,
        ));
    }
    atomic_write(&source, content.as_bytes())?;
    if target != source {
        if let Err(error) = fs::rename(&source, &target) {
            let _ = atomic_write(&source, original.as_bytes());
            return Err(AppError::io("重命名文章文件失败", error));
        }
    }
    let canonical = target
        .canonicalize()
        .map_err(|error| AppError::io("验证重命名后的文章失败", error))?;
    let (summary, cover_asset) = summarize_article(&root, &canonical, kind, &request.article_id)?;
    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    let project = guard.as_mut().ok_or_else(AppError::session_expired)?;
    project.require_identity(&request.project_id, Some(request.session_generation))?;
    if let Some(record) = project.articles.get_mut(&request.article_id) {
        record.canonical_path = canonical;
        record.remote_asset_folder = summary.asset_folder.clone();
    }
    if let Some(existing) = project
        .article_summaries
        .iter_mut()
        .find(|item| item.article_id == request.article_id)
    {
        *existing = summary.clone();
    }
    if let Some((token, mut asset)) = cover_asset {
        asset.generation = request.session_generation;
        project.assets.insert(token, asset);
    }
    drop(guard);
    drop(_file_guard);
    super::sync::schedule_sync_after_save(app, root);
    Ok(summary)
}

#[tauri::command]
pub fn delete_article(
    app: AppHandle,
    project_id: String,
    session_generation: u64,
    article_id: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    ensure_article_action_available(&state)?;
    let root_for_lock = state.with_project(&project_id, Some(session_generation), |project| {
        Ok(project.root.clone())
    })?;
    let file_lock = state.project_file_lock(&root_for_lock)?;
    let _file_guard = file_lock.lock().map_err(|_| AppError::session_expired())?;
    let save_lock = state.article_save_lock(&article_id)?;
    let _save_guard = save_lock
        .lock()
        .map_err(|_| AppError::new("save_queue_poisoned", "文章保存队列不可用。", false))?;
    let (root, path) = state.with_project(&project_id, Some(session_generation), |project| {
        Ok((
            project.root.clone(),
            project.article(&article_id)?.canonical_path.clone(),
        ))
    })?;
    trash::delete(&path).map_err(|error| {
        AppError::new(
            "article_delete_failed",
            format!("无法将文章移到回收站：{error}"),
            true,
        )
    })?;

    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    let project = guard.as_mut().ok_or_else(AppError::session_expired)?;
    project.require_identity(&project_id, Some(session_generation))?;
    project.articles.remove(&article_id);
    project
        .article_summaries
        .retain(|summary| summary.article_id != article_id);
    drop(guard);
    drop(_file_guard);
    super::sync::schedule_sync_after_save(app, root);
    Ok(())
}

#[tauri::command]
pub fn move_article(
    app: AppHandle,
    project_id: String,
    session_generation: u64,
    article_id: String,
    kind: ArticleKind,
    state: State<'_, AppState>,
) -> AppResult<ArticleSummary> {
    ensure_article_action_available(&state)?;
    let root_for_lock = state.with_project(&project_id, Some(session_generation), |project| {
        Ok(project.root.clone())
    })?;
    let file_lock = state.project_file_lock(&root_for_lock)?;
    let _file_guard = file_lock.lock().map_err(|_| AppError::session_expired())?;
    let save_lock = state.article_save_lock(&article_id)?;
    let _save_guard = save_lock
        .lock()
        .map_err(|_| AppError::new("save_queue_poisoned", "文章保存队列不可用。", false))?;
    let (root, source, current_kind) =
        state.with_project(&project_id, Some(session_generation), |project| {
            let article = project.article(&article_id)?;
            let current_kind = project
                .article_summaries
                .iter()
                .find(|summary| summary.article_id == article_id)
                .map(|summary| summary.kind)
                .ok_or_else(|| {
                    AppError::new("article_not_found", "文章已离开当前项目会话。", true)
                })?;
            Ok((
                project.root.clone(),
                article.canonical_path.clone(),
                current_kind,
            ))
        })?;
    if current_kind == kind {
        return state.with_project(&project_id, Some(session_generation), |project| {
            project
                .article_summaries
                .iter()
                .find(|summary| summary.article_id == article_id)
                .cloned()
                .ok_or_else(|| AppError::new("article_not_found", "文章已离开当前项目会话。", true))
        });
    }
    let canonical = crate::engine::move_article_files(&root, &source, current_kind, kind)?;
    let (summary, cover_asset) = summarize_article(&root, &canonical, kind, &article_id)?;

    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    let project = guard.as_mut().ok_or_else(AppError::session_expired)?;
    project.require_identity(&project_id, Some(session_generation))?;
    project
        .articles
        .get_mut(&article_id)
        .ok_or_else(|| AppError::new("article_not_found", "文章已离开当前项目会话。", true))?
        .canonical_path = canonical;
    if let Some(existing) = project
        .article_summaries
        .iter_mut()
        .find(|existing| existing.article_id == article_id)
    {
        *existing = summary.clone();
    }
    if let Some((token, mut asset)) = cover_asset {
        asset.generation = session_generation;
        project.assets.insert(token, asset);
    }
    drop(guard);
    drop(_file_guard);
    super::sync::schedule_sync_after_save(app, root);
    Ok(summary)
}

#[tauri::command]
pub fn reveal_article(
    project_id: String,
    session_generation: u64,
    article_id: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let path = state.with_project(&project_id, Some(session_generation), |project| {
        Ok(project.article(&article_id)?.canonical_path.clone())
    })?;
    #[cfg(windows)]
    {
        crate::platform::silent_command("explorer.exe")
            .arg(format!("/select,{}", path.display()))
            .spawn()
            .map_err(|error| AppError::io("在文件夹中显示文章失败", error))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|error| AppError::io("在 Finder 中显示文章失败", error))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        open::that_detached(path.parent().unwrap_or(Path::new(".")))
            .map_err(|error| AppError::io("打开文章文件夹失败", error))?;
    }
    Ok(())
}

fn ensure_article_action_available(state: &AppState) -> AppResult<()> {
    let available = state
        .task_cancellations
        .lock()
        .map_err(|_| AppError::new("state_poisoned", "任务状态不可用。", false))?
        .is_empty();
    if available {
        Ok(())
    } else {
        Err(AppError::new(
            "task_conflict",
            "项目任务正在运行，请完成后再操作文章。",
            true,
        ))
    }
}

fn open_project_path(state: &AppState, path: &Path) -> AppResult<OpenProjectResult> {
    let (root, name, warnings) = validate_hexo_root(path)?;
    let sync = super::sync::sync_before_open(state, &root);
    let file_lock = state.project_file_lock(&root)?;
    let _file_guard = file_lock.lock().map_err(|_| AppError::session_expired())?;
    let (articles, records, mut assets) = scan_articles(&root)?;
    let generation = state.next_generation();
    for asset in assets.values_mut() {
        asset.generation = generation;
    }
    super::images::recover_editor_image_assets(state, &records, &mut assets, generation)?;
    let session = ProjectSession {
        id: AppState::new_project_id(),
        generation,
        name,
        root: root.clone(),
        warnings,
        article_summaries: articles.clone(),
        articles: records,
        assets,
        remote_assets: Default::default(),
    };
    let view = session.view();
    remember_recent_project(state, &root, &view.name)?;
    state
        .pending_sync_rescans
        .lock()
        .map_err(|_| AppError::session_expired())?
        .remove(&root);
    cancel_project_work(state);
    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    *guard = Some(session);
    Ok(OpenProjectResult {
        session: view,
        articles,
        sync,
    })
}

pub fn rescan_project_after_sync(
    state: &AppState,
    root: &Path,
    project_id: &str,
    previous_generation: u64,
) -> AppResult<ProjectRescanResult> {
    state.with_project(project_id, Some(previous_generation), |project| {
        if project.root != root {
            return Err(AppError::session_expired());
        }
        Ok(())
    })?;
    let file_lock = state.project_file_lock(root)?;
    let _file_guard = file_lock.lock().map_err(|_| AppError::session_expired())?;
    rescan_project_after_sync_locked(state, root, project_id, previous_generation)
}

/// The caller holds the project's file lock through both disk changes and session replacement.
pub(super) fn rescan_project_after_sync_locked(
    state: &AppState,
    root: &Path,
    project_id: &str,
    previous_generation: u64,
) -> AppResult<ProjectRescanResult> {
    state.with_project(project_id, Some(previous_generation), |project| {
        if project.root != root {
            return Err(AppError::session_expired());
        }
        Ok(())
    })?;
    let (validated_root, name, warnings) = validate_hexo_root(root)?;
    let (mut articles, records, mut assets) = scan_articles(&validated_root)?;
    let generation = state.next_generation();
    for asset in assets.values_mut() {
        asset.generation = generation;
    }
    super::images::recover_editor_image_assets(state, &records, &mut assets, generation)?;
    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    let current = guard.as_ref().ok_or_else(AppError::session_expired)?;
    current.require_identity(project_id, Some(previous_generation))?;
    if current.root != validated_root {
        return Err(AppError::session_expired());
    }
    // Paths identify surviving articles across scans; session generation rejects stale requests.
    let previous_by_path = current
        .articles
        .values()
        .map(|record| (&record.canonical_path, record))
        .collect::<std::collections::HashMap<_, _>>();
    let mut stable_records = std::collections::HashMap::new();
    let mut stable_ids = std::collections::HashMap::new();
    for (scanned_id, mut record) in records {
        if let Some(previous) = previous_by_path.get(&record.canonical_path) {
            record.id = previous.id.clone();
            record.revision = previous.revision;
            stable_ids.insert(scanned_id, record.id.clone());
        }
        stable_records.insert(record.id.clone(), record);
    }
    for article in &mut articles {
        if let Some(stable_id) = stable_ids.get(&article.article_id) {
            article.article_id = stable_id.clone();
        }
    }
    // Unsaved editor drafts may still reference a cached asset absent from disk content.
    for (token, previous) in &current.assets {
        assets.entry(token.clone()).or_insert_with(|| {
            let mut asset = previous.clone();
            asset.generation = generation;
            asset
        });
    }
    // A rescan updates the active session only; it must not cancel unrelated jobs.
    *guard = Some(ProjectSession {
        id: project_id.to_string(),
        generation,
        name,
        root: validated_root,
        warnings,
        article_summaries: articles.clone(),
        articles: stable_records,
        assets,
        remote_assets: Default::default(),
    });
    Ok(ProjectRescanResult {
        project_id: project_id.to_string(),
        previous_generation,
        generation,
        articles,
    })
}

fn cancel_project_work(state: &AppState) {
    if let Ok(mut preview) = state.preview.lock() {
        if let Some(runtime) = preview.take() {
            if let Some(cancel) = runtime.cancellation {
                let _ = cancel.send(());
            }
        }
    }
    if let Ok(mut tasks) = state.task_cancellations.lock() {
        for (_, cancel) in tasks.drain() {
            let _ = cancel.send(());
        }
    }
}

fn clean_labels(values: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for value in values {
        for label in value.split([',', '，', '\n']) {
            let label = label.trim();
            if !label.is_empty() && !result.iter().any(|existing| existing == label) {
                result.push(label.to_string());
            }
        }
    }
    result
}

fn load_recent_records(state: &AppState) -> AppResult<Vec<RecentProjectRecord>> {
    if !state.recent_path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&state.recent_path)
        .map_err(|error| AppError::io("读取最近项目失败", error))?;
    if let Ok(records) = serde_json::from_str::<Vec<RecentProjectRecord>>(&content) {
        return Ok(records.into_iter().take(10).collect());
    }
    if let Ok(path) = serde_json::from_str::<String>(&content) {
        let path = PathBuf::from(path);
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("Hexo Project")
            .to_string();
        return Ok(vec![RecentProjectRecord {
            recent_id: Uuid::new_v4().to_string(),
            name,
            path,
            last_opened_at: Local::now().to_rfc3339(),
        }]);
    }
    Err(AppError::new(
        "recent_project_corrupt",
        "最近项目记录已损坏，可以在设置中清空后重新选择。",
        true,
    ))
}

fn save_recent_records(state: &AppState, records: &[RecentProjectRecord]) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(records)
        .map_err(|error| AppError::new("recent_project_serialize", error.to_string(), false))?;
    atomic_write(&state.recent_path, &bytes)
}

fn remember_recent_project(state: &AppState, path: &Path, name: &str) -> AppResult<()> {
    let canonical = path
        .canonicalize()
        .map_err(|error| AppError::io("验证最近项目失败", error))?;
    let mut records = load_recent_records(state).unwrap_or_default();
    let existing_id = records
        .iter()
        .find(|record| record.path == canonical)
        .map(|record| record.recent_id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    records.retain(|record| record.path != canonical);
    records.insert(
        0,
        RecentProjectRecord {
            recent_id: existing_id,
            name: name.to_string(),
            path: canonical,
            last_opened_at: Local::now().to_rfc3339(),
        },
    );
    records.truncate(10);
    save_recent_records(state, &records)
}

fn recent_available(path: &Path) -> bool {
    path.is_dir()
        && path.join("source").join("_posts").is_dir()
        && (path.join("_config.yml").is_file() || path.join("package.json").is_file())
}

fn backup_article(path: &Path) -> AppResult<()> {
    let parent = path.parent().unwrap_or(Path::new("."));
    let backup_dir = parent.join(".hlex-backups");
    fs::create_dir_all(&backup_dir).map_err(|error| AppError::io("创建文章备份目录失败", error))?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("article.md");
    let target = backup_dir.join(format!(
        "{}.{}.bak",
        file_name,
        Local::now().format("%Y%m%d-%H%M%S-%3f")
    ));
    fs::copy(path, target)
        .map(|_| ())
        .map_err(|error| AppError::io("备份文章失败", error))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn recent_projects_are_deduplicated_sorted_and_limited() {
        let temp = TempDir::new().unwrap();
        let config_dir = temp.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        let state = AppState::new(&config_dir);
        for index in 0..12 {
            let project = temp.path().join(format!("project-{index}"));
            fs::create_dir_all(project.join("source/_posts")).unwrap();
            fs::write(project.join("_config.yml"), "title: test").unwrap();
            remember_recent_project(&state, &project, &format!("项目 {index}")).unwrap();
        }
        let records = load_recent_records(&state).unwrap();
        assert_eq!(records.len(), 10);
        assert_eq!(records[0].name, "项目 11");
        assert!(recent_available(&records[0].path));

        let original_id = records[0].recent_id.clone();
        remember_recent_project(&state, &records[0].path, "重命名项目").unwrap();
        let records = load_recent_records(&state).unwrap();
        assert_eq!(records.len(), 10);
        assert_eq!(records[0].recent_id, original_id);
        assert_eq!(records[0].name, "重命名项目");
    }

    #[test]
    fn article_metadata_rewrite_updates_title_folder_and_keeps_body() {
        let source = "---\ntitle: Old\nimgbed_folder: blog/Old\ntags:\n  - Rust\n---\n\n# Body\n";
        let parsed = parse_front_matter(source);
        let mut attributes = parsed.attributes;
        let object = attributes.as_object_mut().unwrap();
        object.insert("title".into(), serde_json::Value::String("New".into()));
        object.insert(
            "imgbed_folder".into(),
            serde_json::Value::String(article_asset_folder("New")),
        );
        let yaml = serde_yaml::to_string(&attributes).unwrap();
        let rewritten = format!(
            "---\n{}---\n{}",
            yaml.trim_start_matches("---\n"),
            parsed.body
        );
        let result = parse_front_matter(&rewritten);
        assert_eq!(result.attributes["title"], "New");
        assert_eq!(result.attributes["imgbed_folder"], "blog/New");
        assert!(result.body.contains("# Body"));
    }
}
