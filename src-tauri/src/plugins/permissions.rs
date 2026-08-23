use crate::domain::{AppError, AppResult};

pub fn validate_permission(permission: &str) -> AppResult<()> {
    if matches!(
        permission,
        "article:read" | "article:write" | "image:read-selected"
    ) {
        return Ok(());
    }
    if let Some(origin) = permission.strip_prefix("network:") {
        let url = url::Url::parse(origin).map_err(|_| AppError::invalid("插件网络权限无效。"))?;
        if url.scheme() == "https"
            && url.path() == "/"
            && url.username().is_empty()
            && url.password().is_none()
        {
            return Ok(());
        }
    }
    Err(AppError::new(
        "plugin_permission_invalid",
        format!("不支持的插件权限：{permission}"),
        false,
    ))
}
