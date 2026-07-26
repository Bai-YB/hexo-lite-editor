use crate::domain::{AppError, AppResult};

pub(super) fn normalize_webdav_endpoint(raw: &str) -> AppResult<String> {
    let mut url =
        url::Url::parse(raw.trim()).map_err(|_| AppError::invalid("WebDAV 服务器地址无效。"))?;
    let local_debug = local_http_webdav_allowed(url.scheme(), url.host_str());
    if (url.scheme() != "https" && !local_debug)
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(AppError::invalid(
            "WebDAV 服务器必须使用不含凭据、查询参数或片段的 HTTPS 地址。",
        ));
    }
    url.set_query(None);
    url.set_fragment(None);
    let path = url.path().trim_end_matches('/').to_string();
    url.set_path(if path.is_empty() { "/" } else { &path });
    Ok(url.to_string().trim_end_matches('/').to_string())
}

pub(super) fn local_http_webdav_allowed(scheme: &str, host: Option<&str>) -> bool {
    if !cfg!(debug_assertions) || scheme != "http" {
        return false;
    }
    if matches!(host, Some("localhost" | "127.0.0.1" | "[::1]")) {
        return true;
    }
    cfg!(test)
        && std::env::var_os("HLEX_REAL_WEBDAV_TEST_ALLOW_HTTP").is_some_and(|value| value == "1")
}

pub(super) fn validate_webdav_remote_dir(raw: &str) -> AppResult<String> {
    let value = raw.trim().trim_matches('/').replace('\\', "/");
    if value.is_empty()
        || value.len() > 512
        || value.split('/').any(|segment| {
            segment.is_empty()
                || matches!(segment, "." | "..")
                || segment.chars().any(char::is_control)
        })
    {
        return Err(AppError::invalid(
            "WebDAV 远端目录必须是不含空段或路径穿越的相对路径。",
        ));
    }
    Ok(value)
}

pub(super) fn validate_branch(value: &str) -> AppResult<String> {
    let branch = value.trim();
    if branch.is_empty()
        || branch.len() > 120
        || branch.starts_with(['-', '.', '/'])
        || branch.ends_with(['.', '/'])
        || branch.ends_with(".lock")
        || branch.contains("..")
        || branch.contains("@{")
        || branch.contains("//")
        || branch.chars().any(|character| {
            character.is_control()
                || character.is_whitespace()
                || matches!(character, '~' | '^' | ':' | '?' | '*' | '[' | '\\')
        })
    {
        return Err(AppError::invalid("内容分支名称无效。"));
    }
    Ok(branch.to_string())
}
