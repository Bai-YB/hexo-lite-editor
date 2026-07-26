use crate::{app::AppState, domain::AppResult};
use tauri::State;

pub fn signal_background_shutdown(state: &AppState) {
    if let Ok(mut preview) = state.preview.lock() {
        if let Some(runtime) = preview.as_mut() {
            if let Some(cancel) = runtime.cancellation.take() {
                let _ = cancel.send(());
            }
        }
    }
    if let Ok(mut tasks) = state.task_cancellations.lock() {
        for (_, cancel) in tasks.drain() {
            let _ = cancel.send(());
        }
    }
    if let Ok(mut schedules) = state.sync_schedules.lock() {
        for (_, cancel) in schedules.drain() {
            let _ = cancel.send(());
        }
    }
}

#[tauri::command]
pub fn cleanup_before_exit(state: State<'_, AppState>) -> AppResult<()> {
    signal_background_shutdown(&state);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{PreviewServerState, PreviewServerView};
    use tempfile::TempDir;
    use tokio::sync::oneshot;

    #[test]
    fn shutdown_signals_are_idempotent() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path());
        let (preview_tx, mut preview_rx) = oneshot::channel();
        let (task_tx, mut task_rx) = oneshot::channel();
        *state.preview.lock().unwrap() = Some(crate::app::PreviewRuntime {
            view: PreviewServerView {
                project_id: "project".to_string(),
                session_generation: 1,
                state: PreviewServerState::Running,
                port: 4000,
                base_url: None,
                drafts_enabled: true,
                started_at: None,
                error: None,
            },
            cancellation: Some(preview_tx),
        });
        state
            .task_cancellations
            .lock()
            .unwrap()
            .insert("task".to_string(), task_tx);

        signal_background_shutdown(&state);
        signal_background_shutdown(&state);

        assert!(preview_rx.try_recv().is_ok());
        assert!(task_rx.try_recv().is_ok());
        assert!(state
            .preview
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|runtime| runtime.cancellation.is_none()));
        assert!(state.task_cancellations.lock().unwrap().is_empty());
    }
}
