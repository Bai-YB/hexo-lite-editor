use crate::{
    app::{AppState, AssetRecord, AssetSource, RemoteAssetRecord},
    data::load_config,
    domain::{
        AppError, AppResult, EditorImageInput, ImageImportResult, ImageProvider, LocalImage,
        RemoteAssetBreadcrumb, RemoteAssetItem, RemoteAssetKind, RemoteAssetPage, UploadResult,
    },
    platform::cloudflare_token,
};
use serde_json::{Map, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;
use url::Url;
use uuid::Uuid;
use walkdir::WalkDir;

const MAX_IMAGE_BYTES: u64 = 25 * 1024 * 1024;

#[tauri::command]
pub fn list_local_images(
    project_id: String,
    session_generation: u64,
    state: State<'_, AppState>,
) -> AppResult<Vec<LocalImage>> {
    list_local_images_impl(&state, &project_id, session_generation)
}

#[tauri::command]
pub fn import_local_images(
    app: AppHandle,
    project_id: String,
    session_generation: u64,
    state: State<'_, AppState>,
) -> AppResult<Vec<LocalImage>> {
    let root = state.with_project(&project_id, Some(session_generation), |project| {
        Ok(project.root.clone())
    })?;
    let config = load_config(&state)?.config;
    let Some(files) = app
        .dialog()
        .file()
        .set_title("导入本地图片")
        .add_filter("图片", &["png", "jpg", "jpeg", "gif", "webp"])
        .blocking_pick_files()
    else {
        return Ok(Vec::new());
    };
    let canonical_target = local_image_directory(&root, &config.image_bed.local_image_dir)?;

    for file in files {
        let source = file
            .into_path()
            .map_err(|error| AppError::invalid(error.to_string()))?;
        validate_image_file(&source)?;
        let file_name = source
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| AppError::invalid("图片文件名不是有效文本。"))?;
        let target = unique_target(&canonical_target, file_name);
        fs::copy(&source, target).map_err(|error| AppError::io("导入图片失败", error))?;
    }
    list_local_images_impl(&state, &project_id, session_generation)
}

#[tauri::command]
pub fn delete_local_image(
    project_id: String,
    session_generation: u64,
    image_id: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let path = state.with_project(&project_id, Some(session_generation), |project| {
        project
            .assets
            .get(&image_id)
            .and_then(|asset| match &asset.source {
                AssetSource::Disk(path) => Some(path.clone()),
                AssetSource::Memory(_) => None,
            })
            .ok_or_else(|| AppError::new("image_not_found", "图片令牌无效或已过期。", true))
    })?;
    trash::delete(&path).map_err(|error| AppError::io("移动图片到回收站失败", error))?;
    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    if let Some(project) = guard.as_mut() {
        project.require_identity(&project_id, Some(session_generation))?;
        project.assets.remove(&image_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn upload_cloudflare_image(
    app: AppHandle,
    project_id: String,
    session_generation: u64,
    state: State<'_, AppState>,
) -> AppResult<Option<UploadResult>> {
    state.with_project(&project_id, Some(session_generation), |_| Ok(()))?;
    let config = load_config(&state)?.config;
    let endpoint = cloudflare_upload_endpoint(
        &config.image_bed.cloudflare_api_url,
        &config.image_bed.upload_folder,
    )?;
    let token = cloudflare_token()?;
    let Some(file) = app
        .dialog()
        .file()
        .set_title("上传到 Cloudflare-ImgBed")
        .add_filter("图片", &["png", "jpg", "jpeg", "gif", "webp"])
        .blocking_pick_file()
    else {
        return Ok(None);
    };
    let path = file
        .into_path()
        .map_err(|error| AppError::invalid(error.to_string()))?;
    validate_image_file(&path)?;
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| AppError::invalid("图片文件名不是有效文本。"))?
        .to_string();
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|error| AppError::io("读取待上传图片失败", error))?;
    let mime = supported_mime(&path)?;
    let url = upload_cloudflare_bytes(&endpoint, &token, &file_name, &mime, bytes).await?;
    Ok(Some(UploadResult {
        markdown: format!("![{file_name}]({url})"),
        url,
        file_name,
    }))
}

#[tauri::command]
pub async fn import_editor_images(
    project_id: String,
    session_generation: u64,
    provider: ImageProvider,
    files: Vec<EditorImageInput>,
    state: State<'_, AppState>,
) -> AppResult<Vec<ImageImportResult>> {
    let root = state.with_project(&project_id, Some(session_generation), |project| {
        Ok(project.root.clone())
    })?;
    let config = load_config(&state)?.config;
    let remote = if provider == ImageProvider::CloudflareImgbed {
        Some((
            cloudflare_upload_endpoint(
                &config.image_bed.cloudflare_api_url,
                &config.image_bed.upload_folder,
            )?,
            cloudflare_token()?,
        ))
    } else {
        None
    };
    let mut results = Vec::with_capacity(files.len());
    for file in files {
        let file_name = file.name.clone();
        let result = match validate_image_input(&file).and_then(|mime| {
            if provider == ImageProvider::Local {
                save_local_editor_image(
                    &root,
                    &config.image_bed.local_image_dir,
                    &config.image_bed.local_markdown_prefix,
                    &file.name,
                    &file.bytes,
                )
                .map(|url| (url, mime))
            } else {
                Ok((String::new(), mime))
            }
        }) {
            Ok((url, _mime)) if provider == ImageProvider::Local => Ok(url),
            Ok((_, mime)) => match remote.as_ref() {
                Some((endpoint, token)) => {
                    upload_cloudflare_bytes(endpoint, token, &file.name, &mime, file.bytes).await
                }
                None => Err(AppError::new(
                    "image_provider_unavailable",
                    "远程图床配置不可用。",
                    true,
                )),
            },
            Err(error) => Err(error),
        };
        match result {
            Ok(url) => results.push(ImageImportResult {
                file_name: file_name.clone(),
                markdown: Some(format!("![{file_name}]({url})")),
                url: Some(url),
                error: None,
            }),
            Err(error) => results.push(ImageImportResult {
                file_name,
                url: None,
                markdown: None,
                error: Some(error),
            }),
        }
    }
    Ok(results)
}

#[tauri::command]
pub async fn list_cloudflare_assets(
    project_id: String,
    session_generation: u64,
    offset: usize,
    count: usize,
    search: String,
    directory: String,
    state: State<'_, AppState>,
) -> AppResult<RemoteAssetPage> {
    state.with_project(&project_id, Some(session_generation), |_| Ok(()))?;
    let config = load_config(&state)?.config;
    let base = validate_cloudflare_url(&config.image_bed.cloudflare_api_url)?;
    let mut endpoint = cloudflare_api_endpoint(&base, "api/manage/list")?;
    endpoint
        .query_pairs_mut()
        .append_pair("start", &offset.to_string())
        .append_pair("count", &count.clamp(1, 100).to_string())
        .append_pair(
            "recursive",
            if search.trim().is_empty() {
                "false"
            } else {
                "true"
            },
        )
        .append_pair("search", search.trim())
        .append_pair("dir", directory.trim());
    let response = reqwest::Client::new()
        .get(endpoint)
        .bearer_auth(cloudflare_token()?)
        .send()
        .await
        .map_err(|error| AppError::new("image_list_failed", error.to_string(), true))?;
    let status = response.status();
    let value: Value = response
        .json()
        .await
        .map_err(|error| AppError::new("image_list_invalid", error.to_string(), true))?;
    if !status.is_success() {
        return Err(AppError::new(
            "image_list_failed",
            format!("Cloudflare-ImgBed 返回 HTTP {status}。"),
            true,
        ));
    }
    let (page, records) = normalize_remote_page(
        &base,
        &value,
        offset,
        count.clamp(1, 100),
        &search,
        &directory,
    );
    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    let project = guard.as_mut().ok_or_else(AppError::session_expired)?;
    project.require_identity(&project_id, Some(session_generation))?;
    project.remote_assets.extend(records);
    Ok(page)
}

#[tauri::command]
pub async fn delete_cloudflare_asset(
    project_id: String,
    session_generation: u64,
    asset_id: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let delete_key = state.with_project(&project_id, Some(session_generation), |project| {
        project
            .remote_assets
            .get(&asset_id)
            .and_then(|record| {
                (record.kind != RemoteAssetKind::Folder).then(|| record.delete_key.clone())
            })
            .ok_or_else(|| {
                AppError::new(
                    "remote_asset_not_found",
                    "远程资源已失效或不允许删除，请刷新后重试。",
                    true,
                )
            })
    })?;
    let config = load_config(&state)?.config;
    let base = validate_cloudflare_url(&config.image_bed.cloudflare_api_url)?;
    let response = reqwest::Client::new()
        .delete(cloudflare_delete_endpoint(&base, &delete_key)?)
        .bearer_auth(cloudflare_token()?)
        .send()
        .await
        .map_err(|error| AppError::new("remote_delete_failed", error.to_string(), true))?;
    if !response.status().is_success() {
        return Err(AppError::new(
            "remote_delete_failed",
            format!("删除失败，图床返回 HTTP {}。", response.status()),
            true,
        ));
    }
    if let Ok(mut guard) = state.project.write() {
        if let Some(project) = guard.as_mut() {
            project.remote_assets.remove(&asset_id);
        }
    }
    Ok(())
}

#[tauri::command]
pub fn reveal_local_image(
    project_id: String,
    session_generation: u64,
    image_id: String,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let path = state.with_project(&project_id, Some(session_generation), |project| {
        project
            .assets
            .get(&image_id)
            .and_then(|asset| match &asset.source {
                AssetSource::Disk(path) => Some(path.clone()),
                AssetSource::Memory(_) => None,
            })
            .ok_or_else(|| AppError::new("image_not_found", "图片资源已失效，请刷新后重试。", true))
    })?;
    #[cfg(windows)]
    {
        Command::new("explorer.exe")
            .arg(format!("/select,{}", path.display()))
            .spawn()
            .map_err(|error| AppError::io("在文件夹中显示图片失败", error))?;
    }
    #[cfg(not(windows))]
    {
        open::that_detached(path.parent().unwrap_or(Path::new(".")))
            .map_err(|error| AppError::io("打开图片文件夹失败", error))?;
    }
    Ok(())
}

async fn upload_cloudflare_bytes(
    endpoint: &Url,
    token: &str,
    file_name: &str,
    mime: &str,
    bytes: Vec<u8>,
) -> AppResult<String> {
    let part = reqwest::multipart::Part::bytes(bytes)
        .file_name(file_name.to_string())
        .mime_str(mime)
        .map_err(|error| AppError::invalid(error.to_string()))?;
    let response = reqwest::Client::new()
        .post(endpoint.clone())
        .bearer_auth(token)
        .header("authCode", token)
        .multipart(reqwest::multipart::Form::new().part("file", part))
        .send()
        .await
        .map_err(|error| AppError::new("upload_failed", error.to_string(), true))?;
    let status = response.status();
    let value: Value = response
        .json()
        .await
        .map_err(|error| AppError::new("upload_response_invalid", error.to_string(), true))?;
    if !status.is_success() {
        return Err(AppError::new(
            "upload_failed",
            format!("图床返回 HTTP {status}。"),
            true,
        ));
    }
    let raw_url = find_url(&value).ok_or_else(|| {
        AppError::new(
            "upload_response_invalid",
            "上传成功，但响应中没有可用图片 URL。",
            true,
        )
    })?;
    normalize_remote_url(endpoint, &raw_url).ok_or_else(|| {
        AppError::new(
            "upload_response_invalid",
            "上传结果中的图片地址无效或不安全。",
            true,
        )
    })
}

fn save_local_editor_image(
    root: &Path,
    image_dir: &str,
    markdown_prefix: &str,
    file_name: &str,
    bytes: &[u8],
) -> AppResult<String> {
    let file_name = safe_image_name(file_name)?;
    let canonical_directory = local_image_directory(root, image_dir)?;
    let target = unique_target(&canonical_directory, &file_name);
    crate::platform::atomic_write(&target, bytes)?;
    local_markdown_url(
        markdown_prefix,
        target
            .strip_prefix(&canonical_directory)
            .map_err(|_| AppError::new("path_escape", "图片不属于配置目录。", false))?,
    )
}

fn local_image_directory(root: &Path, configured: &str) -> AppResult<PathBuf> {
    let normalized = configured.trim().replace('\\', "/");
    let segments = normalized.split('/').collect::<Vec<_>>();
    if segments.len() < 2
        || segments.first() != Some(&"source")
        || segments
            .iter()
            .any(|segment| segment.is_empty() || matches!(*segment, "." | ".."))
    {
        return Err(AppError::invalid(
            "本地图片目录必须是 source/ 下且不含路径穿越的相对路径。",
        ));
    }

    let canonical_root = root
        .canonicalize()
        .map_err(|error| AppError::io("验证项目目录失败", error))?;
    let mut canonical_directory = canonical_root.clone();
    for segment in segments {
        let candidate = canonical_directory.join(segment);
        if !candidate.exists() {
            fs::create_dir(&candidate).map_err(|error| AppError::io("创建图片目录失败", error))?;
        }
        canonical_directory = candidate
            .canonicalize()
            .map_err(|error| AppError::io("验证图片目录失败", error))?;
        if !canonical_directory.starts_with(&canonical_root) {
            return Err(AppError::new(
                "path_escape",
                "图片目录指向项目之外。",
                false,
            ));
        }
    }
    Ok(canonical_directory)
}

fn local_markdown_url(prefix: &str, relative_path: &Path) -> AppResult<String> {
    let prefix = prefix.trim();
    if !prefix.starts_with('/')
        || prefix.starts_with("//")
        || prefix.contains('?')
        || prefix.contains('#')
        || prefix.contains('%')
        || prefix.contains('\\')
        || prefix
            .split('/')
            .skip(1)
            .any(|segment| matches!(segment, "." | ".."))
    {
        return Err(AppError::invalid("Markdown 访问前缀无效。"));
    }

    let mut url = Url::parse("https://hlex.local/").expect("static URL");
    let base_path = if prefix == "/" {
        "/".to_string()
    } else {
        format!("{}/", prefix.trim_end_matches('/'))
    };
    url.set_path(&base_path);
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| AppError::invalid("无法生成 Markdown 图片路径。"))?;
        path.pop_if_empty();
        for component in relative_path.components() {
            let std::path::Component::Normal(segment) = component else {
                return Err(AppError::invalid("图片相对路径无效。"));
            };
            path.push(
                segment
                    .to_str()
                    .ok_or_else(|| AppError::invalid("图片路径不是有效文本。"))?,
            );
        }
    }
    Ok(url.path().to_string())
}

fn validate_image_input(file: &EditorImageInput) -> AppResult<String> {
    if file.bytes.len() as u64 > MAX_IMAGE_BYTES {
        return Err(AppError::new(
            "image_too_large",
            "图片不能超过 25 MB。",
            true,
        ));
    }
    if file.bytes.is_empty() {
        return Err(AppError::invalid("图片内容为空。"));
    }
    let file_name = safe_image_name(&file.name)?;
    let mime = supported_mime(Path::new(&file_name))?;
    if file.mime != mime || !matches_image_signature(&mime, &file.bytes) {
        return Err(AppError::new(
            "image_mime_mismatch",
            "图片扩展名、MIME 与文件内容不一致。",
            true,
        ));
    }
    Ok(mime)
}

fn safe_image_name(value: &str) -> AppResult<String> {
    let path = Path::new(value.trim());
    if path.is_absolute()
        || path.components().count() != 1
        || value.contains('/')
        || value.contains('\\')
        || value.contains("..")
    {
        return Err(AppError::invalid("图片文件名不能包含路径或 ..。"));
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("图片文件名无效。"))?;
    supported_mime(Path::new(file_name))?;
    Ok(file_name.to_string())
}

fn matches_image_signature(mime: &str, bytes: &[u8]) -> bool {
    match mime {
        "image/png" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "image/jpeg" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "image/gif" => bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a"),
        "image/webp" => bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP",
        _ => false,
    }
}

fn cloudflare_upload_endpoint(value: &str, upload_folder: &str) -> AppResult<Url> {
    let base = validate_cloudflare_url(value)?;
    let mut endpoint = if base.path().trim_end_matches('/').ends_with("/upload") {
        base
    } else {
        cloudflare_api_endpoint(&base, "upload")?
    };
    let mut pairs = endpoint.query_pairs_mut();
    pairs
        .append_pair("returnFormat", "full")
        .append_pair("uploadNameType", "origin");
    let folder = upload_folder.trim().trim_matches('/');
    if !folder.is_empty() {
        pairs.append_pair("uploadFolder", folder);
    }
    drop(pairs);
    Ok(endpoint)
}

fn cloudflare_api_endpoint(base: &Url, path: &str) -> AppResult<Url> {
    let mut endpoint = base.clone();
    endpoint.set_path("/");
    endpoint.set_query(None);
    endpoint
        .join(path.trim_start_matches('/'))
        .map_err(|_| AppError::invalid("无法生成 Cloudflare-ImgBed API 地址。"))
}

fn cloudflare_delete_endpoint(base: &Url, delete_key: &str) -> AppResult<Url> {
    let mut endpoint = cloudflare_api_endpoint(base, "api/manage/delete")?;
    let segments = delete_key
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| matches!(*segment, "." | "..") || segment.contains('\\'))
    {
        return Err(AppError::invalid("远程资源删除键无效。"));
    }
    endpoint
        .path_segments_mut()
        .map_err(|_| AppError::invalid("无法生成远程图片删除地址。"))?
        .extend(segments);
    Ok(endpoint)
}

fn normalize_remote_page(
    base: &Url,
    value: &Value,
    offset: usize,
    _count: usize,
    search: &str,
    directory: &str,
) -> (
    RemoteAssetPage,
    std::collections::HashMap<String, RemoteAssetRecord>,
) {
    use std::collections::{HashMap, HashSet};

    let values = remote_item_values(value);
    let needle = search.trim().to_lowercase();
    let current_directory = normalize_directory(directory);
    let mut records = HashMap::new();
    let mut items = Vec::new();
    let mut known_folders = HashSet::new();

    for raw_directory in remote_directory_values(value)
        .into_iter()
        .filter(|_| needle.is_empty())
    {
        let normalized = normalize_directory(&raw_directory);
        let Some(child) = direct_child_directory(&current_directory, &normalized) else {
            continue;
        };
        if !known_folders.insert(child.clone()) {
            continue;
        }
        let asset_id = Uuid::new_v4().to_string();
        records.insert(
            asset_id.clone(),
            RemoteAssetRecord {
                delete_key: child.clone(),
                kind: RemoteAssetKind::Folder,
            },
        );
        items.push(RemoteAssetItem {
            asset_id,
            kind: RemoteAssetKind::Folder,
            name: child.rsplit('/').next().unwrap_or(&child).to_string(),
            file_name: child.rsplit('/').next().unwrap_or(&child).to_string(),
            directory: child,
            extension: None,
            mime: None,
            size: None,
            created_at: None,
            url: None,
            preview_url: None,
            can_preview: false,
        });
    }

    for value in values {
        let Some(object) = value.as_object() else {
            continue;
        };
        let metadata = object.get("metadata").and_then(Value::as_object);
        let identifier =
            object_value(object, metadata, &["name", "id", "key", "fileId"]).unwrap_or_default();
        let raw_url = object_value(
            object,
            metadata,
            &[
                "publicUrl",
                "src",
                "url",
                "fileUrl",
                "fileURL",
                "link",
                "downloadUrl",
                "raw",
            ],
        )
        .or_else(|| (!identifier.is_empty()).then(|| format!("/file/{identifier}")));
        let url = raw_url
            .as_deref()
            .and_then(|raw_url| normalize_remote_url(base, raw_url));
        let file_name = object_value(
            object,
            metadata,
            &["fileName", "file_name", "FileName", "filename"],
        )
        .unwrap_or_else(|| {
            identifier
                .rsplit('/')
                .next()
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| {
                    url.as_deref()
                        .and_then(|url| url.rsplit('/').next())
                        .unwrap_or("file")
                })
                .to_string()
        });
        if !needle.is_empty() && !file_name.to_lowercase().contains(&needle) {
            continue;
        }
        let name = if identifier.is_empty() {
            file_name.clone()
        } else {
            identifier.clone()
        };
        let item_directory = identifier
            .rsplit_once('/')
            .map(|(directory, _)| normalize_directory(directory))
            .unwrap_or_else(|| current_directory.clone());
        if needle.is_empty() && item_directory != current_directory {
            continue;
        }
        let delete_key = object_value(
            object,
            metadata,
            &["name", "id", "key", "fileId", "fileName", "filename"],
        )
        .unwrap_or_else(|| name.clone());
        let mime = object_value(
            object,
            metadata,
            &[
                "mime",
                "contentType",
                "type",
                "fileType",
                "file_type",
                "FileType",
            ],
        );
        let extension = Path::new(&file_name)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase());
        let kind = classify_remote_asset(extension.as_deref(), mime.as_deref());
        let asset_id = Uuid::new_v4().to_string();
        records.insert(asset_id.clone(), RemoteAssetRecord { delete_key, kind });
        items.push(RemoteAssetItem {
            asset_id,
            kind,
            name,
            file_name,
            directory: item_directory,
            extension,
            mime,
            size: object_value(
                object,
                metadata,
                &["size", "fileSize", "file_size", "FileSize", "FileSizeBytes"],
            )
            .and_then(|value| value.parse::<u64>().ok()),
            created_at: object_value(
                object,
                metadata,
                &["createdAt", "created_at", "time", "uploadedAt", "TimeStamp"],
            ),
            preview_url: (kind == RemoteAssetKind::Image)
                .then(|| url.clone())
                .flatten(),
            url,
            can_preview: kind == RemoteAssetKind::Image,
        });
    }
    let total_count = ["/data/totalCount", "/data/total", "/totalCount", "/total"]
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_u64))
        .map(|value| value as usize)
        .unwrap_or(offset + items.len());
    items.sort_by(|left, right| {
        (
            left.kind != RemoteAssetKind::Folder,
            left.name.to_lowercase(),
        )
            .cmp(&(
                right.kind != RemoteAssetKind::Folder,
                right.name.to_lowercase(),
            ))
    });
    let returned_count = items.len();
    let next_offset = (offset + returned_count < total_count && returned_count > 0)
        .then_some(offset + returned_count);
    (
        RemoteAssetPage {
            current_directory: current_directory.clone(),
            breadcrumbs: breadcrumbs(&current_directory),
            items,
            total_count,
            returned_count,
            next_offset,
        },
        records,
    )
}

fn remote_directory_values(value: &Value) -> Vec<String> {
    ["/data/directories", "/directories", "/data/dirs", "/dirs"]
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_array))
        .map(|values| {
            values
                .iter()
                .filter_map(|value| {
                    value.as_str().map(str::to_string).or_else(|| {
                        value.as_object().and_then(|object| {
                            object_value(object, None, &["path", "name", "directory"])
                        })
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_directory(value: &str) -> String {
    value
        .trim()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}

fn direct_child_directory(current: &str, candidate: &str) -> Option<String> {
    let remainder = if current.is_empty() {
        candidate
    } else {
        candidate.strip_prefix(&format!("{current}/"))?
    };
    let child = remainder.split('/').next()?.trim();
    if child.is_empty() {
        None
    } else if current.is_empty() {
        Some(child.to_string())
    } else {
        Some(format!("{current}/{child}"))
    }
}

fn breadcrumbs(directory: &str) -> Vec<RemoteAssetBreadcrumb> {
    let mut result = vec![RemoteAssetBreadcrumb {
        name: "根目录".to_string(),
        directory: String::new(),
    }];
    let mut current = String::new();
    for segment in directory.split('/').filter(|segment| !segment.is_empty()) {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(segment);
        result.push(RemoteAssetBreadcrumb {
            name: segment.to_string(),
            directory: current.clone(),
        });
    }
    result
}

fn classify_remote_asset(extension: Option<&str>, mime: Option<&str>) -> RemoteAssetKind {
    let mime = mime.unwrap_or_default().to_ascii_lowercase();
    if mime.starts_with("image/")
        || matches!(
            extension,
            Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "avif")
        )
    {
        RemoteAssetKind::Image
    } else if matches!(
        extension,
        Some("zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz")
    ) {
        RemoteAssetKind::Archive
    } else if mime.starts_with("audio/")
        || matches!(extension, Some("mp3" | "wav" | "flac" | "m4a" | "ogg"))
    {
        RemoteAssetKind::Audio
    } else if mime.starts_with("video/")
        || matches!(extension, Some("mp4" | "webm" | "mov" | "mkv" | "avi"))
    {
        RemoteAssetKind::Video
    } else if mime.starts_with("text/")
        || matches!(
            extension,
            Some("pdf" | "doc" | "docx" | "txt" | "md" | "csv" | "xls" | "xlsx" | "ppt" | "pptx")
        )
    {
        RemoteAssetKind::Document
    } else {
        RemoteAssetKind::File
    }
}

fn remote_item_values(value: &Value) -> Vec<&Value> {
    for pointer in [
        "/data/files",
        "/data/items",
        "/data/list",
        "/files",
        "/items",
        "/list",
        "/data",
    ] {
        if let Some(values) = value.pointer(pointer).and_then(Value::as_array) {
            return values.iter().collect();
        }
    }
    value
        .as_array()
        .map(|values| values.iter().collect())
        .unwrap_or_default()
}

fn normalize_remote_url(base: &Url, raw: &str) -> Option<String> {
    let url = base.join(raw).ok()?;
    (url.scheme() == "https"
        || (cfg!(debug_assertions) && matches!(url.host_str(), Some("localhost" | "127.0.0.1"))))
    .then(|| url.to_string())
}

fn value_identifier(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn object_value(
    object: &Map<String, Value>,
    metadata: Option<&Map<String, Value>>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(value_identifier))
        .or_else(|| {
            metadata.and_then(|metadata| {
                keys.iter()
                    .find_map(|key| metadata.get(*key).and_then(value_identifier))
            })
        })
        .filter(|value| !value.trim().is_empty())
}

fn list_local_images_impl(
    state: &AppState,
    project_id: &str,
    generation: u64,
) -> AppResult<Vec<LocalImage>> {
    let root = state.with_project(project_id, Some(generation), |project| {
        Ok(project.root.clone())
    })?;
    let config = load_config(state)?.config;
    let canonical_directory = local_image_directory(&root, &config.image_bed.local_image_dir)?;
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
        let markdown_url = local_markdown_url(
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

fn validate_image_file(path: &Path) -> AppResult<()> {
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

fn supported_mime(path: &Path) -> AppResult<String> {
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

fn unique_target(directory: &Path, file_name: &str) -> PathBuf {
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

fn validate_cloudflare_url(value: &str) -> AppResult<Url> {
    let url = Url::parse(value.trim()).map_err(|_| AppError::invalid("图床 API 地址无效。"))?;
    let is_local = matches!(url.host_str(), Some("localhost" | "127.0.0.1"));
    if url.scheme() != "https" && !(cfg!(debug_assertions) && is_local && url.scheme() == "http") {
        return Err(AppError::new(
            "insecure_endpoint",
            "图床 API 必须使用 HTTPS；开发模式仅允许本机 HTTP。",
            true,
        ));
    }
    Ok(url)
}

fn find_url(value: &Value) -> Option<String> {
    match value {
        Value::String(text)
            if text.starts_with("https://")
                || text.starts_with("/file/")
                || (cfg!(debug_assertions) && text.starts_with("http://localhost")) =>
        {
            Some(text.clone())
        }
        Value::Array(values) => values.iter().find_map(find_url),
        Value::Object(map) => [
            "publicUrl",
            "url",
            "src",
            "link",
            "fileUrl",
            "fileURL",
            "downloadUrl",
        ]
        .iter()
        .filter_map(|key| map.get(*key))
        .find_map(find_url)
        .or_else(|| map.values().find_map(find_url)),
        _ => None,
    }
}

fn asset_url(token: &str) -> String {
    if cfg!(windows) {
        format!("http://hlex-asset.localhost/{token}")
    } else {
        format!("hlex-asset://localhost/{token}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn rejects_svg_and_insecure_remote_endpoints() {
        assert!(supported_mime(Path::new("icon.svg")).is_err());
        assert!(validate_cloudflare_url("http://example.com/upload").is_err());
    }

    #[test]
    fn normalizes_cloudflare_metadata_and_relative_urls() {
        let base = Url::parse("https://img.example.com/upload").unwrap();
        let value = json!({
            "files": [{
                "name": "posts/中文 图片.jpg",
                "metadata": {
                    "FileName": "中文 图片.jpg",
                    "FileType": "image/jpeg",
                    "FileSizeBytes": "42",
                    "TimeStamp": "2026-07-17T12:00:00Z",
                    "Channel": "posts"
                }
            }],
            "directories": ["posts"],
            "totalCount": 1
        });
        let (page, records) = normalize_remote_page(&base, &value, 0, 50, "中文", "");
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].file_name, "中文 图片.jpg");
        assert_eq!(page.items[0].size, Some(42));
        assert_eq!(page.items[0].kind, RemoteAssetKind::Image);
        assert_eq!(page.items[0].directory, "posts");
        assert!(page.items[0]
            .url
            .as_deref()
            .unwrap()
            .contains("/file/posts/"));
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn preserves_delete_path_segments_and_upload_response_options() {
        let base = Url::parse("https://img.example.com/upload").unwrap();
        let delete = cloudflare_delete_endpoint(&base, "posts/中文 图片.jpg").unwrap();
        assert!(delete.as_str().contains("/api/manage/delete/posts/"));
        assert!(!delete.as_str().contains("%2F"));
        let upload = cloudflare_upload_endpoint(base.as_str(), "blog/2026").unwrap();
        assert!(upload.query().unwrap().contains("returnFormat=full"));
        assert!(upload.query().unwrap().contains("uploadFolder=blog%2F2026"));
        assert_eq!(
            find_url(&json!({ "data": { "src": "/file/a.jpg" } })).as_deref(),
            Some("/file/a.jpg")
        );
        assert!(cloudflare_delete_endpoint(&base, "../config").is_err());
    }

    #[test]
    fn validates_local_image_paths_and_generates_encoded_markdown_urls() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("blog");
        fs::create_dir(&root).unwrap();

        let directory = local_image_directory(&root, "source/images").unwrap();
        assert_eq!(
            directory,
            root.join("source/images").canonicalize().unwrap()
        );
        assert!(local_image_directory(&root, "../outside").is_err());
        assert!(local_image_directory(&root, "C:/outside").is_err());
        assert_eq!(
            local_markdown_url("/images", Path::new("中文 图片.jpg")).unwrap(),
            "/images/%E4%B8%AD%E6%96%87%20%E5%9B%BE%E7%89%87.jpg"
        );
        assert!(local_markdown_url("/images/../private", Path::new("a.jpg")).is_err());

        let linked_root = temp.path().join("linked-blog");
        let outside = temp.path().join("outside");
        fs::create_dir(&linked_root).unwrap();
        fs::create_dir(&outside).unwrap();
        #[cfg(windows)]
        let linked = std::os::windows::fs::symlink_dir(&outside, linked_root.join("source"));
        #[cfg(unix)]
        let linked = std::os::unix::fs::symlink(&outside, linked_root.join("source"));
        if linked.is_ok() {
            assert!(local_image_directory(&linked_root, "source/images").is_err());
            assert!(!outside.join("images").exists());
        }
    }

    #[test]
    fn classifies_remote_files_without_broken_image_fallbacks() {
        assert_eq!(
            classify_remote_asset(Some("7z"), None),
            RemoteAssetKind::Archive
        );
        assert_eq!(
            classify_remote_asset(Some("rar"), None),
            RemoteAssetKind::Archive
        );
        assert_eq!(
            classify_remote_asset(Some("pdf"), None),
            RemoteAssetKind::Document
        );
        assert_eq!(
            classify_remote_asset(Some("bin"), None),
            RemoteAssetKind::File
        );
        assert_eq!(
            classify_remote_asset(None, Some("image/webp")),
            RemoteAssetKind::Image
        );
        assert_eq!(
            direct_child_directory("", "blog/course/assets").as_deref(),
            Some("blog")
        );
        assert_eq!(
            direct_child_directory("blog", "blog/course/assets").as_deref(),
            Some("blog/course")
        );
    }
}
