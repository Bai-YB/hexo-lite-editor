use crate::{
    app::{ArticleRecord, AssetRecord, AssetSource},
    domain::{
        AppError, AppResult, ArticleCover, ArticleCoverSource, ArticleKind, ArticleSummary,
        FrontMatterResult,
    },
};
use chrono::{DateTime, Local};
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime},
};
use url::Url;
use uuid::Uuid;
use walkdir::WalkDir;

pub type ArticleScan = (
    Vec<ArticleSummary>,
    HashMap<String, ArticleRecord>,
    HashMap<String, AssetRecord>,
);

pub fn validate_hexo_root(path: &Path) -> AppResult<(PathBuf, String, Vec<String>)> {
    let root = path
        .canonicalize()
        .map_err(|error| AppError::io("无法读取所选目录", error))?;
    if !root.is_dir() {
        return Err(AppError::invalid("所选路径不是文件夹。"));
    }
    let mut warnings = Vec::new();
    let config = root.join("_config.yml");
    let package = root.join("package.json");
    let posts = root.join("source").join("_posts");
    if !config.is_file() {
        warnings.push("缺少 _config.yml".to_string());
    }
    if !package.is_file() {
        warnings.push("缺少 package.json".to_string());
    }
    if !posts.is_dir() {
        return Err(AppError::new(
            "invalid_hexo_project",
            "未找到 source/_posts，这不是可编辑的 Hexo 项目。",
            true,
        ));
    }
    if !config.is_file() && !package.is_file() {
        return Err(AppError::new(
            "invalid_hexo_project",
            "项目缺少 _config.yml 和 package.json。",
            true,
        ));
    }
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("Hexo Project")
        .to_string();
    Ok((root, name, warnings))
}

pub fn scan_articles(root: &Path) -> AppResult<ArticleScan> {
    let root = root
        .canonicalize()
        .map_err(|error| AppError::io("无法验证项目根目录", error))?;
    let mut summaries = Vec::new();
    let mut records = HashMap::new();
    let mut assets = HashMap::new();
    for (folder, kind) in [
        ("_posts", ArticleKind::Post),
        ("_drafts", ArticleKind::Draft),
    ] {
        let directory = root.join("source").join(folder);
        if !directory.exists() {
            continue;
        }
        let canonical_directory = directory
            .canonicalize()
            .map_err(|error| AppError::io("无法读取文章目录", error))?;
        if !canonical_directory.starts_with(&root) {
            return Err(AppError::new(
                "path_escape",
                "文章目录指向项目之外，已拒绝扫描。",
                false,
            ));
        }

        for entry in WalkDir::new(&canonical_directory)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let path = entry.path();
            if !is_markdown(path) {
                continue;
            }
            let canonical = path
                .canonicalize()
                .map_err(|error| AppError::io("无法解析文章路径", error))?;
            if !canonical.starts_with(&root) {
                continue;
            }
            let article_id = Uuid::new_v4().to_string();
            let (summary, cover_asset) = summarize_article(&root, &canonical, kind, &article_id)?;
            if let Some((token, asset)) = cover_asset {
                assets.insert(token, asset);
            }
            summaries.push(summary);
            records.insert(
                article_id.clone(),
                ArticleRecord {
                    id: article_id,
                    canonical_path: canonical,
                    revision: 0,
                },
            );
        }
    }
    summaries.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.relative_path.cmp(&right.relative_path))
    });
    Ok((summaries, records, assets))
}

pub fn summarize_article(
    root: &Path,
    canonical: &Path,
    kind: ArticleKind,
    article_id: &str,
) -> AppResult<(ArticleSummary, Option<(String, AssetRecord)>)> {
    let content =
        fs::read_to_string(canonical).map_err(|error| AppError::io("无法读取文章", error))?;
    let parsed = parse_front_matter(&content);
    let fallback = canonical
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("未命名文章")
        .to_string();
    let title = parsed
        .attributes
        .get("title")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(&fallback)
        .to_string();
    let relative_path = canonical
        .strip_prefix(root)
        .map_err(|_| AppError::new("path_escape", "文章不属于当前项目。", false))?
        .to_string_lossy()
        .replace('\\', "/");
    let metadata =
        fs::metadata(canonical).map_err(|error| AppError::io("读取文章信息失败", error))?;
    let modified_at = metadata.modified().map(format_time).unwrap_or_default();
    let created_at = metadata.created().ok().map(format_time);
    let front_matter_date = parsed.attributes.get("date").and_then(value_text);
    let tags = parsed
        .attributes
        .get("tags")
        .map(value_list)
        .unwrap_or_default();
    let categories = parsed
        .attributes
        .get("categories")
        .map(value_list)
        .unwrap_or_default();
    let (cover, asset) = resolve_article_cover(root, canonical, &title, &parsed);
    Ok((
        ArticleSummary {
            article_id: article_id.to_string(),
            relative_path,
            title,
            kind,
            front_matter_date,
            created_at,
            modified_at,
            tags,
            categories,
            cover,
            parse_error: parsed.error,
        },
        asset,
    ))
}

fn resolve_article_cover(
    root: &Path,
    article_path: &Path,
    title: &str,
    parsed: &FrontMatterResult,
) -> (ArticleCover, Option<(String, AssetRecord)>) {
    let candidates = [
        ("cover", ArticleCoverSource::Cover),
        ("top_img", ArticleCoverSource::TopImg),
        ("banner", ArticleCoverSource::Banner),
        ("thumbnail", ArticleCoverSource::Thumbnail),
        ("index_img", ArticleCoverSource::IndexImg),
    ];
    for (key, source) in candidates {
        if let Some(value) = parsed.attributes.get(key).and_then(Value::as_str) {
            if let Some((preview_url, asset)) = resolve_cover_url(root, article_path, value) {
                return (
                    ArticleCover {
                        source,
                        preview_url: Some(preview_url),
                        alt: title.to_string(),
                    },
                    asset,
                );
            }
        }
    }
    (
        ArticleCover {
            source: ArticleCoverSource::Placeholder,
            preview_url: None,
            alt: title.to_string(),
        },
        None,
    )
}

fn resolve_cover_url(
    root: &Path,
    article_path: &Path,
    raw: &str,
) -> Option<(String, Option<(String, AssetRecord)>)> {
    let value = raw.trim().trim_matches(['\'', '"']);
    if value.starts_with("https://") {
        return Some((fresh_remote_url(value), None));
    }
    if value.starts_with("http://") || value.starts_with("data:") || value.is_empty() {
        return None;
    }
    let value = value.split(['?', '#']).next().unwrap_or(value);
    let decoded = percent_encoding::percent_decode_str(value)
        .decode_utf8()
        .ok()?
        .replace('\\', "/");
    let source_root = root.join("source");
    let candidate = if decoded.starts_with("/images/") {
        source_root.join(decoded.trim_start_matches('/'))
    } else if decoded.starts_with("source/") {
        root.join(decoded.trim_start_matches('/'))
    } else if decoded.starts_with('/') {
        source_root.join(decoded.trim_start_matches('/'))
    } else {
        let beside_article = article_path.parent()?.join(&decoded);
        if beside_article.is_file() {
            beside_article
        } else {
            source_root.join(&decoded)
        }
    };
    let canonical = candidate.canonicalize().ok()?;
    let canonical_source = source_root.canonicalize().ok()?;
    if !canonical.starts_with(&canonical_source) || !canonical.is_file() {
        return None;
    }
    let mime = image_mime(&canonical)?;
    let token = Uuid::new_v4().to_string();
    Some((
        asset_url(&token),
        Some((
            token,
            AssetRecord {
                source: AssetSource::Disk(canonical),
                mime,
                generation: 0,
                expires_at: SystemTime::now() + Duration::from_secs(15 * 60),
            },
        )),
    ))
}

fn fresh_remote_url(value: &str) -> String {
    let Ok(mut url) = Url::parse(value) else {
        return value.to_string();
    };
    let signed = url.query_pairs().any(|(key, _)| {
        let key = key.to_ascii_lowercase();
        matches!(
            key.as_str(),
            "signature"
                | "sig"
                | "token"
                | "expires"
                | "policy"
                | "key-pair-id"
                | "credential"
                | "auth"
        ) || key.starts_with("x-amz-")
            || key.starts_with("x-goog-")
    });
    if !signed {
        url.query_pairs_mut()
            .append_pair("_hlex_nocache", &Uuid::new_v4().to_string());
    }
    url.to_string()
}

fn value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn value_list(value: &Value) -> Vec<String> {
    match value {
        Value::Array(values) => values.iter().filter_map(value_text).collect(),
        Value::String(value) => value
            .split([',', '，'])
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn image_mime(path: &Path) -> Option<String> {
    match path
        .extension()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => Some("image/png".to_string()),
        "jpg" | "jpeg" => Some("image/jpeg".to_string()),
        "gif" => Some("image/gif".to_string()),
        "webp" => Some("image/webp".to_string()),
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

pub fn parse_front_matter(content: &str) -> FrontMatterResult {
    let normalized = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut lines = normalized.split_inclusive('\n');
    let first = lines.next().unwrap_or_default();
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return FrontMatterResult {
            attributes: Value::Object(Map::new()),
            body: content.to_string(),
            error: None,
        };
    }

    let header_start = first.len();
    let mut cursor = header_start;
    let mut header_end = None;
    let mut body_start = None;
    for line in lines {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" || trimmed == "..." {
            header_end = Some(cursor);
            body_start = Some(cursor + line.len());
            break;
        }
        cursor += line.len();
    }
    let (Some(header_end), Some(body_start)) = (header_end, body_start) else {
        return FrontMatterResult {
            attributes: Value::Object(Map::new()),
            body: content.to_string(),
            error: Some("Front Matter 缺少结束分隔线，已保留原文。".to_string()),
        };
    };

    let header = &normalized[header_start..header_end];
    match serde_yaml::from_str::<serde_yaml::Value>(header) {
        Ok(yaml) => match serde_json::to_value(yaml) {
            Ok(attributes) => FrontMatterResult {
                attributes,
                body: normalized[body_start..].to_string(),
                error: None,
            },
            Err(error) => FrontMatterResult {
                attributes: Value::Object(Map::new()),
                body: content.to_string(),
                error: Some(format!("Front Matter 转换失败：{error}")),
            },
        },
        Err(error) => FrontMatterResult {
            attributes: Value::Object(Map::new()),
            body: content.to_string(),
            error: Some(format!("Front Matter 解析失败：{error}")),
        },
    }
}

pub fn sanitize_article_name(name: &str) -> AppResult<String> {
    let trimmed = name.trim().trim_end_matches(['.', ' ']);
    if trimmed.is_empty() {
        return Err(AppError::invalid("文件名不能为空。"));
    }
    if Path::new(trimmed).is_absolute()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
        || trimmed.contains(':')
    {
        return Err(AppError::invalid("文件名不能包含路径、盘符或 ..。"));
    }
    let invalid = ['<', '>', '"', '|', '?', '*', '\0'];
    let sanitized: String = trimmed
        .chars()
        .filter(|character| !invalid.contains(character))
        .collect();
    let stem = Path::new(&sanitized)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(&sanitized)
        .trim_end_matches(['.', ' ']);
    let upper = stem.to_ascii_uppercase();
    let reserved = matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );
    if reserved {
        return Err(AppError::invalid("文件名是 Windows 保留名称。"));
    }
    let mut final_name = sanitized;
    if !final_name.to_ascii_lowercase().ends_with(".md")
        && !final_name.to_ascii_lowercase().ends_with(".markdown")
    {
        final_name.push_str(".md");
    }
    Ok(final_name)
}

pub fn article_target(root: &Path, kind: ArticleKind, name: &str) -> AppResult<PathBuf> {
    let file_name = sanitize_article_name(name)?;
    validate_relative_path(Path::new(&file_name))?;
    let folder = match kind {
        ArticleKind::Post => "_posts",
        ArticleKind::Draft => "_drafts",
    };
    let directory = root.join("source").join(folder);
    fs::create_dir_all(&directory).map_err(|error| AppError::io("无法创建文章目录", error))?;
    let canonical_directory = directory
        .canonicalize()
        .map_err(|error| AppError::io("无法验证文章目录", error))?;
    if !canonical_directory.starts_with(root) {
        return Err(AppError::new(
            "path_escape",
            "文章目录指向项目之外。",
            false,
        ));
    }
    let target = canonical_directory.join(file_name);
    if target.exists() {
        return Err(AppError::new("article_exists", "同名文章已经存在。", true));
    }
    Ok(target)
}

pub fn validate_relative_path(path: &Path) -> AppResult<()> {
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(AppError::invalid("只允许规范化的相对路径。"));
    }
    Ok(())
}

fn is_markdown(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase()),
        Some(extension) if extension == "md" || extension == "markdown"
    )
}

fn format_time(time: SystemTime) -> String {
    DateTime::<Local>::from(time).to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn keeps_chinese_file_names_and_rejects_traversal() {
        assert_eq!(sanitize_article_name("你好 Hexo").unwrap(), "你好 Hexo.md");
        assert!(sanitize_article_name("../secret.md").is_err());
        assert!(sanitize_article_name("C:\\secret.md").is_err());
        assert!(sanitize_article_name("CON.md").is_err());
    }

    #[test]
    fn preserves_invalid_front_matter() {
        let source = "---\ntitle: [\n---\n正文";
        let parsed = parse_front_matter(source);
        assert!(parsed.error.is_some());
        assert_eq!(parsed.body, source);
    }

    #[test]
    fn scans_posts_and_drafts() {
        let temp = TempDir::new().unwrap();
        let posts = temp.path().join("source/_posts");
        let drafts = temp.path().join("source/_drafts");
        fs::create_dir_all(&posts).unwrap();
        fs::create_dir_all(&drafts).unwrap();
        fs::write(posts.join("文章.md"), "---\ntitle: 已发布\n---\n正文").unwrap();
        fs::write(drafts.join("草稿.md"), "草稿").unwrap();
        let (summaries, _, _) = scan_articles(temp.path()).unwrap();
        assert_eq!(summaries.len(), 2);
        assert!(summaries.iter().any(|item| item.kind == ArticleKind::Draft));
    }

    #[test]
    fn rejects_non_normal_relative_paths() {
        assert!(validate_relative_path(Path::new("../outside")).is_err());
        assert!(validate_relative_path(Path::new("C:\\outside")).is_err());
    }

    #[test]
    fn extracts_metadata_and_uses_cover_priority() {
        let temp = TempDir::new().unwrap();
        let posts = temp.path().join("source/_posts");
        fs::create_dir_all(&posts).unwrap();
        let article = posts.join("中文文章.md");
        fs::write(
            &article,
            "---\ntitle: 中文标题\ndate: 2026-07-17\ntags: [写作, Hexo]\ncategories: 指南\ncover: https://example.com/cover.jpg\ntop_img: https://example.com/top.jpg\n---\n![正文图](https://example.com/body.jpg)",
        )
        .unwrap();
        let root = temp.path().canonicalize().unwrap();
        let canonical = article.canonicalize().unwrap();
        let (summary, asset) =
            summarize_article(&root, &canonical, ArticleKind::Post, "article").unwrap();
        assert_eq!(summary.title, "中文标题");
        assert_eq!(summary.front_matter_date.as_deref(), Some("2026-07-17"));
        assert_eq!(summary.tags, vec!["写作", "Hexo"]);
        assert_eq!(summary.categories, vec!["指南"]);
        assert_eq!(summary.cover.source, ArticleCoverSource::Cover);
        let preview_url = summary.cover.preview_url.as_deref().unwrap();
        assert!(preview_url.starts_with("https://example.com/cover.jpg?_hlex_nocache="));
        assert!(asset.is_none());
    }

    #[test]
    fn refreshes_unsigned_remote_covers_without_changing_signed_urls() {
        let fresh = fresh_remote_url("https://example.com/cover.jpg?size=2#image");
        assert!(fresh.starts_with("https://example.com/cover.jpg?size=2&_hlex_nocache="));
        assert!(fresh.ends_with("#image"));
        assert_eq!(
            fresh_remote_url("https://example.com/cover.jpg?X-Amz-Signature=abc"),
            "https://example.com/cover.jpg?X-Amz-Signature=abc"
        );
    }

    #[test]
    fn body_images_never_become_an_implicit_cover() {
        let parsed = parse_front_matter("正文\n![首图](https://example.com/first.jpg)");
        let (cover, _) =
            resolve_article_cover(Path::new("."), Path::new("article.md"), "标题", &parsed);
        assert_eq!(cover.source, ArticleCoverSource::Placeholder);
        assert!(cover.preview_url.is_none());
    }
}
