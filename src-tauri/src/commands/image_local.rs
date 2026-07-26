use crate::{
    app::{AppState, AssetRecord, AssetSource},
    data::load_config,
    domain::{AppError, AppResult, LocalImage},
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};
use uuid::Uuid;
use walkdir::WalkDir;

pub(super) const MAX_IMAGE_BYTES: u64 = 25 * 1024 * 1024;

pub(super) fn list_local_images_impl(
    state: &AppState,
    project_id: &str,
    generation: u64,
) -> AppResult<Vec<LocalImage>> {
    let root = state.with_project(project_id, Some(generation), |project| {
        Ok(project.root.clone())
    })?;
    let config = load_config(state)?.config;
    let canonical_directory =
        super::images::local_image_directory(&root, &config.image_bed.local_image_dir)?;
    let mut records = Vec::new();
    for entry in WalkDir::new(&canonical_directory)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        let Ok(mime) = supported_mime(path) else {
            continue;
        };
        let canonical = path
            .canonicalize()
            .map_err(|error| AppError::io("验证图片路径失败", error))?;
        if !canonical.starts_with(&canonical_directory) {
            continue;
        }
        let metadata =
            fs::metadata(&canonical).map_err(|error| AppError::io("读取图片信息失败", error))?;
        let token = Uuid::new_v4().to_string();
        let relative_path = canonical
            .strip_prefix(&root)
            .map_err(|_| AppError::new("path_escape", "图片不属于当前项目。", false))?
            .to_string_lossy()
            .replace('\\', "/");
        let markdown_url = super::images::local_markdown_url(
            &config.image_bed.local_markdown_prefix,
            canonical
                .strip_prefix(&canonical_directory)
                .map_err(|_| AppError::new("path_escape", "图片不属于配置目录。", false))?,
        )?;
        records.push((
            LocalImage {
                image_id: token.clone(),
                name: canonical
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("image")
                    .to_string(),
                relative_path,
                markdown_url,
                mime: mime.clone(),
                size: metadata.len(),
                preview_url: asset_url(&token),
            },
            token,
            AssetRecord {
                source: AssetSource::Disk(canonical),
                mime,
                generation,
                expires_at: SystemTime::now() + Duration::from_secs(15 * 60),
            },
        ));
    }
    records.sort_by(|left, right| left.0.name.cmp(&right.0.name));
    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    let project = guard.as_mut().ok_or_else(AppError::session_expired)?;
    project.require_identity(project_id, Some(generation))?;
    project
        .assets
        .retain(|_, asset| asset.generation == generation && asset.expires_at > SystemTime::now());
    for (_, token, asset) in &records {
        project.assets.insert(token.clone(), asset.clone());
    }
    Ok(records.into_iter().map(|record| record.0).collect())
}

pub(super) fn validate_image_file(path: &Path) -> AppResult<()> {
    let metadata = fs::metadata(path).map_err(|error| AppError::io("读取图片信息失败", error))?;
    if !metadata.is_file() {
        return Err(AppError::invalid("所选内容不是文件。"));
    }
    if metadata.len() > MAX_IMAGE_BYTES {
        return Err(AppError::new(
            "image_too_large",
            "图片不能超过 25 MB。",
            true,
        ));
    }
    supported_mime(path).map(|_| ())
}

pub(super) fn supported_mime(path: &Path) -> AppResult<String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mime = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => {
            return Err(AppError::new(
                "unsupported_image",
                "仅支持 PNG、JPEG、GIF 和 WebP。",
                true,
            ))
        }
    };
    Ok(mime.to_string())
}

pub(super) fn unique_target(directory: &Path, file_name: &str) -> PathBuf {
    let direct = directory.join(file_name);
    if !direct.exists() {
        return direct;
    }
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("image");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    directory.join(format!(
        "{stem}-{}.{}",
        &Uuid::new_v4().to_string()[..8],
        extension
    ))
}

fn asset_url(token: &str) -> String {
    if cfg!(windows) {
        format!("http://hlex-asset.localhost/{token}")
    } else {
        format!("hlex-asset://localhost/{token}")
    }
}
