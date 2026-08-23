use crate::domain::AppError;
pub fn not_found() -> AppError {
    AppError::new("plugin_not_found", "插件不存在。", true)
}

pub fn permission_denied() -> AppError {
    AppError::new(
        "plugin_permission_denied",
        "插件没有此操作所需的权限。",
        true,
    )
}
