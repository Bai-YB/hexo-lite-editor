use crate::{
    app::AppState,
    domain::{AppError, AppResult, UpdateErrorStage, UpdateSnapshot, UpdateStatus},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

const UPDATE_CHECK_RETRY_DELAY_MS: u64 = 750;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadEvent {
    Chunk { bytes: u64, total: Option<u64> },
    Finished,
    Failed,
}

fn apply_download_event(snapshot: &mut UpdateSnapshot, event: DownloadEvent) {
    match event {
        DownloadEvent::Chunk { bytes, total } => {
            snapshot.status = UpdateStatus::Downloading;
            snapshot.downloaded_bytes = Some(snapshot.downloaded_bytes.unwrap_or(0) + bytes);
            snapshot.total_bytes = total;
        }
        DownloadEvent::Finished => {
            snapshot.status = UpdateStatus::Downloaded;
            if let Some(total) = snapshot.total_bytes {
                snapshot.downloaded_bytes = Some(total);
            }
        }
        DownloadEvent::Failed => {
            snapshot.status = UpdateStatus::Error;
            snapshot.error_stage = Some(UpdateErrorStage::Download);
        }
    }
}

fn initial(app: &AppHandle) -> UpdateSnapshot {
    UpdateSnapshot {
        current_version: app.package_info().version.to_string(),
        status: UpdateStatus::Idle,
        latest_version: None,
        release_notes: None,
        release_date: None,
        downloaded_bytes: None,
        total_bytes: None,
        error_stage: None,
        error_message: None,
        release_page_url: Some("https://github.com/Bai-YB/hexo-lite-editor/releases".into()),
        asset_download_url: None,
    }
}

fn store(app: &AppHandle, snapshot: UpdateSnapshot) {
    if let Ok(mut value) = app.state::<AppState>().updater.lock() {
        *value = Some(snapshot.clone());
    }
    let _ = app.emit("update-snapshot-changed", snapshot);
}

fn failure(
    app: &AppHandle,
    mut snapshot: UpdateSnapshot,
    stage: UpdateErrorStage,
    error: impl ToString,
) -> AppError {
    snapshot.status = UpdateStatus::Error;
    snapshot.error_stage = Some(stage);
    snapshot.error_message = Some(error.to_string());
    store(app, snapshot);
    AppError::new("update_failed", error.to_string(), true)
}

fn describe_update_check_error(error: impl ToString) -> String {
    let raw = error.to_string();
    let normalized = raw.to_ascii_lowercase();
    if normalized.contains("api.github.com/repos/bai-yb/hexo-lite-editor/releases/latest") {
        return "当前安装包仍在使用旧的 GitHub API 更新地址，无法检查签名更新。请安装包含 latest.json 更新清单的新版本。".to_string();
    }
    if normalized.contains("latest.json")
        && (normalized.contains("404") || normalized.contains("not found"))
    {
        return "更新发布尚未完成：GitHub Release 缺少 latest.json 签名更新清单。请稍后重试，或联系发布者完成对应版本的发布。".to_string();
    }
    if normalized.contains("403") || normalized.contains("forbidden") {
        return "GitHub 拒绝了更新检查请求（可能是网络代理、访问限制或 API 限流）。请检查网络后重试；应用不会安装未经签名的更新。".to_string();
    }
    raw
}

fn is_transient_update_error(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    [
        "sending request",
        "network",
        "timeout",
        "timed out",
        "connection reset",
        "status 429",
        "status 5",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

async fn check_for_update(app: &AppHandle) -> Result<Option<Update>, String> {
    let first = app
        .updater()
        .map_err(|error| error.to_string())?
        .check()
        .await;
    match first {
        Ok(update) => Ok(update),
        Err(error) if is_transient_update_error(&error.to_string()) => {
            let first_message = error.to_string();
            tokio::time::sleep(std::time::Duration::from_millis(
                UPDATE_CHECK_RETRY_DELAY_MS,
            ))
            .await;
            app.updater()
                .map_err(|error| error.to_string())?
                .check()
                .await
                .map_err(|error| format!("{error}（已自动重试一次；首次失败：{first_message}）"))
        }
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
pub fn get_update_snapshot(app: AppHandle, state: State<'_, AppState>) -> UpdateSnapshot {
    state
        .updater
        .lock()
        .ok()
        .and_then(|value| value.clone())
        .unwrap_or_else(|| initial(&app))
}

#[tauri::command]
pub async fn check_update(app: AppHandle) -> AppResult<UpdateSnapshot> {
    let state = app.state::<AppState>();
    let _operation = state
        .update_operation_lock
        .try_lock()
        .map_err(|_| AppError::new("update_busy", "更新任务正在运行，请稍后重试。", true))?;
    if state
        .downloaded_update
        .lock()
        .is_ok_and(|cached| cached.is_some())
    {
        return Ok(get_update_snapshot(app.clone(), app.state()));
    }
    let mut snapshot = initial(&app);
    snapshot.status = UpdateStatus::Checking;
    store(&app, snapshot.clone());
    match check_for_update(&app).await {
        Ok(Some(update)) => {
            snapshot.status = UpdateStatus::Available;
            snapshot.latest_version = Some(update.version.clone());
            snapshot.release_notes = update.body.clone();
            snapshot.release_date = update.date.map(|date| date.to_string());
            snapshot.asset_download_url = Some(update.download_url.to_string());
            store(&app, snapshot.clone());
            Ok(snapshot)
        }
        Ok(None) => {
            snapshot.status = UpdateStatus::UpToDate;
            store(&app, snapshot.clone());
            Ok(snapshot)
        }
        Err(error) => Err(failure(
            &app,
            snapshot,
            UpdateErrorStage::Check,
            describe_update_check_error(error),
        )),
    }
}

#[tauri::command]
pub async fn download_update(app: AppHandle) -> AppResult<UpdateSnapshot> {
    let state = app.state::<AppState>();
    let _operation = state
        .update_operation_lock
        .try_lock()
        .map_err(|_| AppError::new("update_busy", "更新任务正在运行，请稍后重试。", true))?;
    if state
        .downloaded_update
        .lock()
        .is_ok_and(|cached| cached.is_some())
    {
        return Ok(get_update_snapshot(app.clone(), app.state()));
    }
    let mut snapshot = get_update_snapshot(app.clone(), app.state());
    snapshot.status = UpdateStatus::Downloading;
    snapshot.downloaded_bytes = Some(0);
    snapshot.total_bytes = None;
    snapshot.error_stage = None;
    snapshot.error_message = None;
    store(&app, snapshot.clone());

    let update = check_for_update(&app).await.map_err(|error| {
        failure(
            &app,
            snapshot.clone(),
            UpdateErrorStage::Download,
            describe_update_check_error(error),
        )
    })?;
    let Some(update) = update else {
        snapshot = initial(&app);
        snapshot.status = UpdateStatus::UpToDate;
        store(&app, snapshot.clone());
        return Ok(snapshot);
    };
    snapshot.latest_version = Some(update.version.clone());
    snapshot.release_notes = update.body.clone();
    snapshot.release_date = update.date.map(|date| date.to_string());
    snapshot.asset_download_url = Some(update.download_url.to_string());
    let progress_app = app.clone();
    let progress_snapshot = std::sync::Arc::new(std::sync::Mutex::new(snapshot));
    let progress_state = progress_snapshot.clone();
    let bytes = update
        .download(
            move |chunk, total| {
                if let Ok(mut value) = progress_state.lock() {
                    apply_download_event(
                        &mut value,
                        DownloadEvent::Chunk {
                            bytes: chunk as u64,
                            total,
                        },
                    );
                    store(&progress_app, value.clone());
                }
            },
            || {},
        )
        .await
        .map_err(|error| {
            let mut current = progress_snapshot
                .lock()
                .map(|value| value.clone())
                .unwrap_or_else(|_| initial(&app));
            apply_download_event(&mut current, DownloadEvent::Failed);
            failure(&app, current, UpdateErrorStage::Download, error)
        })?;

    let mut final_snapshot = progress_snapshot
        .lock()
        .map(|value| value.clone())
        .unwrap_or_else(|_| initial(&app));
    final_snapshot.total_bytes = Some(bytes.len() as u64);
    apply_download_event(&mut final_snapshot, DownloadEvent::Finished);
    app.state::<AppState>()
        .downloaded_update
        .lock()
        .map_err(|_| {
            AppError::new(
                "update_cache_unavailable",
                "Update cache is unavailable.",
                true,
            )
        })?
        .replace((update, bytes));
    store(&app, final_snapshot.clone());
    Ok(final_snapshot)
}

#[tauri::command]
pub fn install_update(app: AppHandle) -> AppResult<()> {
    let state = app.state::<AppState>();
    let _operation = state
        .update_operation_lock
        .try_lock()
        .map_err(|_| AppError::new("update_busy", "更新任务正在运行，请稍后重试。", true))?;
    let mut snapshot = get_update_snapshot(app.clone(), app.state());
    let cached = app
        .state::<AppState>()
        .downloaded_update
        .lock()
        .map_err(|_| {
            AppError::new(
                "update_cache_unavailable",
                "Update cache is unavailable.",
                true,
            )
        })?
        .take()
        .ok_or_else(|| {
            AppError::new(
                "update_not_downloaded",
                "Download the update before installing it.",
                true,
            )
        })?;
    snapshot.status = UpdateStatus::Installing;
    snapshot.error_stage = None;
    snapshot.error_message = None;
    store(&app, snapshot.clone());
    if let Err(error) = cached.0.install(&cached.1) {
        if let Ok(mut value) = app.state::<AppState>().downloaded_update.lock() {
            *value = Some(cached);
        }
        return Err(failure(&app, snapshot, UpdateErrorStage::Install, error));
    }
    app.restart();
}

#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> AppResult<()> {
    download_update(app.clone()).await?;
    install_update(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> UpdateSnapshot {
        UpdateSnapshot {
            current_version: "1.0.6".into(),
            status: UpdateStatus::Available,
            latest_version: Some("1.0.7".into()),
            release_notes: Some("notes".into()),
            release_date: None,
            downloaded_bytes: None,
            total_bytes: None,
            error_stage: None,
            error_message: None,
            release_page_url: None,
            asset_download_url: None,
        }
    }

    #[test]
    fn progress_accumulates_and_finishes_at_total() {
        let mut value = snapshot();
        apply_download_event(
            &mut value,
            DownloadEvent::Chunk {
                bytes: 25,
                total: Some(100),
            },
        );
        apply_download_event(
            &mut value,
            DownloadEvent::Chunk {
                bytes: 75,
                total: Some(100),
            },
        );
        apply_download_event(&mut value, DownloadEvent::Finished);
        assert_eq!(value.status, UpdateStatus::Downloaded);
        assert_eq!(value.downloaded_bytes, Some(100));
        assert_eq!(value.release_notes.as_deref(), Some("notes"));
    }

    #[test]
    fn progress_supports_unknown_total_and_preserves_failure_stage() {
        let mut value = snapshot();
        apply_download_event(
            &mut value,
            DownloadEvent::Chunk {
                bytes: 64,
                total: None,
            },
        );
        assert_eq!(value.downloaded_bytes, Some(64));
        assert_eq!(value.total_bytes, None);
        apply_download_event(&mut value, DownloadEvent::Failed);
        assert_eq!(value.status, UpdateStatus::Error);
        assert_eq!(value.error_stage, Some(UpdateErrorStage::Download));
    }

    #[test]
    fn explains_legacy_api_and_missing_manifest_failures() {
        assert!(describe_update_check_error("error sending request for url (https://api.github.com/repos/Bai-YB/hexo-lite-editor/releases/latest)").contains("旧的 GitHub API"));
        assert!(
            describe_update_check_error("HTTP 404 latest.json not found")
                .contains("缺少 latest.json")
        );
    }
}
