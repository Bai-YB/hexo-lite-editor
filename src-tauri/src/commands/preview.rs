use crate::{
    app::{AppState, PreviewRuntime},
    data::{append_task_event, finish_task_log, redact_log_line, start_task_log},
    domain::{
        AppError, AppResult, PreviewServerState, PreviewServerView, TaskEvent, TaskEventKind,
        TaskStream, TaskType,
    },
    engine::build_task_steps,
    platform::command_path,
};
use chrono::Local;
use process_wrap::tokio::*;
use serde::Deserialize;
use std::{
    process::Stdio,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::oneshot,
};
use uuid::Uuid;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const PREVIEW_READY_TIMEOUT: Duration = Duration::from_secs(20);
const ROUTE_HELPER_SCRIPT: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/resolve-hexo-route.cjs"
));

#[tauri::command]
pub fn get_preview_status(
    project_id: String,
    session_generation: u64,
    state: State<'_, AppState>,
) -> AppResult<PreviewServerView> {
    state.with_project(&project_id, Some(session_generation), |_| Ok(()))?;
    let guard = state
        .preview
        .lock()
        .map_err(|_| AppError::new("state_poisoned", "预览服务状态不可用。", false))?;
    Ok(guard
        .as_ref()
        .filter(|runtime| {
            runtime.view.project_id == project_id
                && runtime.view.session_generation == session_generation
        })
        .map(|runtime| runtime.view.clone())
        .unwrap_or_else(|| stopped_view(project_id, session_generation, 0, false)))
}

#[tauri::command]
pub async fn start_preview_server(
    app: AppHandle,
    project_id: String,
    session_generation: u64,
) -> AppResult<PreviewServerView> {
    start_preview_internal(&app, &project_id, session_generation).await
}

#[tauri::command]
pub async fn stop_preview_server(
    app: AppHandle,
    project_id: String,
    session_generation: u64,
) -> AppResult<PreviewServerView> {
    stop_preview_internal(&app, &project_id, session_generation).await
}

pub async fn start_preview_internal(
    app: &AppHandle,
    project_id: &str,
    session_generation: u64,
) -> AppResult<PreviewServerView> {
    let state = app.state::<AppState>();
    let root = state.with_project(project_id, Some(session_generation), |project| {
        Ok(project.root.clone())
    })?;
    let config = crate::data::load_config(&state)?.config;
    let port = config.hexo.preview_port;
    let drafts_enabled = config.hexo.preview_drafts;
    {
        let guard = state
            .preview
            .lock()
            .map_err(|_| AppError::new("state_poisoned", "预览服务状态不可用。", false))?;
        if let Some(runtime) = guard.as_ref().filter(|runtime| {
            runtime.view.project_id == project_id
                && runtime.view.session_generation == session_generation
                && matches!(
                    runtime.view.state,
                    PreviewServerState::Starting | PreviewServerState::Running
                )
        }) {
            return Ok(runtime.view.clone());
        }
    }
    if port_is_ready(port).await {
        return Err(AppError::new(
            "preview_port_in_use",
            format!("端口 {port} 已被其他程序占用，请在设置中更换端口。"),
            true,
        ));
    }

    let mut steps = build_task_steps(TaskType::ServerStart, &config, &root)?;
    let mut step = steps.pop().ok_or_else(|| {
        AppError::new("preview_command_missing", "无法生成 Hexo 预览命令。", false)
    })?;
    step.args.extend(["--port".to_string(), port.to_string()]);
    if drafts_enabled {
        step.args.push("--draft".to_string());
    }
    let view = PreviewServerView {
        project_id: project_id.to_string(),
        session_generation,
        state: PreviewServerState::Starting,
        port,
        base_url: None,
        drafts_enabled,
        started_at: None,
        error: None,
    };
    let (cancel_tx, cancel_rx) = oneshot::channel();
    {
        let mut guard = state
            .preview
            .lock()
            .map_err(|_| AppError::new("state_poisoned", "预览服务状态不可用。", false))?;
        if let Some(previous) = guard.take() {
            if let Some(cancel) = previous.cancellation {
                let _ = cancel.send(());
            }
        }
        *guard = Some(PreviewRuntime {
            view: view.clone(),
            cancellation: Some(cancel_tx),
        });
    }
    emit_preview_status(app, &view);
    let app_handle = app.clone();
    let task_id = format!("preview-{}", Uuid::new_v4());
    tauri::async_runtime::spawn(async move {
        run_preview_process(app_handle, task_id, root, step, view, cancel_rx).await;
    });
    get_preview_view(app, project_id, session_generation)
}

pub async fn stop_preview_internal(
    app: &AppHandle,
    project_id: &str,
    session_generation: u64,
) -> AppResult<PreviewServerView> {
    let state = app.state::<AppState>();
    state.with_project(project_id, Some(session_generation), |_| Ok(()))?;
    let view = {
        let mut guard = state
            .preview
            .lock()
            .map_err(|_| AppError::new("state_poisoned", "预览服务状态不可用。", false))?;
        let Some(runtime) = guard.as_mut().filter(|runtime| {
            runtime.view.project_id == project_id
                && runtime.view.session_generation == session_generation
        }) else {
            return Ok(stopped_view(
                project_id.to_string(),
                session_generation,
                0,
                false,
            ));
        };
        runtime.view.state = PreviewServerState::Stopping;
        runtime.view.error = None;
        if let Some(cancel) = runtime.cancellation.take() {
            let _ = cancel.send(());
        }
        runtime.view.clone()
    };
    emit_preview_status(app, &view);
    for _ in 0..60 {
        let current = get_preview_view(app, project_id, session_generation)?;
        if matches!(
            current.state,
            PreviewServerState::Stopped | PreviewServerState::Error
        ) {
            return Ok(current);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(AppError::new(
        "preview_stop_timeout",
        "Hexo 预览服务未能在预期时间内停止。",
        true,
    ))
}

#[tauri::command]
pub async fn resolve_article_preview_url(
    app: AppHandle,
    project_id: String,
    session_generation: u64,
    article_id: String,
) -> AppResult<String> {
    let state = app.state::<AppState>();
    let (root, relative_path, is_draft) =
        state.with_project(&project_id, Some(session_generation), |project| {
            let article = project.article(&article_id)?;
            let summary = project
                .article_summaries
                .iter()
                .find(|summary| summary.article_id == article_id)
                .ok_or_else(|| AppError::new("article_not_found", "文章资源已失效。", true))?;
            Ok((
                project.root.clone(),
                article
                    .canonical_path
                    .strip_prefix(&project.root)
                    .map_err(|_| AppError::new("path_escape", "文章不属于当前项目。", false))?
                    .to_string_lossy()
                    .replace('\\', "/"),
                summary.kind == crate::domain::ArticleKind::Draft,
            ))
        })?;
    let preview = get_preview_view(&app, &project_id, session_generation)?;
    if preview.state != PreviewServerState::Running {
        return Err(AppError::new(
            "preview_not_running",
            "请先启动 Hexo 浏览器预览服务。",
            true,
        ));
    }
    if is_draft && !preview.drafts_enabled {
        return Err(AppError::new(
            "preview_drafts_disabled",
            "当前 Hexo 预览未启用草稿，请在设置中开启后重启预览。",
            true,
        ));
    }
    let mut route_command = tokio::process::Command::new("node");
    route_command
        // Execute through stdin so platform-specific path quoting cannot alter the helper.
        .arg("-")
        .current_dir(&root)
        .env("HLEX_REQUESTED_SOURCE", &relative_path)
        .env("PATH", command_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    route_command.creation_flags(CREATE_NO_WINDOW);
    let mut route_process = route_command.spawn().map_err(|error| {
        AppError::new(
            "preview_route_runtime_missing",
            format!("无法运行项目的 Hexo 路由解析器：{error}"),
            true,
        )
    })?;
    if let Some(mut stdin) = route_process.stdin.take() {
        stdin
            .write_all(ROUTE_HELPER_SCRIPT.as_bytes())
            .await
            .map_err(|error| {
                AppError::new(
                    "preview_route_runtime_missing",
                    format!("鏃犳硶鍐欏叆 Hexo 璺敱瑙ｆ瀽鍣細{error}"),
                    true,
                )
            })?;
    }
    let output = route_process.wait_with_output().await.map_err(|error| {
        AppError::new(
            "preview_route_runtime_missing",
            format!("鏃犳硶杩愯 Hexo 璺敱瑙ｆ瀽鍣細{error}"),
            true,
        )
    })?;
    if !output.status.success() {
        let error = redact_log_line(&String::from_utf8_lossy(&output.stderr));
        return Err(AppError::new(
            "preview_route_failed",
            format!("Hexo 未能解析当前文章路由：{}", error.trim()),
            true,
        ));
    }
    #[derive(Deserialize)]
    struct RouteResult {
        path: String,
    }
    let result: RouteResult = serde_json::from_slice(&output.stdout).map_err(|error| {
        AppError::new(
            "preview_route_invalid",
            format!("Hexo 路由解析结果无效：{error}"),
            true,
        )
    })?;
    build_loopback_article_url(preview.port, &result.path)
}

async fn run_preview_process(
    app: AppHandle,
    task_id: String,
    root: std::path::PathBuf,
    step: crate::engine::TaskStep,
    starting_view: PreviewServerView,
    mut cancel_rx: oneshot::Receiver<()>,
) {
    let state = app.state::<AppState>();
    let project_name = state
        .with_project(
            &starting_view.project_id,
            Some(starting_view.session_generation),
            |project| Ok(project.name.clone()),
        )
        .unwrap_or_else(|_| "Hexo Project".to_string());
    let _ = start_task_log(&state, &task_id, &project_name, TaskType::ServerStart);
    let sequence = Arc::new(AtomicU64::new(0));
    preview_log(
        &app,
        &task_id,
        &starting_view.project_id,
        &sequence,
        TaskEventKind::Queued,
        None,
        None,
    );
    let mut command = CommandWrap::with_new(&step.program, |command| {
        command
            .args(&step.args)
            .current_dir(&root)
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
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let app_error = AppError::new(
                "preview_start_failed",
                format!("无法启动 Hexo 预览：{error}"),
                true,
            );
            set_preview_error(&app, &starting_view, app_error.clone());
            preview_log(
                &app,
                &task_id,
                &starting_view.project_id,
                &sequence,
                TaskEventKind::Finished,
                Some(app_error.message),
                Some(false),
            );
            let _ = finish_task_log(&state, &task_id, false);
            return;
        }
    };
    let stdout_task = child.stdout().take().map(|stdout| {
        spawn_preview_reader(
            app.clone(),
            task_id.clone(),
            starting_view.project_id.clone(),
            TaskStream::Stdout,
            sequence.clone(),
            stdout,
        )
    });
    let stderr_task = child.stderr().take().map(|stderr| {
        spawn_preview_reader(
            app.clone(),
            task_id.clone(),
            starting_view.project_id.clone(),
            TaskStream::Stderr,
            sequence.clone(),
            stderr,
        )
    });

    let deadline = tokio::time::Instant::now() + PREVIEW_READY_TIMEOUT;
    let ready = loop {
        tokio::select! {
            _ = &mut cancel_rx => {
                let _ = Box::into_pin(child.kill()).await;
                set_preview_stopped(&app, &starting_view);
                break false;
            }
            _ = tokio::time::sleep(Duration::from_millis(120)) => {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        set_preview_error(&app, &starting_view, AppError::new(
                            "preview_exited_early",
                            format!("Hexo 预览在就绪前退出（{}）。", status.code().unwrap_or(-1)),
                            true,
                        ));
                        break false;
                    }
                    Err(error) => {
                        set_preview_error(&app, &starting_view, AppError::new(
                            "preview_status_failed",
                            format!("无法检查 Hexo 预览状态：{error}"),
                            true,
                        ));
                        break false;
                    }
                    Ok(None) if port_is_ready(starting_view.port).await => {
                        set_preview_running(&app, &starting_view);
                        break true;
                    }
                    Ok(None) if tokio::time::Instant::now() >= deadline => {
                        let _ = Box::into_pin(child.kill()).await;
                        set_preview_error(&app, &starting_view, AppError::new(
                            "preview_ready_timeout",
                            "Hexo 进程已经启动，但端口在 20 秒内没有就绪。",
                            true,
                        ));
                        break false;
                    }
                    Ok(None) => {}
                }
            }
        }
    };
    if ready {
        tokio::select! {
            status = child.wait() => {
                let message = match status {
                    Ok(status) => format!("Hexo 预览意外退出（{}）。", status.code().unwrap_or(-1)),
                    Err(error) => format!("等待 Hexo 预览进程失败：{error}"),
                };
                set_preview_error(&app, &starting_view, AppError::new("preview_exited", message, true));
            }
            _ = &mut cancel_rx => {
                let _ = Box::into_pin(child.kill()).await;
                set_preview_stopped(&app, &starting_view);
            }
        }
    }
    if let Some(task) = stdout_task {
        let _ = task.await;
    }
    if let Some(task) = stderr_task {
        let _ = task.await;
    }
    let success = matches!(
        get_preview_view(
            &app,
            &starting_view.project_id,
            starting_view.session_generation
        )
        .map(|view| view.state),
        Ok(PreviewServerState::Stopped)
    );
    preview_log(
        &app,
        &task_id,
        &starting_view.project_id,
        &sequence,
        TaskEventKind::Finished,
        None,
        Some(success),
    );
    let _ = finish_task_log(&state, &task_id, success);
}

fn spawn_preview_reader<R>(
    app: AppHandle,
    task_id: String,
    project_id: String,
    stream: TaskStream,
    sequence: Arc<AtomicU64>,
    reader: R,
) -> tauri::async_runtime::JoinHandle<()>
where
    R: AsyncRead + Unpin + Send + 'static,
{
    tauri::async_runtime::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let state = app.state::<AppState>();
            let mut event = TaskEvent {
                task_id: task_id.clone(),
                project_id: project_id.clone(),
                sequence: sequence.fetch_add(1, Ordering::SeqCst),
                kind: TaskEventKind::Log,
                step: Some("Hexo 预览".to_string()),
                stream: Some(stream),
                line: Some(line),
                success: None,
                exit_code: None,
                timestamp: Local::now().to_rfc3339(),
            };
            let _ = append_task_event(&state, &mut event);
        }
    })
}

fn preview_log(
    app: &AppHandle,
    task_id: &str,
    project_id: &str,
    sequence: &AtomicU64,
    kind: TaskEventKind,
    line: Option<String>,
    success: Option<bool>,
) {
    let state = app.state::<AppState>();
    let mut event = TaskEvent {
        task_id: task_id.to_string(),
        project_id: project_id.to_string(),
        sequence: sequence.fetch_add(1, Ordering::SeqCst),
        kind,
        step: Some("Hexo 预览".to_string()),
        stream: line.as_ref().map(|_| TaskStream::Stderr),
        line,
        success,
        exit_code: None,
        timestamp: Local::now().to_rfc3339(),
    };
    let _ = append_task_event(&state, &mut event);
}

fn get_preview_view(
    app: &AppHandle,
    project_id: &str,
    session_generation: u64,
) -> AppResult<PreviewServerView> {
    let state = app.state::<AppState>();
    let guard = state
        .preview
        .lock()
        .map_err(|_| AppError::new("state_poisoned", "预览服务状态不可用。", false))?;
    Ok(guard
        .as_ref()
        .filter(|runtime| {
            runtime.view.project_id == project_id
                && runtime.view.session_generation == session_generation
        })
        .map(|runtime| runtime.view.clone())
        .unwrap_or_else(|| stopped_view(project_id.to_string(), session_generation, 0, false)))
}

fn set_preview_running(app: &AppHandle, starting: &PreviewServerView) {
    update_preview(app, starting, |view| {
        view.state = PreviewServerState::Running;
        view.base_url = Some(format!("http://127.0.0.1:{}/", view.port));
        view.started_at = Some(Local::now().to_rfc3339());
        view.error = None;
    });
}

fn set_preview_stopped(app: &AppHandle, starting: &PreviewServerView) {
    update_preview(app, starting, |view| {
        view.state = PreviewServerState::Stopped;
        view.base_url = None;
        view.error = None;
    });
}

fn set_preview_error(app: &AppHandle, starting: &PreviewServerView, error: AppError) {
    update_preview(app, starting, move |view| {
        view.state = PreviewServerState::Error;
        view.base_url = None;
        view.error = Some(error);
    });
}

fn update_preview(
    app: &AppHandle,
    starting: &PreviewServerView,
    update: impl FnOnce(&mut PreviewServerView),
) {
    let state = app.state::<AppState>();
    let next = state.preview.lock().ok().and_then(|mut guard| {
        let runtime = guard.as_mut()?;
        if runtime.view.project_id != starting.project_id
            || runtime.view.session_generation != starting.session_generation
        {
            return None;
        }
        update(&mut runtime.view);
        if matches!(
            runtime.view.state,
            PreviewServerState::Stopped | PreviewServerState::Error
        ) {
            runtime.cancellation = None;
        }
        Some(runtime.view.clone())
    });
    if let Some(view) = next {
        emit_preview_status(app, &view);
    }
}

fn emit_preview_status(app: &AppHandle, view: &PreviewServerView) {
    let _ = app.emit("preview-status", view);
}

fn stopped_view(
    project_id: String,
    session_generation: u64,
    port: u16,
    drafts_enabled: bool,
) -> PreviewServerView {
    PreviewServerView {
        project_id,
        session_generation,
        state: PreviewServerState::Stopped,
        port,
        base_url: None,
        drafts_enabled,
        started_at: None,
        error: None,
    }
}

async fn port_is_ready(port: u16) -> bool {
    tokio::time::timeout(
        Duration::from_millis(250),
        TcpStream::connect(("127.0.0.1", port)),
    )
    .await
    .is_ok_and(|result| result.is_ok())
}

#[allow(dead_code)]
fn preview_helper_path(app: &AppHandle) -> AppResult<std::path::PathBuf> {
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|error| AppError::new("resource_dir", error.to_string(), false))?
        .join("resources")
        .join("resolve-hexo-route.cjs");
    if bundled.is_file() {
        return Ok(bundled);
    }
    let development = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("resolve-hexo-route.cjs");
    if development.is_file() {
        return Ok(development);
    }
    Err(AppError::new(
        "preview_helper_missing",
        "应用缺少 Hexo 路由解析资源。",
        false,
    ))
}

fn build_loopback_article_url(port: u16, path: &str) -> AppResult<String> {
    let route = if let Ok(url) = url::Url::parse(path) {
        url.path().to_string()
    } else {
        format!("/{}", path.trim_start_matches('/'))
    };
    let mut base = url::Url::parse(&format!("http://127.0.0.1:{port}/"))
        .map_err(|_| AppError::invalid("预览地址无效。"))?;
    base.set_path(&route);
    base.set_query(None);
    base.set_fragment(None);
    Ok(base.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_url_is_always_loopback_and_keeps_hexo_route() {
        assert_eq!(
            build_loopback_article_url(4000, "posts/中文文章/").unwrap(),
            "http://127.0.0.1:4000/posts/%E4%B8%AD%E6%96%87%E6%96%87%E7%AB%A0/"
        );
        assert_eq!(
            build_loopback_article_url(5000, "https://example.com/blog/post/").unwrap(),
            "http://127.0.0.1:5000/blog/post/"
        );
    }

    #[tokio::test]
    async fn detects_a_port_that_is_already_owned() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        assert!(port_is_ready(port).await);
    }
}
