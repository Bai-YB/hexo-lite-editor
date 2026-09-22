use crate::{
    app::AppState,
    domain::{AppError, AppResult, UpdateErrorStage, UpdateSnapshot, UpdateStatus},
};
use std::ffi::OsString;
use std::path::Path;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::{Update, UpdaterExt};

const UPDATE_CHECK_RETRY_DELAY_MS: u64 = 750;

/// Builds the NSIS `/D=<directory>` argument used to keep an update in place.
///
/// NSIS only accepts `/D=` unquoted and as the final command line argument,
/// which is what [`tauri_plugin_updater`] produces for `installer_args`.
fn install_dir_argument(directory: &Path) -> Option<OsString> {
    if directory.as_os_str().is_empty() {
        return None;
    }
    Some(OsString::from(format!("/D={}", directory.display())))
}

/// The directory the running executable lives in, as an installer argument.
///
/// A portable copy therefore keeps updating its own folder instead of a fixed
/// location, and an installation keeps the directory that was chosen during the
/// first install (or the installer default). Only Windows installers read these
/// arguments, so every other platform simply ignores them.
fn in_place_installer_args() -> Vec<OsString> {
    std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(Path::to_path_buf))
        .and_then(|directory| install_dir_argument(&directory))
        .into_iter()
        .collect()
}

/// Windows updates are shipped as the NSIS setup executable, which is the only
/// artifact that understands the `/D=` argument.
fn is_nsis_setup(url: &url::Url) -> bool {
    url.path().to_ascii_lowercase().ends_with("-setup.exe")
}

fn build_updater(app: &AppHandle, in_place: bool) -> Result<tauri_plugin_updater::Updater, String> {
    let mut builder = app.updater_builder();
    if in_place {
        builder = builder.installer_args(in_place_installer_args());
    }
    builder.build().map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadEvent {
    Chunk { bytes: u64, total: Option<u64> },
    Verifying,
    Finished,
    Failed,
}

fn apply_download_event(snapshot: &mut UpdateSnapshot, event: DownloadEvent) {
    match event {
        DownloadEvent::Chunk { bytes, total } => {
            snapshot.status = UpdateStatus::Downloading;
            snapshot.downloaded_bytes = Some(snapshot.downloaded_bytes.unwrap_or(0) + bytes);
            snapshot.total_bytes = total.filter(|total| *total > 0).or(snapshot.total_bytes);
            if snapshot.downloaded_bytes > snapshot.total_bytes {
                snapshot.total_bytes = None;
            }
        }
        DownloadEvent::Verifying => {
            snapshot.status = UpdateStatus::Verifying;
            snapshot.total_bytes = snapshot.downloaded_bytes;
        }
        DownloadEvent::Finished => {
            snapshot.status = UpdateStatus::Downloaded;
            if let Some(total) = snapshot.total_bytes {
                snapshot.downloaded_bytes = Some(total);
            }
        }
        DownloadEvent::Failed => {
            snapshot.error_stage = Some(if snapshot.status == UpdateStatus::Verifying {
                UpdateErrorStage::Verify
            } else {
                UpdateErrorStage::Download
            });
            snapshot.status = UpdateStatus::Error;
        }
    }
}

fn manifest_asset_size(manifest: &serde_json::Value, asset_url: &str) -> Option<u64> {
    // Match the exact updater asset selected by Tauri, including universal macOS builds.
    manifest
        .get("platforms")?
        .as_object()?
        .values()
        .find_map(|entry| {
            if entry.get("url")?.as_str()? != asset_url {
                return None;
            }
            entry.get("size")?.as_u64().filter(|size| *size > 0)
        })
}

fn initial(app: &AppHandle) -> UpdateSnapshot {
    UpdateSnapshot {
        current_version: crate::app::version::display_version(
            &app.package_info().version.to_string(),
        ),
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
    check_for_update_with(app, false).await
}

async fn check_for_update_with(app: &AppHandle, in_place: bool) -> Result<Option<Update>, String> {
    let first = build_updater(app, in_place)?.check().await;
    match first {
        Ok(update) => Ok(update),
        Err(error) if is_transient_update_error(&error.to_string()) => {
            let first_message = error.to_string();
            tokio::time::sleep(std::time::Duration::from_millis(
                UPDATE_CHECK_RETRY_DELAY_MS,
            ))
            .await;
            build_updater(app, in_place)?
                .check()
                .await
                .map_err(|error| format!("{error}（已自动重试一次；首次失败：{first_message}）"))
        }
        Err(error) => Err(error.to_string()),
    }
}

/// Resolves the update that is about to be downloaded and installed.
///
/// Windows installers are asked to update the directory the application is
/// currently running from. Updater artifacts that are not the NSIS setup
/// executable (for example an MSI package) take MSI arguments instead, so they
/// are resolved without the NSIS-only `/D=` flag.
async fn resolve_install_update(app: &AppHandle) -> Result<Option<Update>, String> {
    match check_for_update_with(app, true).await? {
        Some(update) if is_nsis_setup(&update.download_url) => Ok(Some(update)),
        Some(_) => check_for_update_with(app, false).await,
        None => Ok(None),
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
            snapshot.latest_version = Some(crate::app::version::display_version(&update.version));
            snapshot.release_notes = update.body.clone();
            snapshot.release_date = update.date.map(|date| date.to_string());
            snapshot.asset_download_url = Some(update.download_url.to_string());
            snapshot.total_bytes =
                manifest_asset_size(&update.raw_json, update.download_url.as_str());
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

    let update = resolve_install_update(&app).await.map_err(|error| {
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
    snapshot.latest_version = Some(crate::app::version::display_version(&update.version));
    snapshot.release_notes = update.body.clone();
    snapshot.release_date = update.date.map(|date| date.to_string());
    snapshot.asset_download_url = Some(update.download_url.to_string());
    snapshot.total_bytes = manifest_asset_size(&update.raw_json, update.download_url.as_str());
    store(&app, snapshot.clone());
    let progress_app = app.clone();
    let progress_snapshot = std::sync::Arc::new(std::sync::Mutex::new(snapshot));
    let progress_state = progress_snapshot.clone();
    let verify_app = app.clone();
    let verify_state = progress_snapshot.clone();
    let mut last_emit = std::time::Instant::now();
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
                    if last_emit.elapsed() >= std::time::Duration::from_millis(120) {
                        store(&progress_app, value.clone());
                        last_emit = std::time::Instant::now();
                    }
                }
            },
            move || {
                if let Ok(mut value) = verify_state.lock() {
                    apply_download_event(&mut value, DownloadEvent::Verifying);
                    store(&verify_app, value.clone());
                }
            },
        )
        .await
        .map_err(|error| {
            let mut current = progress_snapshot
                .lock()
                .map(|value| value.clone())
                .unwrap_or_else(|_| initial(&app));
            apply_download_event(&mut current, DownloadEvent::Failed);
            let stage = current
                .error_stage
                .clone()
                .unwrap_or(UpdateErrorStage::Download);
            failure(&app, current, stage, error)
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

    #[test]
    fn manifest_size_matches_the_exact_selected_asset() {
        let manifest = serde_json::json!({ "platforms": {
            "windows-x86_64": { "url": "https://example.com/app.exe", "size": 200 },
            "darwin-aarch64": { "url": "https://example.com/app.tar.gz", "size": 300 }
        }});
        assert_eq!(
            manifest_asset_size(&manifest, "https://example.com/app.exe"),
            Some(200)
        );
        assert_eq!(
            manifest_asset_size(&manifest, "https://example.com/other.exe"),
            None
        );
        assert_eq!(
            manifest_asset_size(&serde_json::json!({}), "https://example.com/app.exe"),
            None
        );
    }

    #[test]
    fn missing_content_length_uses_size_until_actual_bytes_exceed_it() {
        let mut value = snapshot();
        value.total_bytes = Some(100);
        apply_download_event(
            &mut value,
            DownloadEvent::Chunk {
                bytes: 40,
                total: None,
            },
        );
        assert_eq!(value.total_bytes, Some(100));
        apply_download_event(
            &mut value,
            DownloadEvent::Chunk {
                bytes: 70,
                total: None,
            },
        );
        assert_eq!(value.total_bytes, None);
        apply_download_event(&mut value, DownloadEvent::Verifying);
        assert_eq!(value.total_bytes, Some(110));
        assert_eq!(value.status, UpdateStatus::Verifying);
        apply_download_event(&mut value, DownloadEvent::Failed);
        assert_eq!(value.error_stage, Some(UpdateErrorStage::Verify));
    }

    #[test]
    fn old_update_settings_keep_download_manual() {
        let old: crate::domain::UpdateConfig =
            serde_json::from_value(serde_json::json!({ "checkOnStart": true })).unwrap();
        assert!(old.check_on_start);
        assert!(!old.auto_download);
        let enabled: crate::domain::UpdateConfig = serde_json::from_value(
            serde_json::json!({ "checkOnStart": true, "autoDownload": true }),
        )
        .unwrap();
        assert!(enabled.auto_download);
    }

    #[test]
    fn installer_directory_argument_keeps_updates_in_place() {
        let argument = install_dir_argument(Path::new(r"C:\Users\me\Hexo Lite Editor")).unwrap();
        // NSIS requires the unquoted `/D=` form, even when the path has spaces.
        assert_eq!(
            argument.to_string_lossy(),
            r"/D=C:\Users\me\Hexo Lite Editor"
        );
        assert_eq!(install_dir_argument(Path::new("")), None);
    }

    #[test]
    fn only_the_nsis_setup_artifact_receives_installer_directory_arguments() {
        let parse = |value: &str| url::Url::parse(value).unwrap();
        assert!(is_nsis_setup(&parse(
            "https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.2/Hexo-Lite-Editor_1.0.6.5.2_windows-x64-setup.exe"
        )));
        assert!(!is_nsis_setup(&parse(
            "https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.2/Hexo-Lite-Editor_1.0.6.5.2_windows-x64.msi"
        )));
        assert!(!is_nsis_setup(&parse(
            "https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.2/Hexo-Lite-Editor_1.0.6.5.2_windows-x64-portable.zip"
        )));
        assert!(!is_nsis_setup(&parse(
            "https://github.com/Bai-YB/hexo-lite-editor/releases/download/v1.0.6.5.2/Hexo-Lite-Editor_1.0.6.5.2_macos-universal.app.tar.gz"
        )));
    }
}
