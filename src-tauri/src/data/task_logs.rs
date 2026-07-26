use crate::{
    app::AppState,
    data::load_config,
    domain::{AppError, AppResult, TaskEvent, TaskLogPage, TaskLogSummary, TaskType},
    platform::cloudflare_token_for_redaction,
};
use chrono::{DateTime, Duration, Local};
use regex::Regex;
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    sync::LazyLock,
};
use tauri::State;

const MAX_TASK_LOG_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TASK_LOG_FILES: usize = 100;

static HEADER_SECRET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(authorization|authcode|token|secret|password)(\s*[:=]\s*)([^\s,;]+)")
        .expect("valid secret pattern")
});
static BEARER_SECRET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(bearer\s+)[A-Za-z0-9._~+/=-]+").expect("valid bearer pattern")
});
static URL_SECRET: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(https?://)[^/@:\s]+:[^/@\s]+@").expect("valid URL credential pattern")
});

pub fn start_task_log(
    state: &AppState,
    task_id: &str,
    project_name: &str,
    task_type: TaskType,
) -> AppResult<()> {
    validate_log_id(task_id)?;
    fs::create_dir_all(&state.task_log_dir)
        .map_err(|error| AppError::io("创建诊断日志目录失败", error))?;
    let summary = TaskLogSummary {
        task_id: task_id.to_string(),
        project_name: project_name.to_string(),
        task_type,
        started_at: Local::now().to_rfc3339(),
        finished_at: None,
        success: None,
        size: 0,
        truncated: false,
    };
    write_summary(&state.task_log_dir, &summary)?;
    cleanup_task_logs(state)
}

pub fn append_task_event(state: &AppState, event: &mut TaskEvent) -> AppResult<()> {
    validate_log_id(&event.task_id)?;
    let _write_guard = state
        .task_log_write_lock
        .lock()
        .map_err(|_| AppError::new("state_poisoned", "诊断日志写入队列不可用。", false))?;
    if let Some(line) = event.line.as_mut() {
        *line = redact_log_line(line);
    }
    let path = event_path(&state.task_log_dir, &event.task_id);
    let current_size = fs::metadata(&path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let mut bytes = serde_json::to_vec(event)
        .map_err(|error| AppError::new("task_log_serialize", error.to_string(), false))?;
    bytes.push(b'\n');
    if current_size.saturating_add(bytes.len() as u64) > MAX_TASK_LOG_BYTES {
        mark_truncated(&state.task_log_dir, &event.task_id)?;
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| AppError::io("写入诊断日志失败", error))?;
    file.write_all(&bytes)
        .map_err(|error| AppError::io("写入诊断日志失败", error))?;
    Ok(())
}

pub fn finish_task_log(state: &AppState, task_id: &str, success: bool) -> AppResult<()> {
    let mut summary = read_summary(&state.task_log_dir, task_id)?;
    summary.finished_at = Some(Local::now().to_rfc3339());
    summary.success = Some(success);
    summary.size = fs::metadata(event_path(&state.task_log_dir, task_id))
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    write_summary(&state.task_log_dir, &summary)?;
    cleanup_task_logs(state)
}

#[tauri::command]
pub fn list_task_logs(state: State<'_, AppState>) -> AppResult<Vec<TaskLogSummary>> {
    cleanup_task_logs(&state)?;
    let mut summaries = read_all_summaries(&state.task_log_dir)?;
    summaries.sort_by(|left, right| right.started_at.cmp(&left.started_at));
    Ok(summaries)
}

#[tauri::command]
pub fn read_task_log(
    task_id: String,
    cursor: Option<usize>,
    count: Option<usize>,
    state: State<'_, AppState>,
) -> AppResult<TaskLogPage> {
    validate_log_id(&task_id)?;
    let start = cursor.unwrap_or(0);
    let count = count.unwrap_or(300).clamp(1, 500);
    let path = event_path(&state.task_log_dir, &task_id);
    let file = fs::File::open(path).map_err(|error| AppError::io("读取诊断日志失败", error))?;
    let mut events = Vec::new();
    let mut has_more = false;
    for line in BufReader::new(file).lines().skip(start).take(count + 1) {
        let line = line.map_err(|error| AppError::io("读取诊断日志失败", error))?;
        if events.len() == count {
            has_more = true;
            break;
        }
        if let Ok(event) = serde_json::from_str::<TaskEvent>(&line) {
            events.push(event);
        }
    }
    Ok(TaskLogPage {
        next_cursor: has_more.then_some(start + events.len()),
        events,
    })
}

#[tauri::command]
pub fn delete_task_log(task_id: String, state: State<'_, AppState>) -> AppResult<()> {
    validate_log_id(&task_id)?;
    remove_log_files(&state.task_log_dir, &task_id)
}

#[tauri::command]
pub fn clear_task_logs(state: State<'_, AppState>) -> AppResult<()> {
    for summary in read_all_summaries(&state.task_log_dir)? {
        remove_log_files(&state.task_log_dir, &summary.task_id)?;
    }
    Ok(())
}

pub fn cleanup_task_logs(state: &AppState) -> AppResult<()> {
    let config = load_config(state)?.config;
    let cutoff = Local::now() - Duration::days(config.diagnostics.log_retention_days.into());
    let mut summaries = read_all_summaries(&state.task_log_dir)?;
    for summary in &summaries {
        let expired = DateTime::parse_from_rfc3339(&summary.started_at)
            .map(|date| date.with_timezone(&Local) < cutoff)
            .unwrap_or(true);
        if expired {
            remove_log_files(&state.task_log_dir, &summary.task_id)?;
        }
    }
    summaries = read_all_summaries(&state.task_log_dir)?;
    summaries.sort_by(|left, right| left.started_at.cmp(&right.started_at));
    let max_bytes = u64::from(config.diagnostics.max_log_storage_mb) * 1024 * 1024;
    let mut total = summaries.iter().map(|summary| summary.size).sum::<u64>();
    while summaries.len() > MAX_TASK_LOG_FILES || total > max_bytes {
        let oldest = summaries.remove(0);
        total = total.saturating_sub(oldest.size);
        remove_log_files(&state.task_log_dir, &oldest.task_id)?;
    }
    Ok(())
}

pub fn redact_log_line(line: &str) -> String {
    let mut redacted = BEARER_SECRET.replace_all(line, "$1[REDACTED]").into_owned();
    redacted = HEADER_SECRET
        .replace_all(&redacted, "$1$2[REDACTED]")
        .into_owned();
    redacted = URL_SECRET
        .replace_all(&redacted, "$1[REDACTED]@")
        .into_owned();
    if let Ok(secret) = cloudflare_token_for_redaction("primary") {
        if !secret.is_empty() {
            redacted = redacted.replace(&secret, "[REDACTED]");
        }
    }
    redacted
}

fn read_all_summaries(directory: &Path) -> AppResult<Vec<TaskLogSummary>> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut summaries = Vec::new();
    for entry in
        fs::read_dir(directory).map_err(|error| AppError::io("读取诊断日志目录失败", error))?
    {
        let entry = entry.map_err(|error| AppError::io("读取诊断日志目录失败", error))?;
        if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        if let Ok(content) = fs::read_to_string(entry.path()) {
            if let Ok(summary) = serde_json::from_str::<TaskLogSummary>(&content) {
                summaries.push(summary);
            }
        }
    }
    Ok(summaries)
}

fn read_summary(directory: &Path, task_id: &str) -> AppResult<TaskLogSummary> {
    validate_log_id(task_id)?;
    let content = fs::read_to_string(summary_path(directory, task_id))
        .map_err(|error| AppError::io("读取诊断日志摘要失败", error))?;
    serde_json::from_str(&content)
        .map_err(|error| AppError::new("task_log_invalid", error.to_string(), true))
}

fn write_summary(directory: &Path, summary: &TaskLogSummary) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(summary)
        .map_err(|error| AppError::new("task_log_serialize", error.to_string(), false))?;
    crate::platform::atomic_write(&summary_path(directory, &summary.task_id), &bytes)
}

fn mark_truncated(directory: &Path, task_id: &str) -> AppResult<()> {
    let mut summary = read_summary(directory, task_id)?;
    if !summary.truncated {
        summary.truncated = true;
        summary.size = MAX_TASK_LOG_BYTES;
        write_summary(directory, &summary)?;
    }
    Ok(())
}

fn remove_log_files(directory: &Path, task_id: &str) -> AppResult<()> {
    for path in [
        summary_path(directory, task_id),
        event_path(directory, task_id),
    ] {
        if path.exists() {
            fs::remove_file(&path).map_err(|error| AppError::io("删除诊断日志失败", error))?;
        }
    }
    Ok(())
}

fn summary_path(directory: &Path, task_id: &str) -> PathBuf {
    directory.join(format!("{task_id}.json"))
}

fn event_path(directory: &Path, task_id: &str) -> PathBuf {
    directory.join(format!("{task_id}.jsonl"))
}

fn validate_log_id(task_id: &str) -> AppResult<()> {
    if task_id.is_empty()
        || task_id.len() > 64
        || !task_id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err(AppError::invalid("任务日志 ID 无效。"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_headers_bearer_url_credentials_and_known_tokens() {
        let line = "Authorization: Bearer abc.def token=secret https://user:pass@example.com";
        let redacted = redact_log_line(line);
        assert!(!redacted.contains("abc.def"));
        assert!(!redacted.contains("secret"));
        assert!(!redacted.contains("user:pass"));
        assert!(redacted.contains("[REDACTED]"));
    }
}
