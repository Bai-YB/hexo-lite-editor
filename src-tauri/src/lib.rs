use chrono::Local;
use serde::Serialize;
use serde_json::{json, Value as JsonValue};
use serde_yaml::Value;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};
use walkdir::WalkDir;

#[derive(Default)]
struct HexoServerState {
    child: Mutex<Option<Child>>,
}

#[derive(Serialize)]
struct ProjectValidation {
    root_path: String,
    posts_path: String,
    config_path: String,
    package_json_path: Option<String>,
    name: String,
    is_valid: bool,
    warnings: Vec<String>,
}

#[derive(Serialize)]
struct PostMeta {
    id: String,
    title: String,
    file_name: String,
    file_path: String,
    date: Option<String>,
    cover: Option<String>,
    top_img: Option<String>,
    banner: Option<String>,
    thumbnail: Option<String>,
    index_img: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    categories: Vec<String>,
    created_at: Option<String>,
    updated_at: Option<String>,
    is_draft: bool,
}

#[derive(Serialize)]
struct CommandResult {
    success: bool,
    command: String,
    stdout: String,
    stderr: String,
    code: Option<i32>,
}

#[derive(Serialize)]
struct UploadResult {
    url: String,
    markdown: String,
    file_path: String,
}

#[derive(Serialize)]
struct ImageBedItem {
    id: String,
    name: String,
    url: String,
    file_name: String,
    file_type: String,
    file_size: String,
    created_at: String,
    channel: String,
    raw: JsonValue,
}

#[derive(Serialize)]
struct ImageBedListResult {
    files: Vec<ImageBedItem>,
    directories: Vec<String>,
    total_count: u64,
    returned_count: u64,
}

#[derive(Serialize)]
struct HexoConfigFile {
    exists: bool,
    project_path: String,
    config_path: String,
    content: String,
    latest_backup_path: Option<String>,
}

#[derive(Serialize)]
struct BackupResult {
    backup_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateSettings {
    update_source: String,
    github_owner: Option<String>,
    github_repo: Option<String>,
    custom_update_url: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateCheckResult {
    current_version: String,
    latest_version: String,
    has_update: bool,
    release_notes: Option<String>,
    download_url: Option<String>,
    release_page_url: Option<String>,
}

#[tauri::command]
fn validate_hexo_project(path: String) -> Result<ProjectValidation, String> {
    let root = PathBuf::from(path);
    let posts = root.join("source").join("_posts");
    let config = root.join("_config.yml");
    let package_json = root.join("package.json");
    let mut warnings = Vec::new();

    if !config.exists() {
        warnings.push("缺少 _config.yml".to_string());
    }
    if !package_json.exists() {
        warnings.push("缺少 package.json".to_string());
    }
    if !posts.exists() {
        warnings.push("缺少 source/_posts".to_string());
    }

    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Hexo Project")
        .to_string();

    Ok(ProjectValidation {
        root_path: path_string(&root),
        posts_path: path_string(&posts),
        config_path: path_string(&config),
        package_json_path: package_json.exists().then(|| path_string(&package_json)),
        name,
        is_valid: posts.exists() && (config.exists() || package_json.exists()),
        warnings,
    })
}

#[tauri::command]
fn scan_posts(project_path: String) -> Result<Vec<PostMeta>, String> {
    let posts_dir = PathBuf::from(project_path).join("source").join("_posts");
    if !posts_dir.exists() {
        return Err("source/_posts 不存在".to_string());
    }

    let mut posts = Vec::new();
    for entry in WalkDir::new(posts_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if !is_markdown_file(path) {
            continue;
        }

        let content = fs::read_to_string(path).unwrap_or_default();
        let front_matter = parse_front_matter(&content);
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string();
        let fallback_title = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(&file_name)
            .to_string();
        let title = yaml_string(&front_matter, "title").unwrap_or(fallback_title);
        let metadata = fs::metadata(path).ok();

        posts.push(PostMeta {
            id: path_string(path),
            title,
            file_name,
            file_path: path_string(path),
            date: yaml_string(&front_matter, "date"),
            cover: yaml_string(&front_matter, "cover"),
            top_img: yaml_string(&front_matter, "top_img"),
            banner: yaml_string(&front_matter, "banner"),
            thumbnail: yaml_string(&front_matter, "thumbnail"),
            index_img: yaml_string(&front_matter, "index_img"),
            description: yaml_string(&front_matter, "description"),
            tags: yaml_strings(&front_matter, "tags"),
            categories: yaml_strings(&front_matter, "categories"),
            created_at: metadata.as_ref().and_then(|meta| meta.created().ok()).map(system_time_string),
            updated_at: metadata.as_ref().and_then(|meta| meta.modified().ok()).map(system_time_string),
            is_draft: path_string(path).contains(&format!("{}source{}{}", std::path::MAIN_SEPARATOR, std::path::MAIN_SEPARATOR, "_drafts")),
        });
    }

    posts.sort_by(|a, b| b.date.cmp(&a.date).then_with(|| b.updated_at.cmp(&a.updated_at)));
    Ok(posts)
}

#[tauri::command]
fn read_text_file(path: String) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| error.to_string())
}

#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    fs::write(path, content).map_err(|error| error.to_string())
}

#[tauri::command]
fn backup_text_file(project_path: String, file_path: String) -> Result<BackupResult, String> {
    let source = PathBuf::from(file_path);
    if !source.exists() {
        return Err("要备份的文件不存在".to_string());
    }
    let project = PathBuf::from(project_path);
    let dir = backup_dir(&project);
    fs::create_dir_all(&dir).map_err(|error| format!("创建备份目录失败: {error}"))?;
    let file_name = source.file_name().and_then(|value| value.to_str()).unwrap_or("file");
    let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
    let target = dir.join(format!("{file_name}.{timestamp}.bak"));
    fs::copy(&source, &target).map_err(|error| format!("备份文件失败: {error}"))?;
    Ok(BackupResult {
        backup_path: path_string(&target),
    })
}

#[tauri::command]
fn create_post(project_path: String, file_name: String, content: String) -> Result<String, String> {
    let safe_name = sanitize_file_name(&file_name);
    let final_name = if safe_name.ends_with(".md") || safe_name.ends_with(".markdown") {
        safe_name
    } else {
        format!("{safe_name}.md")
    };
    let path = PathBuf::from(project_path).join("source").join("_posts").join(final_name);
    if path.exists() {
        return Err("同名文章已存在".to_string());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, content).map_err(|error| error.to_string())?;
    Ok(path_string(&path))
}

#[tauri::command]
fn copy_image_to_project(project_path: String, image_path: String) -> Result<UploadResult, String> {
    let source = PathBuf::from(image_path);
    if !source.exists() {
        return Err("图片文件不存在".to_string());
    }

    let images_dir = PathBuf::from(project_path).join("source").join("images");
    fs::create_dir_all(&images_dir).map_err(|error| error.to_string())?;
    let original_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("image.png");
    let target_name = unique_file_name(&images_dir, original_name);
    let target = images_dir.join(&target_name);
    fs::copy(&source, &target).map_err(|error| error.to_string())?;

    let url = format!("/images/{target_name}");
    Ok(UploadResult {
        markdown: format!("![图片描述]({url})"),
        url,
        file_path: path_string(&target),
    })
}

#[tauri::command]
fn save_clipboard_image_to_project(
    project_path: String,
    file_name: Option<String>,
    mime_type: Option<String>,
    data: Vec<u8>,
) -> Result<UploadResult, String> {
    if data.is_empty() {
        return Err("剪贴板图片内容为空".to_string());
    }

    let images_dir = PathBuf::from(project_path).join("source").join("images");
    fs::create_dir_all(&images_dir).map_err(|error| error.to_string())?;
    let raw_name = file_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "clipboard-image-{}.{}",
                Local::now().format("%Y%m%d%H%M%S"),
                extension_for_mime(mime_type.as_deref())
            )
        });
    let safe_name = sanitize_file_name(&raw_name);
    let target_name = unique_file_name(&images_dir, &safe_name);
    let target = images_dir.join(&target_name);
    fs::write(&target, data).map_err(|error| error.to_string())?;

    let url = format!("/images/{target_name}");
    Ok(UploadResult {
        markdown: format!("![图片描述]({url})"),
        url,
        file_path: path_string(&target),
    })
}

#[tauri::command]
fn upload_image_path_to_cloudflare_imgbed(
    api_url: String,
    token: Option<String>,
    image_path: String,
) -> Result<UploadResult, String> {
    let source = PathBuf::from(image_path);
    if !source.exists() {
        return Err("Image file does not exist".to_string());
    }
    let file_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("image.png")
        .to_string();
    let mime_type = mime_for_path(&source).to_string();
    let data = fs::read(&source).map_err(|error| error.to_string())?;

    upload_cloudflare_imgbed_bytes(api_url, token, file_name, Some(mime_type), data)
}

#[tauri::command]
fn upload_clipboard_image_to_cloudflare_imgbed(
    api_url: String,
    token: Option<String>,
    file_name: Option<String>,
    mime_type: Option<String>,
    data: Vec<u8>,
) -> Result<UploadResult, String> {
    if data.is_empty() {
        return Err("Clipboard image data is empty".to_string());
    }
    let final_name = file_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "clipboard-image-{}.{}",
                Local::now().format("%Y%m%d%H%M%S"),
                extension_for_mime(mime_type.as_deref())
            )
        });

    upload_cloudflare_imgbed_bytes(api_url, token, final_name, mime_type, data)
}

fn upload_cloudflare_imgbed_bytes(
    api_url: String,
    token: Option<String>,
    file_name: String,
    mime_type: Option<String>,
    data: Vec<u8>,
) -> Result<UploadResult, String> {
    let endpoint = cloudflare_imgbed_upload_url(&api_url)?;
    let clean_name = sanitize_file_name(&file_name);
    let content_type = mime_type
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let part = reqwest::blocking::multipart::Part::bytes(data)
        .file_name(clean_name.clone())
        .mime_str(&content_type)
        .map_err(|error| error.to_string())?;
    let form = reqwest::blocking::multipart::Form::new().part("file", part);
    let client = reqwest::blocking::Client::builder()
        .user_agent("Hexo Lite Editor/1.0.1")
        .build()
        .map_err(|error| error.to_string())?;
    let mut request = client.post(endpoint.clone()).multipart(form);

    if let Some(token) = token.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()) {
        request = request
            .header("Authorization", format!("Bearer {token}"))
            .header("authCode", token);
    }

    let response = request.send().map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!("CloudFlare-ImgBed upload failed {status}: {body}"));
    }

    let json: JsonValue = serde_json::from_str(&body)
        .map_err(|error| format!("CloudFlare-ImgBed did not return JSON: {error}; {body}"))?;
    let raw_url = extract_cloudflare_imgbed_url(&json)
        .ok_or_else(|| format!("CloudFlare-ImgBed response did not contain src/url: {body}"))?;
    let url = absolutize_cloudflare_imgbed_url(&endpoint, &raw_url)?;

    Ok(UploadResult {
        markdown: format!("![image]({url})"),
        url: url.clone(),
        file_path: url,
    })
}

#[tauri::command]
fn list_cloudflare_imgbed_images(
    api_url: String,
    token: Option<String>,
    start: Option<u64>,
    count: Option<u64>,
    search: Option<String>,
    dir: Option<String>,
) -> Result<ImageBedListResult, String> {
    let mut url = cloudflare_imgbed_api_url(&api_url, "/api/manage/list")?;
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("start", &start.unwrap_or(0).to_string());
        query.append_pair("count", &count.unwrap_or(50).to_string());
        query.append_pair("recursive", "true");
        if let Some(search) = search.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
            query.append_pair("search", search);
        }
        if let Some(dir) = dir.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
            query.append_pair("dir", dir);
        }
    }

    let response = cloudflare_imgbed_request(reqwest::blocking::Client::new().get(url.clone()), token)
        .send()
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!("CloudFlare-ImgBed list failed {status}: {body}"));
    }

    let json: JsonValue = serde_json::from_str(&body)
        .map_err(|error| format!("CloudFlare-ImgBed list did not return JSON: {error}; {body}"))?;
    let origin = cloudflare_imgbed_origin(&api_url)?;
    let files = json
        .get("files")
        .and_then(JsonValue::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| normalize_imagebed_item(item, &origin))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let directories = json
        .get("directories")
        .and_then(JsonValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(ImageBedListResult {
        returned_count: json
            .get("returnedCount")
            .or_else(|| json.get("returned_count"))
            .and_then(JsonValue::as_u64)
            .unwrap_or(files.len() as u64),
        total_count: json
            .get("totalCount")
            .or_else(|| json.get("total_count"))
            .and_then(JsonValue::as_u64)
            .unwrap_or(files.len() as u64),
        files,
        directories,
    })
}

#[tauri::command]
fn delete_cloudflare_imgbed_image(
    api_url: String,
    token: Option<String>,
    file_id: String,
) -> Result<(), String> {
    let clean_id = file_id.trim();
    if clean_id.is_empty() {
        return Err("Image file id is empty".to_string());
    }

    let path = format!("/api/manage/delete/{}", encode_path_segments(clean_id));
    let url = cloudflare_imgbed_api_url(&api_url, &path)?;
    let response = cloudflare_imgbed_request(reqwest::blocking::Client::new().delete(url), token)
        .send()
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!("CloudFlare-ImgBed delete failed {status}: {body}"));
    }

    let json: JsonValue = serde_json::from_str(&body)
        .map_err(|error| format!("CloudFlare-ImgBed delete did not return JSON: {error}; {body}"))?;
    if json.get("success").and_then(JsonValue::as_bool).unwrap_or(false) {
        Ok(())
    } else {
        Err(format!("CloudFlare-ImgBed delete failed: {body}"))
    }
}

fn cloudflare_imgbed_upload_url(api_url: &str) -> Result<String, String> {
    let trimmed = api_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Please configure the CloudFlare-ImgBed API URL first".to_string());
    }
    let mut url = reqwest::Url::parse(trimmed).map_err(|error| format!("Invalid API URL: {error}"))?;
    let path = url.path().trim_end_matches('/');
    if !path.ends_with("/upload") && path != "/upload" {
        let next_path = if path.is_empty() || path == "/" {
            "/upload".to_string()
        } else {
            format!("{path}/upload")
        };
        url.set_path(&next_path);
    }
    {
        let mut query = url.query_pairs_mut();
        query.append_pair("returnFormat", "full");
        query.append_pair("uploadNameType", "origin");
    }
    Ok(url.to_string())
}

fn cloudflare_imgbed_api_url(api_url: &str, path: &str) -> Result<reqwest::Url, String> {
    let trimmed = api_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("Please configure the CloudFlare-ImgBed API URL first".to_string());
    }
    let mut url = reqwest::Url::parse(trimmed).map_err(|error| format!("Invalid API URL: {error}"))?;
    url.set_path(path);
    url.set_query(None);
    Ok(url)
}

fn cloudflare_imgbed_origin(api_url: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(api_url.trim()).map_err(|error| format!("Invalid API URL: {error}"))?;
    Ok(format!(
        "{}://{}",
        url.scheme(),
        url.host_str().ok_or_else(|| "Invalid API host".to_string())?
    ))
}

fn cloudflare_imgbed_request(
    request: reqwest::blocking::RequestBuilder,
    token: Option<String>,
) -> reqwest::blocking::RequestBuilder {
    if let Some(token) = token.map(|value| value.trim().to_string()).filter(|value| !value.is_empty()) {
        request.header("Authorization", format!("Bearer {token}"))
    } else {
        request
    }
}

fn normalize_imagebed_item(item: &JsonValue, origin: &str) -> ImageBedItem {
    let name = json_string(item, &["name", "id", "key", "fileId"]).unwrap_or_default();
    let metadata = item.get("metadata").unwrap_or(&JsonValue::Null);
    let file_name = json_string(item, &["fileName", "file_name", "FileName"])
        .or_else(|| json_string(metadata, &["FileName", "fileName", "file_name"]))
        .unwrap_or_else(|| name.rsplit('/').next().unwrap_or(&name).to_string());
    let file_type = json_string(item, &["fileType", "file_type", "FileType"])
        .or_else(|| json_string(metadata, &["FileType", "fileType", "file_type"]))
        .unwrap_or_default();
    let file_size = json_string(item, &["fileSize", "file_size", "FileSize"])
        .or_else(|| json_string(metadata, &["FileSize", "FileSizeBytes", "fileSize", "file_size"]))
        .unwrap_or_default();
    let created_at = json_string(item, &["createdAt", "created_at", "TimeStamp"])
        .or_else(|| json_string(metadata, &["TimeStamp", "createdAt", "created_at"]))
        .unwrap_or_default();
    let channel = json_string(item, &["channel", "Channel"])
        .or_else(|| json_string(metadata, &["Channel", "channel"]))
        .unwrap_or_default();
    let raw_url = json_string(item, &["publicUrl", "src", "url", "fileUrl", "fileURL"])
        .or_else(|| json_string(metadata, &["publicUrl", "src", "url", "fileUrl", "fileURL"]))
        .unwrap_or_else(|| format!("{}/file/{}", origin.trim_end_matches('/'), name));
    let url = if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        raw_url
    } else {
        format!(
            "{}/{}",
            origin.trim_end_matches('/'),
            raw_url.trim_start_matches('/')
        )
    };

    ImageBedItem {
        id: name.clone(),
        name,
        url,
        file_name,
        file_type,
        file_size,
        created_at,
        channel,
        raw: item.clone(),
    }
}

fn json_string(value: &JsonValue, keys: &[&str]) -> Option<String> {
    for key in keys {
        match value.get(*key) {
            Some(JsonValue::String(text)) if !text.trim().is_empty() => return Some(text.clone()),
            Some(JsonValue::Number(number)) => return Some(number.to_string()),
            Some(JsonValue::Bool(flag)) => return Some(flag.to_string()),
            _ => {}
        }
    }
    None
}

fn encode_path_segments(path: &str) -> String {
    path.split('/')
        .map(url_encode_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn url_encode_segment(segment: &str) -> String {
    segment
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

fn extract_cloudflare_imgbed_url(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(text) if text.starts_with("http") || text.starts_with("/file/") => Some(text.clone()),
        JsonValue::Array(items) => items.iter().find_map(extract_cloudflare_imgbed_url),
        JsonValue::Object(map) => {
            for key in ["publicUrl", "src", "url", "link", "fileUrl", "fileURL"] {
                if let Some(JsonValue::String(text)) = map.get(key) {
                    if !text.trim().is_empty() {
                        return Some(text.clone());
                    }
                }
            }
            for key in ["data", "result", "results"] {
                if let Some(found) = map.get(key).and_then(extract_cloudflare_imgbed_url) {
                    return Some(found);
                }
            }
            None
        }
        _ => None,
    }
}

fn absolutize_cloudflare_imgbed_url(endpoint: &str, raw_url: &str) -> Result<String, String> {
    if raw_url.starts_with("http://") || raw_url.starts_with("https://") {
        return Ok(raw_url.to_string());
    }
    let base = reqwest::Url::parse(endpoint).map_err(|error| error.to_string())?;
    base.join(raw_url)
        .map(|value| value.to_string())
        .map_err(|error| error.to_string())
}

fn mime_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|value| value.to_str()).unwrap_or("").to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

#[tauri::command]
fn run_hexo_generate(project_path: String) -> Result<CommandResult, String> {
    run_npx_hexo(&project_path, &["generate"])
}

#[tauri::command]
fn run_hexo_deploy(project_path: String) -> Result<CommandResult, String> {
    run_npx_hexo(&project_path, &["deploy"])
}

#[tauri::command]
fn run_hexo_generate_deploy(project_path: String) -> Result<CommandResult, String> {
    let clean = run_npx_hexo(&project_path, &["clean"])?;
    if !clean.success {
        return Ok(clean);
    }
    let generate = run_npx_hexo(&project_path, &["generate"])?;
    if !generate.success {
        return Ok(generate);
    }
    run_npx_hexo(&project_path, &["deploy"])
}

#[tauri::command]
fn run_hexo_server(project_path: String, state: tauri::State<HexoServerState>) -> Result<String, String> {
    let mut current = state.child.lock().map_err(|_| "Hexo Server 状态锁定失败".to_string())?;
    if current.is_some() {
        return Ok("Hexo Server 已在运行".to_string());
    }

    let child = Command::new(npx_command())
        .current_dir(project_path)
        .args(["hexo", "server"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("启动 Hexo Server 失败: {error}"))?;

    *current = Some(child);
    Ok("Hexo Server 已启动，默认地址 http://localhost:4000".to_string())
}

#[tauri::command]
fn stop_hexo_server(state: tauri::State<HexoServerState>) -> Result<String, String> {
    let mut current = state.child.lock().map_err(|_| "Hexo Server 状态锁定失败".to_string())?;
    if let Some(mut child) = current.take() {
        child.kill().map_err(|error| format!("停止 Hexo Server 失败: {error}"))?;
        return Ok("Hexo Server 已停止".to_string());
    }
    Ok("Hexo Server 未运行".to_string())
}

#[tauri::command]
fn git_status(project_path: String) -> Result<CommandResult, String> {
    run_git(&project_path, &["status", "--short"])
}

#[tauri::command]
fn run_terminal_command(project_path: String, command: String) -> Result<CommandResult, String> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return Err("命令不能为空".to_string());
    }
    let project = PathBuf::from(&project_path);
    if !project.exists() {
        return Err("请先打开有效的 Hexo 项目".to_string());
    }

    let mut process = if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", trimmed]);
        command
    } else {
        let mut command = Command::new("sh");
        command.args(["-lc", trimmed]);
        command
    };

    let output = process
        .current_dir(project)
        .output()
        .map_err(|error| format!("命令执行失败: {error}"))?;

    Ok(CommandResult {
        success: output.status.success(),
        command: trimmed.to_string(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        code: output.status.code(),
    })
}

#[tauri::command]
fn load_app_config(app: AppHandle) -> Result<serde_json::Value, String> {
    let path = config_path(&app)?;
    if !path.exists() {
        return Ok(default_app_config());
    }
    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_app_config(app: AppHandle, config: serde_json::Value) -> Result<(), String> {
    let path = config_path(&app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let content = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

#[tauri::command]
fn reset_app_config(app: AppHandle) -> Result<serde_json::Value, String> {
    let config = default_app_config();
    save_app_config(app, config.clone())?;
    Ok(config)
}

#[tauri::command]
fn open_config_dir(app: AppHandle) -> Result<(), String> {
    let path = config_path(&app)?;
    if let Some(dir) = path.parent() {
        open_path_external(dir)?;
        return Ok(());
    }
    Err("无法定位配置目录".to_string())
}

#[tauri::command]
fn read_hexo_config(project_path: String) -> Result<HexoConfigFile, String> {
    let project = PathBuf::from(&project_path);
    let config = project.join("_config.yml");
    let content = if config.exists() {
        fs::read_to_string(&config).map_err(|error| format!("读取 _config.yml 失败: {error}"))?
    } else {
        String::new()
    };

    Ok(HexoConfigFile {
        exists: config.exists(),
        project_path: path_string(&project),
        config_path: path_string(&config),
        content,
        latest_backup_path: latest_backup(&project).map(|path| path_string(&path)),
    })
}

#[tauri::command]
fn save_hexo_config(project_path: String, content: String) -> Result<BackupResult, String> {
    if content.trim().is_empty() {
        return Err("_config.yml 内容不能为空".to_string());
    }
    let project = PathBuf::from(project_path);
    let config = project.join("_config.yml");
    if !config.exists() {
        return Err("_config.yml 不存在".to_string());
    }
    let backup_path = backup_hexo_config_file(&project)?;
    fs::write(&config, content).map_err(|error| format!("保存 _config.yml 失败: {error}"))?;
    prune_backups(&project, 10)?;
    Ok(BackupResult {
        backup_path: path_string(&backup_path),
    })
}

#[tauri::command]
fn backup_hexo_config(project_path: String) -> Result<BackupResult, String> {
    let project = PathBuf::from(project_path);
    let backup_path = backup_hexo_config_file(&project)?;
    prune_backups(&project, 10)?;
    Ok(BackupResult {
        backup_path: path_string(&backup_path),
    })
}

#[tauri::command]
fn restore_latest_hexo_config_backup(project_path: String) -> Result<HexoConfigFile, String> {
    let project = PathBuf::from(project_path);
    let config = project.join("_config.yml");
    let latest = latest_backup(&project).ok_or_else(|| "没有可恢复的 _config.yml 备份".to_string())?;
    fs::copy(&latest, &config).map_err(|error| format!("恢复备份失败: {error}"))?;
    read_hexo_config(path_string(&project))
}

#[tauri::command]
fn open_hexo_config_external(project_path: String) -> Result<(), String> {
    let config = PathBuf::from(project_path).join("_config.yml");
    if !config.exists() {
        return Err("_config.yml 不存在".to_string());
    }
    open_path_external(&config)
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
fn check_update(settings: UpdateSettings) -> Result<UpdateCheckResult, String> {
    let current = get_app_version();
    let client = reqwest::blocking::Client::builder()
        .user_agent("Hexo Lite Editor")
        .build()
        .map_err(|error| error.to_string())?;

    if settings.update_source == "custom" {
        let url = settings
            .custom_update_url
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| "请先填写自定义更新地址".to_string())?;
        let data: serde_json::Value = client
            .get(&url)
            .send()
            .map_err(|error| format!("请求更新地址失败: {error}"))?
            .error_for_status()
            .map_err(|error| format!("更新地址返回错误: {error}"))?
            .json()
            .map_err(|error| format!("解析更新信息失败: {error}"))?;
        let latest = data
            .get("latestVersion")
            .or_else(|| data.get("version"))
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        return Ok(UpdateCheckResult {
            current_version: current.clone(),
            latest_version: latest.clone(),
            has_update: normalize_version(&latest) != normalize_version(&current),
            release_notes: data
                .get("releaseNotes")
                .or_else(|| data.get("notes"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string),
            download_url: data.get("downloadUrl").and_then(|value| value.as_str()).map(ToString::to_string),
            release_page_url: data
                .get("releasePageUrl")
                .or_else(|| data.get("url"))
                .and_then(|value| value.as_str())
                .map(ToString::to_string),
        });
    }

    let owner = settings
        .github_owner
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "请先填写 GitHub Owner".to_string())?;
    let repo = settings
        .github_repo
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "请先填写 GitHub Repo".to_string())?;
    let url = format!("https://api.github.com/repos/{owner}/{repo}/releases/latest");
    let release: serde_json::Value = client
        .get(url)
        .send()
        .map_err(|error| format!("请求 GitHub Releases 失败: {error}"))?
        .error_for_status()
        .map_err(|error| format!("GitHub Releases 返回错误: {error}"))?
        .json()
        .map_err(|error| format!("解析 GitHub Releases 失败: {error}"))?;

    let latest = release
        .get("tag_name")
        .or_else(|| release.get("name"))
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let download_url = release
        .get("assets")
        .and_then(|value| value.as_array())
        .and_then(|assets| assets.first())
        .and_then(|asset| asset.get("browser_download_url"))
        .and_then(|value| value.as_str())
        .map(ToString::to_string);

    Ok(UpdateCheckResult {
        current_version: current.clone(),
        latest_version: latest.clone(),
        has_update: normalize_version(&latest) != normalize_version(&current),
        release_notes: release.get("body").and_then(|value| value.as_str()).map(ToString::to_string),
        download_url,
        release_page_url: release.get("html_url").and_then(|value| value.as_str()).map(ToString::to_string),
    })
}

#[tauri::command]
fn open_release_page(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("只允许打开 http/https 发布页面".to_string());
    }
    open_url_external(&url)
}

fn run_npx_hexo(project_path: &str, hexo_args: &[&str]) -> Result<CommandResult, String> {
    let mut args = vec!["hexo"];
    args.extend(hexo_args);
    run_command(project_path, npx_command(), &args)
}

fn run_git(project_path: &str, args: &[&str]) -> Result<CommandResult, String> {
    run_command(project_path, git_command(), args)
}

fn run_command(project_path: &str, command: &str, args: &[&str]) -> Result<CommandResult, String> {
    let output = Command::new(command)
        .current_dir(project_path)
        .args(args)
        .output()
        .map_err(|error| format!("命令执行失败: {error}"))?;
    let command_text = format!("{command} {}", args.join(" "));

    Ok(CommandResult {
        success: output.status.success(),
        command: command_text,
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        code: output.status.code(),
    })
}

fn parse_front_matter(content: &str) -> HashMap<String, Value> {
    if !content.starts_with("---") {
        return HashMap::new();
    }
    let mut lines = content.lines();
    lines.next();
    let mut yaml = String::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    serde_yaml::from_str(&yaml).unwrap_or_default()
}

fn yaml_string(map: &HashMap<String, Value>, key: &str) -> Option<String> {
    map.get(key).and_then(|value| match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    })
}

fn yaml_strings(map: &HashMap<String, Value>, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(Value::Sequence(values)) => values
            .iter()
            .filter_map(|value| match value {
                Value::String(text) => Some(text.clone()),
                Value::Number(number) => Some(number.to_string()),
                _ => None,
            })
            .collect(),
        Some(Value::String(text)) => vec![text.clone()],
        _ => Vec::new(),
    }
}

fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown"))
        .unwrap_or(false)
}

fn sanitize_file_name(input: &str) -> String {
    let trimmed = input.trim();
    let mut value = String::new();
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            value.push(ch);
        } else if ch.is_whitespace() {
            value.push('-');
        }
    }
    if value.is_empty() {
        "untitled".to_string()
    } else {
        value
    }
}

fn unique_file_name(dir: &Path, file_name: &str) -> String {
    let path = PathBuf::from(file_name);
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("image");
    let ext = path.extension().and_then(|value| value.to_str()).unwrap_or("png");
    let mut candidate = format!("{stem}.{ext}");
    let mut index = 1;

    while dir.join(&candidate).exists() {
        candidate = format!("{stem}-{index}.{ext}");
        index += 1;
    }
    candidate
}

fn extension_for_mime(mime_type: Option<&str>) -> &'static str {
    match mime_type.unwrap_or_default() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/bmp" => "bmp",
        _ => "png",
    }
}

fn system_time_string(time: std::time::SystemTime) -> String {
    match time.duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => String::new(),
    }
}

fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("读取配置目录失败: {error}"))?;
    Ok(dir.join("app-config.json"))
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn backup_dir(project: &Path) -> PathBuf {
    project.join(".hexo-lite-editor").join("backups")
}

fn backup_hexo_config_file(project: &Path) -> Result<PathBuf, String> {
    let config = project.join("_config.yml");
    if !config.exists() {
        return Err("_config.yml 不存在，无法备份".to_string());
    }

    let dir = backup_dir(project);
    fs::create_dir_all(&dir).map_err(|error| format!("创建备份目录失败: {error}"))?;
    let timestamp = Local::now().format("%Y-%m-%d-%H-%M-%S");
    let target = dir.join(format!("_config.yml.{timestamp}.bak"));
    fs::copy(&config, &target).map_err(|error| format!("备份 _config.yml 失败: {error}"))?;
    Ok(target)
}

fn latest_backup(project: &Path) -> Option<PathBuf> {
    let dir = backup_dir(project);
    let mut backups = backup_entries(&dir).ok()?;
    backups.sort_by_key(|(_, modified)| *modified);
    backups.pop().map(|(path, _)| path)
}

fn prune_backups(project: &Path, max_count: usize) -> Result<(), String> {
    let dir = backup_dir(project);
    let mut backups = backup_entries(&dir)?;
    backups.sort_by_key(|(_, modified)| *modified);
    while backups.len() > max_count {
        if let Some((path, _)) = backups.first() {
            fs::remove_file(path).map_err(|error| format!("清理旧备份失败: {error}"))?;
        }
        backups.remove(0);
    }
    Ok(())
}

fn backup_entries(dir: &Path) -> Result<Vec<(PathBuf, SystemTime)>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let name = path.file_name().and_then(|value| value.to_str()).unwrap_or_default();
        if !name.starts_with("_config.yml.") || !name.ends_with(".bak") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH);
        entries.push((path, modified));
    }
    Ok(entries)
}

fn open_path_external(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &path_string(path)])
            .spawn()
            .map_err(|error| format!("打开失败: {error}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|error| format!("打开失败: {error}"))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|error| format!("打开失败: {error}"))?;
    }
    Ok(())
}

fn open_url_external(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|error| format!("打开发布页面失败: {error}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|error| format!("打开发布页面失败: {error}"))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|error| format!("打开发布页面失败: {error}"))?;
    }
    Ok(())
}

fn default_app_config() -> serde_json::Value {
    json!({
        "general": {
            "openRecentProjectOnStart": true,
            "autoSave": true,
            "autoSaveInterval": 3000,
            "backupBeforeSave": false,
            "defaultPage": "editor",
            "maxLogCount": 500
        },
        "appearance": {
            "themeMode": "system",
            "colorScheme": "vscode-light",
            "compactMode": false,
            "showPostCover": true,
            "fontScale": 1
        },
        "editor": {
            "fontSize": 16,
            "lineHeight": 1.6,
            "showLineNumbers": true,
            "lineWrapping": true,
            "highlightActiveLine": true,
            "markdownHighlight": true,
            "tabSize": 2,
            "defaultEditorMode": "split"
        },
        "layout": {
            "sidebarWidth": 300,
            "previewWidth": 0,
            "logPanelHeight": 240,
            "showPreview": true,
            "showLogPanel": false
        },
        "ribbon": {
            "activeTab": "write"
        },
        "postList": {
            "showCover": true,
            "coverSourcePriority": ["cover"]
        },
        "uploader": {
            "defaultType": "local",
            "apiUrl": "",
            "token": "",
            "method": "POST",
            "fileField": "file",
            "urlField": "data.url",
            "autoInsertMarkdown": true
        },
        "publish": {
            "hexoServerCommand": "npx hexo server",
            "hexoCleanCommand": "npx hexo clean",
            "hexoGenerateCommand": "npx hexo generate",
            "hexoDeployCommand": "npx hexo deploy",
            "saveBeforePublish": true,
            "cleanBeforeGenerate": false,
            "generateBeforeDeploy": true,
            "gitPushAfterDeploy": false
        },
        "update": {
            "checkUpdateOnStart": false,
            "updateSource": "github",
            "githubOwner": "",
            "githubRepo": "",
            "customUpdateUrl": ""
        },
        "recentProjects": []
    })
}

fn normalize_version(version: &str) -> String {
    version.trim().trim_start_matches('v').trim_start_matches('V').to_string()
}

fn npx_command() -> &'static str {
    if cfg!(windows) {
        "npx.cmd"
    } else {
        "npx"
    }
}

fn git_command() -> &'static str {
    if cfg!(windows) {
        "git.exe"
    } else {
        "git"
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(HexoServerState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            validate_hexo_project,
            scan_posts,
            read_text_file,
            write_text_file,
            backup_text_file,
            create_post,
            copy_image_to_project,
            save_clipboard_image_to_project,
            upload_image_path_to_cloudflare_imgbed,
            upload_clipboard_image_to_cloudflare_imgbed,
            list_cloudflare_imgbed_images,
            delete_cloudflare_imgbed_image,
            run_hexo_server,
            stop_hexo_server,
            run_hexo_generate,
            run_hexo_deploy,
            run_hexo_generate_deploy,
            git_status,
            run_terminal_command,
            load_app_config,
            save_app_config,
            reset_app_config,
            open_config_dir,
            read_hexo_config,
            save_hexo_config,
            backup_hexo_config,
            restore_latest_hexo_config_backup,
            open_hexo_config_external,
            get_app_version,
            check_update,
            open_release_page
        ])
        .setup(|app| {
            if let Ok(dir) = app.path().app_config_dir() {
                let path = dir.join("app-config.json");
                if !path.exists() {
                    let _ = fs::create_dir_all(&dir);
                    if let Ok(content) = serde_json::to_string_pretty(&default_app_config()) {
                        let _ = fs::write(path, content);
                    }
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
