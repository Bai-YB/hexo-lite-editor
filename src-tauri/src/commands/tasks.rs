use crate::{
    app::AppState,
    commands::{start_preview_internal, stop_preview_internal},
    data::{append_task_event, finish_task_log, load_config, start_task_log},
    domain::{
        AppError, AppResult, PreviewServerState, TaskEvent, TaskEventKind, TaskReceipt, TaskStream,
        TaskType,
    },
    engine::{build_task_steps, TaskStep},
    platform::command_path,
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
use walkdir::WalkDir;

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
    if kind == TaskType::Publish {
        ensure_no_pending_editor_images(&root)?;
        validate_publish_articles(&root)?;
    }
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

fn ensure_no_pending_editor_images(root: &Path) -> AppResult<()> {
    let source = root.join("source");
    if !source.is_dir() {
        return Ok(());
    }
    let pending_image = regex::Regex::new(
        r"!\[[^\]\r\n]*\]\(\s*(?:hlex-asset://localhost/|http://hlex-asset\.localhost/)",
    )
    .expect("static pending image pattern");
    for entry in WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|error| AppError::io("检查文章图片状态失败", error))?;
        if !entry.file_type().is_file()
            || !entry
                .path()
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            continue;
        }
        let content = std::fs::read_to_string(entry.path())
            .map_err(|error| AppError::io("检查文章图片状态失败", error))?;
        if pending_image.is_match(&content) {
            return Err(AppError::new(
                "pending_image_uploads",
                "文章中还有尚未上传成功的图片，请等待上传完成或移除该图片后再发布。",
                true,
            ));
        }
    }
    Ok(())
}

fn validate_publish_articles(root: &Path) -> AppResult<()> {
    let source = root.join("source");
    if !source.is_dir() {
        return Ok(());
    }
    let mut invalid = Vec::new();
    for entry in WalkDir::new(&source).follow_links(false) {
        let entry = entry.map_err(|error| AppError::io("检查文章格式失败", error))?;
        if !entry.file_type().is_file()
            || !entry.path().extension().is_some_and(|extension| {
                extension.to_str().is_some_and(|extension| {
                    matches!(extension.to_ascii_lowercase().as_str(), "md" | "markdown")
                })
            })
        {
            continue;
        }
        let content = std::fs::read_to_string(entry.path())
            .map_err(|error| AppError::io("检查文章格式失败", error))?;
        if crate::engine::parse_front_matter(&content).error.is_some() {
            let relative = entry
                .path()
                .strip_prefix(root)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            invalid.push(relative);
        }
    }
    if invalid.is_empty() {
        return Ok(());
    }
    invalid.sort();
    let examples = invalid
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join("、");
    let suffix = if invalid.len() > 3 {
        format!("等 {} 篇", invalid.len())
    } else {
        format!("共 {} 篇", invalid.len())
    };
    Err(AppError::new(
        "invalid_article_front_matter",
        format!("发布已停止：{examples} 的文章信息格式无效（{suffix}）。请修正文章开头后重试，未生成的内容不会被上传。"),
        true,
    ))
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
    let semantic_failure = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut command = CommandWrap::with_new(&step.program, |command| {
        command
            .args(&step.args)
            .current_dir(root)
            .env("PATH", command_path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
    });
    #[cfg(windows)]
    command.wrap(CreationFlags(
        windows::Win32::System::Threading::CREATE_NO_WINDOW,
    ));
    // JobObject adds CREATE_SUSPENDED, which makes Windows Terminal host these child processes.
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
            semantic_failure.clone(),
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
            semantic_failure.clone(),
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
    if result.as_ref().is_ok_and(|code| *code == Some(0)) && semantic_failure.load(Ordering::SeqCst)
    {
        return Err(format!(
            "{} 输出了错误，发布已停止，未继续使用不完整的生成结果。",
            step.name
        ));
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
    semantic_failure: Arc<std::sync::atomic::AtomicBool>,
) -> tauri::async_runtime::JoinHandle<()>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if contains_process_error(&line) {
                        semantic_failure.store(true, Ordering::SeqCst);
                    }
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

fn contains_process_error(line: &str) -> bool {
    let plain = regex::Regex::new(r"\x1b\[[0-9;]*[A-Za-z]")
        .expect("static ANSI pattern")
        .replace_all(line, "");
    let normalized = plain.trim().to_ascii_uppercase();
    normalized.starts_with("ERROR")
        || normalized.starts_with("FATAL")
        || normalized.contains("YAMLEXCEPTION:")
        || normalized.contains("PROCESS FAILED:")
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_preflight_rejects_temporary_editor_image_urls() {
        let temp = tempfile::TempDir::new().unwrap();
        let posts = temp.path().join("source/_posts");
        std::fs::create_dir_all(&posts).unwrap();
        let article = posts.join("post.md");
        std::fs::write(&article, "![说明](http://hlex-asset.localhost/pending)").unwrap();
        assert_eq!(
            ensure_no_pending_editor_images(temp.path())
                .unwrap_err()
                .code,
            "pending_image_uploads"
        );
        std::fs::write(&article, "![说明](https://img.example.com/ready.png)").unwrap();
        assert!(ensure_no_pending_editor_images(temp.path()).is_ok());
        std::fs::write(
            &article,
            "`http://hlex-asset.localhost/example` is documentation, not an image.",
        )
        .unwrap();
        assert!(ensure_no_pending_editor_images(temp.path()).is_ok());
    }

    #[test]
    fn publish_preflight_rejects_invalid_front_matter() {
        let temp = tempfile::TempDir::new().unwrap();
        let posts = temp.path().join("source/_posts");
        std::fs::create_dir_all(&posts).unwrap();
        let article = posts.join("broken.md");
        std::fs::write(&article, "---\ntitle: broken\ntags:\n* Hexo\n---\n正文").unwrap();
        let error = validate_publish_articles(temp.path()).unwrap_err();
        assert_eq!(error.code, "invalid_article_front_matter");
        assert!(error.message.contains("source/_posts/broken.md"));
        std::fs::write(&article, "---\ntitle: ready\ntags:\n  - Hexo\n---\n正文").unwrap();
        assert!(validate_publish_articles(temp.path()).is_ok());
    }

    #[test]
    fn treats_hexo_error_output_as_failure_even_with_a_zero_exit_code() {
        assert!(contains_process_error(
            "\u{1b}[41mERROR\u{1b}[49m Process failed: post.md"
        ));
        assert!(contains_process_error(
            "YAMLException: invalid front matter"
        ));
        assert!(!contains_process_error("INFO Generated: index.html"));
        assert!(!contains_process_error(
            "warning: LF will be replaced by CRLF"
        ));
    }
}
