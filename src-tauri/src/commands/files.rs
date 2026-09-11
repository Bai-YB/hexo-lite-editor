use crate::{
    app::AppState,
    domain::{AppError, AppResult, ProjectFileEntry, ProjectFileSnapshot},
    platform::atomic_write,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, Manager};

const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;

fn safe_path(root: &Path, relative: &str) -> AppResult<PathBuf> {
    let rel = Path::new(relative);
    if rel.is_absolute()
        || relative.contains(':')
        || rel
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
            && !relative.is_empty()
    {
        return Err(AppError::invalid("文件路径必须位于项目目录内。"));
    }
    let root = root
        .canonicalize()
        .map_err(|e| AppError::io("无法解析项目目录", e))?;
    let mut current = root.clone();
    for component in rel.components() {
        current.push(component.as_os_str());
        let meta =
            fs::symlink_metadata(&current).map_err(|e| AppError::io("文件或目录不存在", e))?;
        if meta.file_type().is_symlink() || is_reparse_point(&meta) {
            return Err(AppError::invalid(
                "暂不支持打开符号链接或目录联接，请打开项目内的原始路径。",
            ));
        }
        current = current
            .canonicalize()
            .map_err(|e| AppError::io("无法解析文件路径", e))?;
        if !current.starts_with(&root) {
            return Err(AppError::invalid("文件路径必须位于项目目录内。"));
        }
    }
    Ok(current)
}

#[cfg(windows)]
fn is_reparse_point(meta: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    meta.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
fn is_reparse_point(_: &fs::Metadata) -> bool {
    false
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn known_binary(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "ico"
            | "avif"
            | "pdf"
            | "zip"
            | "7z"
            | "gz"
            | "woff"
            | "woff2"
            | "ttf"
            | "mp4"
            | "mp3"
            | "exe"
            | "dll"
    )
}

fn read_snapshot(
    root: &Path,
    project_id: &str,
    generation: u64,
    path: &str,
) -> AppResult<ProjectFileSnapshot> {
    let file = safe_path(root, path)?;
    let meta = fs::metadata(&file).map_err(|e| AppError::io("读取文件信息失败", e))?;
    if !meta.is_file() {
        return Err(AppError::invalid("请选择文件。"));
    }
    let mut snapshot = ProjectFileSnapshot {
        project_id: project_id.into(),
        session_generation: generation,
        path: path.into(),
        content: String::new(),
        content_hash: String::new(),
        editable: false,
        read_only_reason: None,
    };
    if meta.len() > MAX_TEXT_BYTES {
        snapshot.read_only_reason = Some("文件超过 2 MB，请使用外部编辑器打开。".into());
        return Ok(snapshot);
    }
    if known_binary(&file) {
        snapshot.read_only_reason = Some("这是二进制文件，无法作为文本编辑。".into());
        return Ok(snapshot);
    }
    let bytes = fs::read(&file).map_err(|e| AppError::io("读取文件失败", e))?;
    snapshot.content_hash = hash(&bytes);
    match String::from_utf8(bytes) {
        Ok(content) if !content.contains('\0') => {
            snapshot.content = content;
            snapshot.editable = true;
        }
        _ => snapshot.read_only_reason = Some("该文件不是 UTF-8 文本，无法在此编辑。".into()),
    }
    if Path::new(path)
        .components()
        .any(|part| part.as_os_str() == ".git")
    {
        snapshot.editable = false;
        snapshot.read_only_reason = Some("Git 内部文件仅供查看，请使用 Git 管理仓库。".into());
    }
    Ok(snapshot)
}

fn list_files(
    project_id: String,
    session_generation: u64,
    directory: String,
    state: &AppState,
) -> AppResult<Vec<ProjectFileEntry>> {
    state.with_project(&project_id, Some(session_generation), |project| {
        let dir = safe_path(&project.root, &directory)?;
        if !dir.is_dir() {
            return Err(AppError::invalid("目录不存在。"));
        }
        let mut out = Vec::new();
        for entry in fs::read_dir(&dir).map_err(|e| AppError::io("读取目录失败", e))? {
            let entry = entry.map_err(|e| AppError::io("读取目录项失败", e))?;
            let path = entry.path();
            let meta =
                fs::symlink_metadata(&path).map_err(|e| AppError::io("读取文件信息失败", e))?;
            if meta.file_type().is_symlink() || is_reparse_point(&meta) {
                continue;
            }
            let rel = path
                .strip_prefix(&project.root)
                .map_err(|_| AppError::invalid("文件路径必须位于项目目录内。"))?
                .to_string_lossy()
                .replace('\\', "/");
            let is_dir = meta.is_dir();
            out.push(ProjectFileEntry {
                path: rel,
                name: entry.file_name().to_string_lossy().to_string(),
                kind: if is_dir {
                    "directory".into()
                } else {
                    "file".into()
                },
                extension: path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(str::to_string),
                size: (!is_dir).then_some(meta.len()),
                modified_at: meta
                    .modified()
                    .ok()
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339()),
                editable: !is_dir && meta.len() <= MAX_TEXT_BYTES && !known_binary(&path),
            });
        }
        out.sort_by_key(|e| (e.kind != "directory", e.name.to_lowercase()));
        Ok(out)
    })
}

fn load_file(
    project_id: String,
    session_generation: u64,
    path: String,
    state: &AppState,
) -> AppResult<ProjectFileSnapshot> {
    state.with_project(&project_id, Some(session_generation), |project| {
        ensure_non_article(project, &path)?;
        read_snapshot(&project.root, &project.id, project.generation, &path)
    })
}

fn save_file(
    project_id: String,
    session_generation: u64,
    path: String,
    content: String,
    expected_hash: String,
    app: AppHandle,
    state: &AppState,
) -> AppResult<ProjectFileSnapshot> {
    let root = state.with_project(&project_id, Some(session_generation), |p| {
        Ok(p.root.clone())
    })?;
    let lock = state.project_file_lock(&root)?;
    let _guard = lock.lock().map_err(|_| AppError::session_expired())?;
    let backup = crate::data::load_config(state)?
        .config
        .general
        .backup_before_save;
    let result = state.with_project(&project_id, Some(session_generation), |project| {
        ensure_non_article(project, &path)?;
        save_text(&project.root, &path, &content, &expected_hash, backup)?;
        Ok(ProjectFileSnapshot {
            project_id: project.id.clone(),
            session_generation: project.generation,
            path: path.clone(),
            content: content.clone(),
            content_hash: hash(content.as_bytes()),
            editable: true,
            read_only_reason: None,
        })
    })?;
    drop(_guard);
    super::sync::schedule_sync_after_save(app, root);
    Ok(result)
}

fn ensure_non_article(project: &crate::app::ProjectSession, path: &str) -> AppResult<()> {
    let canonical = safe_path(&project.root, path)?;
    if project
        .articles
        .values()
        .any(|article| article.canonical_path == canonical)
    {
        return Err(AppError::new(
            "article_requires_editor",
            "这是博文文件，请从目录树选择原始路径，在写作页编辑。",
            true,
        ));
    }
    Ok(())
}

fn save_text(
    root: &Path,
    path: &str,
    content: &str,
    expected_hash: &str,
    backup: bool,
) -> AppResult<()> {
    if content.len() as u64 > MAX_TEXT_BYTES {
        return Err(AppError::invalid("内容超过 2 MB，无法保存。"));
    }
    let current = read_snapshot(root, "", 0, path)?;
    if !current.editable {
        return Err(AppError::invalid(
            current
                .read_only_reason
                .unwrap_or_else(|| "文件不可编辑。".into()),
        ));
    }
    if current.content_hash != expected_hash {
        return Err(AppError::new(
            "file_changed",
            "文件已在外部修改。当前编辑已保留，请比较磁盘内容后重试。",
            true,
        ));
    }
    let file = safe_path(root, path)?;
    if backup {
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        let backup_dir = file.parent().unwrap_or(root).join(".hlex-backups");
        if backup_dir.exists() {
            safe_path(
                root,
                &backup_dir
                    .strip_prefix(root)
                    .map_err(|_| AppError::invalid("备份目录不在项目内。"))?
                    .to_string_lossy(),
            )?;
        }
        fs::create_dir_all(&backup_dir).map_err(|e| AppError::io("创建备份目录失败", e))?;
        let backup_path = backup_dir.join(format!(
            "{name}.{}.bak",
            chrono::Local::now().format("%Y%m%d%H%M%S%3f")
        ));
        fs::copy(&file, backup_path).map_err(|e| AppError::io("保存前备份失败", e))?;
    }
    atomic_write(&file, content.as_bytes())
}

#[tauri::command]
pub async fn list_project_files(
    project_id: String,
    session_generation: u64,
    directory: String,
    app: AppHandle,
) -> AppResult<Vec<ProjectFileEntry>> {
    tauri::async_runtime::spawn_blocking(move || {
        list_files(
            project_id,
            session_generation,
            directory,
            &app.state::<AppState>(),
        )
    })
    .await
    .map_err(|e| AppError::io("读取目录任务失败", e))?
}

#[tauri::command]
pub async fn load_project_file(
    project_id: String,
    session_generation: u64,
    path: String,
    app: AppHandle,
) -> AppResult<ProjectFileSnapshot> {
    tauri::async_runtime::spawn_blocking(move || {
        load_file(
            project_id,
            session_generation,
            path,
            &app.state::<AppState>(),
        )
    })
    .await
    .map_err(|e| AppError::io("读取文件任务失败", e))?
}

#[tauri::command]
pub async fn save_project_file(
    project_id: String,
    session_generation: u64,
    path: String,
    content: String,
    expected_hash: String,
    app: AppHandle,
) -> AppResult<ProjectFileSnapshot> {
    tauri::async_runtime::spawn_blocking(move || {
        save_file(
            project_id,
            session_generation,
            path,
            content,
            expected_hash,
            app.clone(),
            &app.state::<AppState>(),
        )
    })
    .await
    .map_err(|e| AppError::io("保存文件任务失败", e))?
}

#[tauri::command]
pub async fn reveal_project_file(
    project_id: String,
    session_generation: u64,
    path: String,
    app: AppHandle,
) -> AppResult<()> {
    tauri::async_runtime::spawn_blocking(move || {
        let file =
            app.state::<AppState>()
                .with_project(&project_id, Some(session_generation), |p| {
                    safe_path(&p.root, &path)
                })?;
        reveal_file(&file)
    })
    .await
    .map_err(|e| AppError::io("打开文件位置失败", e))?
}

fn reveal_file(path: &Path) -> AppResult<()> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", path.display()))
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| AppError::io("打开文件位置失败", e))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(|e| AppError::io("打开文件位置失败", e))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path.parent().unwrap_or(path))
            .spawn()
            .map_err(|e| AppError::io("打开文件位置失败", e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_generic_article_access_even_through_path_aliases() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::create_dir_all(root.join("source/_posts")).unwrap();
        fs::write(root.join("source/_posts/article.md"), "# article").unwrap();
        let canonical = root
            .join("source/_posts/article.md")
            .canonicalize()
            .unwrap();
        let state = AppState::new(&dir.path().join("appdata"));
        *state.project.write().unwrap() = Some(crate::app::ProjectSession {
            id: "project".into(),
            generation: 1,
            name: "fixture".into(),
            root,
            warnings: vec![],
            article_summaries: vec![],
            assets: Default::default(),
            remote_assets: Default::default(),
            articles: [(
                "article".into(),
                crate::app::ArticleRecord {
                    id: "article".into(),
                    canonical_path: canonical,
                    revision: 0,
                    remote_asset_folder: String::new(),
                },
            )]
            .into(),
        });
        assert_eq!(
            load_file(
                "project".into(),
                1,
                "source//_posts/article.md".into(),
                &state
            )
            .unwrap_err()
            .code,
            "article_requires_editor"
        );
        state
            .with_project("project", Some(1), |project| {
                assert_eq!(
                    ensure_non_article(project, "source/_posts/article.md")
                        .unwrap_err()
                        .code,
                    "article_requires_editor"
                );
                #[cfg(windows)]
                assert_eq!(
                    ensure_non_article(project, "SOURCE/_POSTS/ARTICLE.MD")
                        .unwrap_err()
                        .code,
                    "article_requires_editor"
                );
                Ok(())
            })
            .unwrap();
    }
    #[test]
    fn lists_canonical_project_paths_and_saves_with_hidden_backup() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().canonicalize().unwrap();
        fs::create_dir_all(root.join("source/_data")).unwrap();
        fs::write(root.join("source/_data/link.yml"), "- name: original\n").unwrap();
        let state = AppState::new(&dir.path().join("appdata"));
        *state.project.write().unwrap() = Some(crate::app::ProjectSession {
            id: "project".into(),
            generation: 1,
            name: "fixture".into(),
            root: root.clone(),
            warnings: vec![],
            article_summaries: vec![],
            articles: Default::default(),
            assets: Default::default(),
            remote_assets: Default::default(),
        });
        let entries = list_files("project".into(), 1, "source/_data".into(), &state).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "source/_data/link.yml");
        assert!(list_files("project".into(), 2, "source".into(), &state).is_err());
        let loaded = read_snapshot(&root, "project", 1, &entries[0].path).unwrap();
        save_text(
            &root,
            &loaded.path,
            "- name: updated\n",
            &loaded.content_hash,
            true,
        )
        .unwrap();
        let backups = fs::read_dir(root.join("source/_data/.hlex-backups"))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(
            fs::read_to_string(backups[0].path()).unwrap(),
            loaded.content
        );
    }
    #[test]
    fn saves_friend_links_and_rejects_same_second_external_changes() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("source/_data")).unwrap();
        fs::write(dir.path().join("source/_data/links.yml"), "- name: first\n").unwrap();
        let loaded = read_snapshot(dir.path(), "project", 1, "source/_data/links.yml").unwrap();
        save_text(
            dir.path(),
            &loaded.path,
            "- name: second\n",
            &loaded.content_hash,
            false,
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(dir.path().join(&loaded.path)).unwrap(),
            "- name: second\n"
        );
        assert_eq!(
            save_text(
                dir.path(),
                &loaded.path,
                "stale",
                &loaded.content_hash,
                false
            )
            .unwrap_err()
            .code,
            "file_changed"
        );
    }
    #[test]
    fn blocks_parent_absolute_and_binary_paths_and_allows_dotfile_text() {
        let dir = tempfile::tempdir().unwrap();
        assert!(safe_path(dir.path(), "../outside").is_err());
        assert!(safe_path(dir.path(), "/outside").is_err());
        assert!(safe_path(dir.path(), "C:\\outside").is_err());
        fs::write(dir.path().join(".gitignore"), "node_modules\n").unwrap();
        assert!(
            read_snapshot(dir.path(), "p", 1, ".gitignore")
                .unwrap()
                .editable
        );
        fs::write(dir.path().join("binary.dat"), [0, 255, 1]).unwrap();
        assert!(
            !read_snapshot(dir.path(), "p", 1, "binary.dat")
                .unwrap()
                .editable
        );
        assert!(save_text(dir.path(), "binary.dat", "replacement", "", false).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlink_directory_escape() {
        let dir = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(external.path(), dir.path().join("linked")).unwrap();
        assert!(safe_path(dir.path(), "linked").is_err());
    }
}
