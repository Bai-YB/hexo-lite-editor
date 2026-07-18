use crate::{
    app::AppState,
    commands::{start_preview_internal, stop_preview_internal},
    data::{append_task_event, finish_task_log, load_config, start_task_log},
    domain::{
        AppError, AppResult, PreviewServerState, TaskEvent, TaskEventKind, TaskReceipt, TaskStream,
        TaskType,
    },
    engine::{build_task_steps, TaskStep},
};
use chrono::Local;
use process_wrap::tokio::*;
use std::{
    path::Path,
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    sync::oneshot,
};
use uuid::Uuid;

#[tauri::command]
pub async fn start_task(
    app: AppHandle,
    project_id: String,
    kind: TaskType,
    state: State<'_, AppState>,
) -> AppResult<TaskReceipt> {
    let (root, generation, project_name) = state.with_project(&project_id, None, |project| {
        Ok((
            project.root.clone(),
            project.generation,
            project.name.clone(),
        ))
    })?;
    if kind == TaskType::ServerStart {
        start_preview_internal(&app, &project_id, generation).await?;
        return Ok(TaskReceipt {
            task_id: format!("preview-{}", Uuid::new_v4()),
        });
    }
    if kind == TaskType::ServerStop {
        stop_preview_internal(&app, &project_id, generation).await?;
        return Ok(TaskReceipt {
            task_id: format!("preview-stop-{}", Uuid::new_v4()),
        });
    }

    let config = load_config(&state)?.config;
    let steps = build_task_steps(kind, &config, &root)?;
    let modifies_project = !matches!(kind, TaskType::GitStatus);
    let preview_was_running = if modifies_project {
        state
            .preview
            .lock()
            .map_err(|_| AppError::new("state_poisoned", "预览服务状态不可用。", false))?
            .as_ref()
            .is_some_and(|runtime| {
                runtime.view.project_id == project_id
                    && runtime.view.session_generation == generation
                    && matches!(
                        runtime.view.state,
                        PreviewServerState::Starting | PreviewServerState::Running
                    )
            })
    } else {
        false
    };

    let task_id = Uuid::new_v4().to_string();
    let (cancel_tx, cancel_rx) = oneshot::channel();
    {
        let mut tasks = state
            .task_cancellations
            .lock()
            .map_err(|_| AppError::new("state_poisoned", "任务状态不可用。", false))?;
        if modifies_project && !tasks.is_empty() {
            return Err(AppError::new(
                "task_conflict",
                "当前已有项目任务正在运行。",
                true,
            ));
        }
        tasks.insert(task_id.clone(), cancel_tx);
    }
    start_task_log(&state, &task_id, &project_name, kind)?;
    spawn_task(
        app,
        task_id.clone(),
        project_id,
        generation,
        root,
        steps,
        cancel_rx,
        preview_was_running || (modifies_project && config.hexo.auto_start_preview),
    );
    Ok(TaskReceipt { task_id })
}

#[tauri::command]
pub fn cancel_task(task_id: String, state: State<'_, AppState>) -> AppResult<()> {
    let cancel = state
        .task_cancellations
        .lock()
        .map_err(|_| AppError::new("state_poisoned", "任务状态不可用。", false))?
        .remove(&task_id);
    if let Some(cancel) = cancel {
        let _ = cancel.send(());
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn spawn_task(
    app: AppHandle,
    task_id: String,
    project_id: String,
    generation: u64,
    root: std::path::PathBuf,
    steps: Vec<TaskStep>,
    cancel_rx: oneshot::Receiver<()>,
    restore_preview: bool,
) {
    tauri::async_runtime::spawn(async move {
        let sequence = Arc::new(AtomicU64::new(0));
        emit_event(
            &app,
            new_event(
                &task_id,
                &project_id,
                next_sequence(&sequence),
                TaskEventKind::Queued,
            ),
        );
        if restore_preview {
            if let Err(error) = stop_preview_internal(&app, &project_id, generation).await {
                emit_log(
                    &app,
                    &task_id,
                    &project_id,
                    &sequence,
                    "停止预览",
                    TaskStream::Stderr,
                    &error.message,
                );
                let mut finished = new_event(
                    &task_id,
                    &project_id,
                    next_sequence(&sequence),
                    TaskEventKind::Finished,
                );
                finished.success = Some(false);
                emit_event(&app, finished);
                let state = app.state::<AppState>();
                let _ = finish_task_log(&state, &task_id, false);
                if let Ok(mut tasks) = state.task_cancellations.lock() {
                    tasks.remove(&task_id);
                }
                return;
            }
        }
        let mut success = true;
        let mut cancel_rx = cancel_rx;
        for step in steps {
            emit_step(
                &app,
                &task_id,
                &project_id,
                &sequence,
                &step,
                TaskEventKind::StepStarted,
                None,
                None,
            );
            match run_step(
                &app,
                &task_id,
                &project_id,
                &sequence,
                &root,
                &step,
                &mut cancel_rx,
            )
            .await
            {
                Ok(code) => {
                    let step_success = code == Some(0);
                    emit_step(
                        &app,
                        &task_id,
                        &project_id,
                        &sequence,
                        &step,
                        TaskEventKind::StepFinished,
                        Some(step_success),
                        code,
                    );
                    if !step_success {
                        success = false;
                        break;
                    }
                }
                Err(error) => {
                    emit_log(
                        &app,
                        &task_id,
                        &project_id,
                        &sequence,
                        step.name,
                        TaskStream::Stderr,
                        &error,
                    );
                    success = false;
                    break;
                }
            }
        }
        let mut finished = new_event(
            &task_id,
            &project_id,
            next_sequence(&sequence),
            TaskEventKind::Finished,
        );
        finished.success = Some(success);
        emit_event(&app, finished);
        let state = app.state::<AppState>();
        let _ = finish_task_log(&state, &task_id, success);
        if let Ok(mut tasks) = state.task_cancellations.lock() {
            tasks.remove(&task_id);
        }
        if restore_preview {
            let _ = start_preview_internal(&app, &project_id, generation).await;
        }
    });
}

#[allow(clippy::too_many_arguments)]
async fn run_step(
    app: &AppHandle,
    task_id: &str,
    project_id: &str,
    sequence: &Arc<AtomicU64>,
    root: &Path,
    step: &TaskStep,
    cancel_rx: &mut oneshot::Receiver<()>,
) -> Result<Option<i32>, String> {
    let mut command = CommandWrap::with_new(&step.program, |command| {
        command
            .args(&step.args)
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });
    #[cfg(windows)]
    command.wrap(JobObject);
    #[cfg(unix)]
    command.wrap(ProcessGroup::leader());
    command.wrap(KillOnDrop);
    let mut child = command
        .spawn()
        .map_err(|error| format!("无法启动 {}：{error}", step.name))?;

    let stdout_task = child.stdout().take().map(|stdout| {
        spawn_log_reader(
            app.clone(),
            task_id.to_string(),
            project_id.to_string(),
            sequence.clone(),
            step.name.to_string(),
            TaskStream::Stdout,
            stdout,
        )
    });
    let stderr_task = child.stderr().take().map(|stderr| {
        spawn_log_reader(
            app.clone(),
            task_id.to_string(),
            project_id.to_string(),
            sequence.clone(),
            step.name.to_string(),
            TaskStream::Stderr,
            stderr,
        )
    });

    let result = tokio::select! {
        status = child.wait() => status
            .map(|status| status.code())
            .map_err(|error| format!("等待 {} 结束失败：{error}", step.name)),
        _ = cancel_rx => {
            Box::into_pin(child.kill())
                .await
                .map_err(|error| format!("取消 {} 失败：{error}", step.name))?;
            Err(format!("{} 已取消", step.name))
        }
    };
    if let Some(task) = stdout_task {
        let _ = task.await;
    }
    if let Some(task) = stderr_task {
        let _ = task.await;
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn spawn_log_reader<R>(
    app: AppHandle,
    task_id: String,
    project_id: String,
    sequence: Arc<AtomicU64>,
    step: String,
    stream: TaskStream,
    reader: R,
) -> tauri::async_runtime::JoinHandle<()>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    emit_log(&app, &task_id, &project_id, &sequence, &step, stream, &line)
                }
                Ok(None) => break,
                Err(error) => {
                    emit_log(
                        &app,
                        &task_id,
                        &project_id,
                        &sequence,
                        &step,
                        TaskStream::Stderr,
                        &format!("读取进程输出失败：{error}"),
                    );
                    break;
                }
            }
        }
    })
}

#[allow(clippy::too_many_arguments)]
fn emit_step(
    app: &AppHandle,
    task_id: &str,
    project_id: &str,
    sequence: &Arc<AtomicU64>,
    step: &TaskStep,
    kind: TaskEventKind,
    success: Option<bool>,
    exit_code: Option<i32>,
) {
    let mut event = new_event(task_id, project_id, next_sequence(sequence), kind);
    event.step = Some(step.name.to_string());
    event.success = success;
    event.exit_code = exit_code;
    emit_event(app, event);
}

fn emit_log(
    app: &AppHandle,
    task_id: &str,
    project_id: &str,
    sequence: &Arc<AtomicU64>,
    step: &str,
    stream: TaskStream,
    line: &str,
) {
    let mut event = new_event(
        task_id,
        project_id,
        next_sequence(sequence),
        TaskEventKind::Log,
    );
    event.step = Some(step.to_string());
    event.stream = Some(stream);
    event.line = Some(line.to_string());
    emit_event(app, event);
}

fn new_event(task_id: &str, project_id: &str, sequence: u64, kind: TaskEventKind) -> TaskEvent {
    TaskEvent {
        task_id: task_id.to_string(),
        project_id: project_id.to_string(),
        sequence,
        kind,
        step: None,
        stream: None,
        line: None,
        success: None,
        exit_code: None,
        timestamp: Local::now().to_rfc3339(),
    }
}

fn next_sequence(sequence: &AtomicU64) -> u64 {
    sequence.fetch_add(1, Ordering::SeqCst)
}

fn emit_event(app: &AppHandle, mut event: TaskEvent) {
    let state = app.state::<AppState>();
    let _ = append_task_event(&state, &mut event);
    let _ = app.emit("task-event", event);
}
