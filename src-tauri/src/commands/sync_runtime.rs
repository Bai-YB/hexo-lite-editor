use crate::{
    app::AppState,
    domain::{AppError, AppResult},
};
use std::{
    cell::RefCell,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::{Emitter, Manager};

type ProgressCallback = dyn Fn(&str, &str, Option<usize>, Option<usize>) + Send + Sync;

#[derive(Clone)]
struct Operation {
    cancelled: Arc<AtomicBool>,
    deadline: Instant,
    progress: Arc<ProgressCallback>,
}

thread_local! { static CURRENT: RefCell<Option<Operation>> = const { RefCell::new(None) }; }

pub(super) fn check() -> AppResult<()> {
    CURRENT.with(|slot| {
        if let Some(operation) = slot.borrow().as_ref() {
            if operation.cancelled.load(Ordering::Relaxed) {
                return Err(AppError::new(
                    "sync_cancelled",
                    "同步已停止。已完成的远端提交会保留；重试时将重新检查两端状态。",
                    true,
                ));
            }
            if Instant::now() >= operation.deadline {
                return Err(AppError::new(
                    "sync_timeout",
                    "本轮同步已超过 10 分钟并停止，请检查网络后重试。已传输的文件可在重试时复用。",
                    true,
                ));
            }
        }
        Ok(())
    })
}

pub(super) fn progress(phase: &str, message: &str, completed: Option<usize>, total: Option<usize>) {
    CURRENT.with(|slot| {
        if let Some(operation) = slot.borrow().as_ref() {
            (operation.progress)(phase, message, completed, total);
        }
    });
}

pub(super) async fn run<R, T, F>(
    app: tauri::AppHandle<R>,
    project_id: String,
    generation: u64,
    work: F,
) -> AppResult<T>
where
    R: tauri::Runtime,
    T: Send + 'static,
    F: FnOnce(&AppState) -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        state.with_project(&project_id, Some(generation), |_| Ok(()))?;
        let key = project_id.clone();
        let cancelled = Arc::new(AtomicBool::new(false));
        {
            let mut operations = state
                .sync_operations
                .lock()
                .map_err(|_| AppError::invalid("同步任务列表不可用。"))?;
            if operations.contains_key(&key) {
                return Err(AppError::new(
                    "sync_busy",
                    "此项目已有同步操作，请等待完成或点击停止。",
                    true,
                ));
            }
            operations.insert(key.clone(), cancelled.clone());
        }
        let progress_app = app.clone();
        let progress_id = project_id.clone();
        let callback =
            move |phase: &str, message: &str, completed: Option<usize>, total: Option<usize>| {
                let event = super::sync::ContentSyncEvent {
                    project_id: Some(progress_id.clone()),
                    session_generation: Some(generation),
                    phase: phase.to_string(),
                    status: super::sync::ContentSyncStatus::Checking,
                    message: Some(message.to_string()),
                    completed_files: completed,
                    total_files: total,
                };
                if let Ok(mut snapshots) = progress_app
                    .state::<AppState>()
                    .sync_operation_progress
                    .lock()
                {
                    snapshots.insert(progress_id.clone(), event.clone());
                }
                let _ = progress_app.emit("content-sync-phase", event);
            };
        CURRENT.with(|slot| {
            *slot.borrow_mut() = Some(Operation {
                cancelled: cancelled.clone(),
                deadline: Instant::now() + Duration::from_secs(600),
                progress: Arc::new(callback),
            })
        });
        progress("preparing", "正在准备同步操作。", None, None);
        struct Cleanup<'a> {
            state: &'a AppState,
            key: String,
        }
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                CURRENT.with(|slot| *slot.borrow_mut() = None);
                if let Ok(mut operations) = self.state.sync_operations.lock() {
                    operations.remove(&self.key);
                }
                if let Ok(mut snapshots) = self.state.sync_operation_progress.lock() {
                    snapshots.remove(&self.key);
                }
            }
        }
        let _cleanup = Cleanup { state: &state, key };
        let result = work(&state);
        let message = result
            .as_ref()
            .err()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "同步操作已结束。".to_string());
        let _ = app.emit(
            "content-sync-phase",
            super::sync::ContentSyncEvent {
                project_id: Some(project_id),
                session_generation: Some(generation),
                // Ending a connection test or returning a conflict does not mean content is synced.
                phase: if result.is_err() {
                    "failed"
                } else {
                    "operationFinished"
                }
                .to_string(),
                status: if result.is_err() {
                    super::sync::ContentSyncStatus::Error
                } else {
                    super::sync::ContentSyncStatus::Checking
                },
                message: Some(message),
                completed_files: None,
                total_files: None,
            },
        );
        result
    })
    .await
    .map_err(|error| {
        AppError::new(
            "sync_worker_failed",
            format!("同步后台任务失败：{error}"),
            true,
        )
    })?
}

#[tauri::command]
pub fn cancel_content_sync(
    project_id: String,
    session_generation: u64,
    state: tauri::State<'_, AppState>,
) -> AppResult<bool> {
    let _ = session_generation;
    // Never wait for the project write lock: a rescan may hold it while cancellation is requested.
    let key = project_id;
    let operations = state
        .sync_operations
        .lock()
        .map_err(|_| AppError::invalid("同步任务列表不可用。"))?;
    if let Some(cancelled) = operations.get(&key) {
        cancelled.store(true, Ordering::Relaxed);
        return Ok(true);
    }
    Ok(false)
}

#[tauri::command]
pub fn get_content_sync_progress(
    project_id: String,
    state: tauri::State<'_, AppState>,
) -> Option<super::sync::ContentSyncEvent> {
    state
        .sync_operation_progress
        .lock()
        .ok()?
        .get(&project_id)
        .cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_worker_reports_progress_cancels_without_project_lock_and_releases_operation() {
        let temp = tempfile::tempdir().unwrap();
        let state = AppState::new(temp.path());
        *state.project.write().unwrap() = Some(crate::app::ProjectSession {
            id: "sync-worker".into(),
            generation: 1,
            name: "Test".into(),
            root: temp.path().into(),
            warnings: vec![],
            article_summaries: vec![],
            articles: Default::default(),
            assets: Default::default(),
            remote_assets: Default::default(),
        });
        let app = tauri::test::mock_builder()
            .manage(state)
            .build(tauri::test::mock_context(tauri::test::noop_assets()))
            .unwrap();
        let handle = app.handle().clone();
        let (started, ready) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            tauri::async_runtime::block_on(run(
                handle,
                "sync-worker".into(),
                1,
                move |_| -> AppResult<()> {
                    progress("uploading", "a.md", Some(1), Some(3));
                    started.send(()).unwrap();
                    loop {
                        check()?;
                        std::thread::sleep(Duration::from_millis(5));
                    }
                },
            ))
        });
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        let progress =
            get_content_sync_progress("sync-worker".into(), app.state::<AppState>()).unwrap();
        assert_eq!(progress.completed_files, Some(1));
        let state = app.state::<AppState>();
        let _project_write_lock = state.project.write().unwrap();
        assert!(cancel_content_sync("sync-worker".into(), 1, app.state::<AppState>()).unwrap());
        assert_eq!(worker.join().unwrap().unwrap_err().code, "sync_cancelled");
        assert!(state.sync_operations.lock().unwrap().is_empty());
        assert!(get_content_sync_progress("sync-worker".into(), app.state::<AppState>()).is_none());
    }
    #[test]
    fn cancellation_and_deadline_interrupt_before_the_next_write() {
        let cancelled = Arc::new(AtomicBool::new(false));
        CURRENT.with(|slot| {
            *slot.borrow_mut() = Some(Operation {
                cancelled: cancelled.clone(),
                deadline: Instant::now() + Duration::from_secs(60),
                progress: Arc::new(|_, _, _, _| {}),
            })
        });
        assert!(check().is_ok());
        cancelled.store(true, Ordering::Relaxed);
        assert_eq!(check().unwrap_err().code, "sync_cancelled");
        cancelled.store(false, Ordering::Relaxed);
        CURRENT.with(|slot| {
            slot.borrow_mut().as_mut().unwrap().deadline = Instant::now() - Duration::from_secs(1)
        });
        assert_eq!(check().unwrap_err().code, "sync_timeout");
        CURRENT.with(|slot| *slot.borrow_mut() = None);
        assert!(check().is_ok());
    }
}
