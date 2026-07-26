use crate::{
    app::{AppState, AssetRecord, AssetSource},
    data::load_config,
    domain::{
        AppError, AppResult, PreviewImageFailureKind, PreviewImageResult, PreviewImageState,
        ResolveArticlePreviewImagesRequest,
    },
};
use percent_encoding::percent_decode_str;
use reqwest::Url;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tauri::State;
use uuid::Uuid;

const MAX_BATCH_IMAGES: usize = 32;
const MAX_FALLBACK_BYTES: usize = 25 * 1024 * 1024;

#[derive(Debug)]
struct ResolvedImage {
    mime: String,
    source: AssetSource,
}

#[derive(Debug)]
struct ResolveFailure {
    kind: PreviewImageFailureKind,
    message: String,
}

impl ResolveFailure {
    fn new(kind: PreviewImageFailureKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

#[tauri::command]
pub async fn resolve_article_preview_images(
    request: ResolveArticlePreviewImagesRequest,
    state: State<'_, AppState>,
) -> AppResult<Vec<PreviewImageResult>> {
    let (root, article_path) = state.with_project(
        &request.project_id,
        Some(request.session_generation),
        |project| {
            Ok((
                project.root.clone(),
                project.article(&request.article_id)?.canonical_path.clone(),
            ))
        },
    )?;
    let config = load_config(&state)?.config;
    expire_assets(&state, &request.project_id, request.session_generation)?;

    if request.sources.len() > MAX_BATCH_IMAGES {
        return Err(AppError::invalid("单次最多解析 32 张预览图片。"));
    }

    let mut results = Vec::with_capacity(request.sources.len());
    for original_source in request.sources {
        // Normal remote images load directly. This command only downloads a URL after
        // WebView reported an error, so an image body returned with HTTP 404 can display.
        if is_remote_source(&original_source) {
            match fetch_remote_fallback(&original_source).await {
                Ok((bytes, mime, status)) if !bytes.is_empty() => {
                    let token = Uuid::new_v4().to_string();
                    let preview_url = asset_url(&token);
                    insert_asset(
                        &state,
                        &request.project_id,
                        request.session_generation,
                        token,
                        mime,
                        AssetSource::Memory(Arc::new(bytes)),
                    )?;
                    results.push(PreviewImageResult {
                        original_source,
                        state: PreviewImageState::Ready,
                        preview_url: Some(preview_url),
                        http_status: Some(status),
                        failure_kind: None,
                        message: None,
                    });
                }
                Ok((_, _, status)) => results.push(unavailable_remote(
                    original_source,
                    Some(status),
                    PreviewImageFailureKind::Empty,
                    "图片返回为空。",
                )),
                Err(message) => results.push(unavailable_remote(
                    original_source,
                    None,
                    PreviewImageFailureKind::Network,
                    &message,
                )),
            }
            continue;
        }

        match resolve_local_image(
            &root,
            &article_path,
            &config.image_bed.local_image_dir,
            &config.image_bed.local_markdown_prefix,
            &original_source,
        ) {
            Ok(image) => {
                let token = Uuid::new_v4().to_string();
                let preview_url = asset_url(&token);
                insert_asset(
                    &state,
                    &request.project_id,
                    request.session_generation,
                    token,
                    image.mime,
                    image.source,
                )?;
                results.push(PreviewImageResult {
                    original_source,
                    state: PreviewImageState::Ready,
                    preview_url: Some(preview_url),
                    http_status: None,
                    failure_kind: None,
                    message: None,
                });
            }
            Err(error) => results.push(PreviewImageResult {
                original_source,
                state: PreviewImageState::Unavailable,
                preview_url: None,
                http_status: None,
                failure_kind: Some(error.kind),
                message: Some(error.message),
            }),
        }
    }
    Ok(results)
}

fn unavailable_remote(
    original_source: String,
    http_status: Option<u16>,
    failure_kind: PreviewImageFailureKind,
    message: &str,
) -> PreviewImageResult {
    PreviewImageResult {
        original_source,
        state: PreviewImageState::Unavailable,
        preview_url: None,
        http_status,
        failure_kind: Some(failure_kind),
        message: Some(message.to_string()),
    }
}

async fn fetch_remote_fallback(original: &str) -> Result<(Vec<u8>, String, u16), String> {
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| format!("无法创建图片请求：{error}"))?
        .get(original)
        .send()
        .await
        .map_err(|error| format!("图片加载失败：{error}"))?;
    let status = response.status().as_u16();
    if response
        .content_length()
        .is_some_and(|length| length > MAX_FALLBACK_BYTES as u64)
    {
        return Err("图片响应超过 25 MB。".to_string());
    }
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("application/octet-stream")
        .trim()
        .to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("读取图片响应失败：{error}"))?;
    if bytes.len() > MAX_FALLBACK_BYTES {
        return Err("图片响应超过 25 MB。".to_string());
    }
    Ok((bytes.to_vec(), mime, status))
}

fn expire_assets(state: &AppState, project_id: &str, generation: u64) -> AppResult<()> {
    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    let project = guard.as_mut().ok_or_else(AppError::session_expired)?;
    project.require_identity(project_id, Some(generation))?;
    let now = SystemTime::now();
    project
        .assets
        .retain(|_, asset| asset.generation == generation && asset.expires_at > now);
    Ok(())
}

fn insert_asset(
    state: &AppState,
    project_id: &str,
    generation: u64,
    token: String,
    mime: String,
    source: AssetSource,
) -> AppResult<()> {
    let mut guard = state
        .project
        .write()
        .map_err(|_| AppError::new("state_poisoned", "项目状态不可用。", false))?;
    let project = guard.as_mut().ok_or_else(AppError::session_expired)?;
    project.require_identity(project_id, Some(generation))?;
    project.assets.insert(
        token,
        AssetRecord {
            source,
            mime,
            generation,
            expires_at: SystemTime::now() + Duration::from_secs(15 * 60),
        },
    );
    Ok(())
}

fn resolve_local_image(
    root: &Path,
    article_path: &Path,
    configured_directory: &str,
    markdown_prefix: &str,
    original: &str,
) -> Result<ResolvedImage, ResolveFailure> {
    if original.trim().is_empty() {
        return Err(ResolveFailure::new(
            PreviewImageFailureKind::InvalidSource,
            "图片地址为空。",
        ));
    }
    if Url::parse(original).is_ok() || original.starts_with("//") {
        return Err(ResolveFailure::new(
            PreviewImageFailureKind::InvalidSource,
            "图片地址无法加载。",
        ));
    }

    let encoded_path = original.split(['?', '#']).next().unwrap_or(original);
    let decoded = percent_decode_str(encoded_path)
        .decode_utf8()
        .map_err(|_| {
            ResolveFailure::new(
                PreviewImageFailureKind::InvalidSource,
                "图片地址包含无效的 URL 编码。",
            )
        })?
        .replace('\\', "/");
    if decoded.split('/').any(|part| matches!(part, "." | "..")) || decoded.contains(':') {
        return Err(ResolveFailure::new(
            PreviewImageFailureKind::UnsafeSource,
            "图片路径越出了允许的项目目录。",
        ));
    }

    let source_root = root.join("source");
    let configured_root =
        root.join(configured_directory.replace('/', std::path::MAIN_SEPARATOR_STR));
    let prefix = markdown_prefix.trim_end_matches('/');
    let prefixed = decoded
        .strip_prefix(prefix)
        .filter(|suffix| suffix.starts_with('/'))
        .map(|suffix| configured_root.join(suffix.trim_start_matches('/')));

    let mut candidates = Vec::new();
    if let Some(candidate) = prefixed {
        candidates.push(candidate);
    } else if decoded.starts_with("source/") || decoded.starts_with("/source/") {
        candidates.push(root.join(decoded.trim_start_matches('/')));
    } else if decoded.starts_with('/') {
        candidates.push(source_root.join(decoded.trim_start_matches('/')));
        candidates.push(
            article_path
                .parent()
                .unwrap_or(&source_root)
                .join(decoded.trim_start_matches('/')),
        );
    } else {
        if let Some(parent) = article_path.parent() {
            candidates.push(parent.join(&decoded));
        }
        candidates.push(article_path.with_extension("").join(&decoded));
        candidates.push(source_root.join(&decoded));
    }

    let canonical_source = source_root.canonicalize().map_err(|_| {
        ResolveFailure::new(
            PreviewImageFailureKind::NotFound,
            "项目 source 目录不可用。",
        )
    })?;
    for candidate in candidates {
        let Ok(canonical) = candidate.canonicalize() else {
            continue;
        };
        if canonical.starts_with(&canonical_source) && canonical.is_file() {
            return read_local_image(canonical);
        }
    }
    Err(ResolveFailure::new(
        PreviewImageFailureKind::NotFound,
        "本地图片不存在或不属于当前项目。",
    ))
}

fn read_local_image(path: PathBuf) -> Result<ResolvedImage, ResolveFailure> {
    let metadata = fs::metadata(&path).map_err(|_| {
        ResolveFailure::new(PreviewImageFailureKind::NotFound, "本地图片无法读取。")
    })?;
    if metadata.len() == 0 {
        return Err(ResolveFailure::new(
            PreviewImageFailureKind::Empty,
            "本地图片内容为空。",
        ));
    }
    Ok(ResolvedImage {
        mime: mime_guess::from_path(&path)
            .first_raw()
            .unwrap_or("application/octet-stream")
            .to_string(),
        source: AssetSource::Disk(path),
    })
}

fn is_remote_source(value: &str) -> bool {
    Url::parse(value).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.username().is_empty()
            && url.password().is_none()
    })
}

fn asset_url(token: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("http://hlex-asset.localhost/{token}")
    } else {
        format!("hlex-asset://localhost/{token}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn recognizes_http_images_without_fetching_them() {
        assert!(is_remote_source("https://example.com/image.png"));
        assert!(is_remote_source("http://127.0.0.1/image.png"));
        assert!(!is_remote_source("file:///image.png"));
    }

    #[test]
    fn resolves_local_images_without_decoding_content() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        let posts = root.join("source/_posts");
        fs::create_dir_all(posts.join("hello")).unwrap();
        fs::create_dir_all(root.join("source/images")).unwrap();
        fs::write(posts.join("hello.md"), "post").unwrap();
        fs::write(
            posts.join("hello/not-really-decoded.png"),
            b"trusted content",
        )
        .unwrap();
        fs::write(root.join("source/images/empty.png"), []).unwrap();
        let article = posts.join("hello.md").canonicalize().unwrap();

        assert!(resolve_local_image(
            root,
            &article,
            "source/images",
            "/images",
            "hello/not-really-decoded.png"
        )
        .is_ok());
        assert!(resolve_local_image(
            root,
            &article,
            "source/images",
            "/images",
            "/images/empty.png"
        )
        .is_err());
        assert!(
            resolve_local_image(root, &article, "source/images", "/images", "../secret.png")
                .is_err()
        );
    }
}
