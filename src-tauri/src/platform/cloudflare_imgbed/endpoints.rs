use crate::domain::{AppError, AppResult};
use url::Url;

pub fn management_endpoint(base: &Url, operation: &str, asset_path: &str) -> AppResult<Url> {
    let segments = safe_segments(asset_path)?;
    let mut endpoint = base.clone();
    endpoint.set_path("/");
    endpoint.set_query(None);
    endpoint = endpoint
        .join(&format!("api/manage/{operation}"))
        .map_err(|_| AppError::invalid("无法生成 CloudFlare-ImgBed 管理地址。"))?;
    endpoint
        .path_segments_mut()
        .map_err(|_| AppError::invalid("无法生成 CloudFlare-ImgBed 管理地址。"))?
        .pop_if_empty()
        .extend(segments);
    Ok(endpoint)
}

pub fn safe_segments(path: &str) -> AppResult<Vec<&str>> {
    let segments = path
        .split('/')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if segments.is_empty()
        || segments
            .iter()
            .any(|value| matches!(*value, "." | "..") || value.contains('\\'))
    {
        return Err(AppError::invalid("远程资源路径无效。"));
    }
    Ok(segments)
}
