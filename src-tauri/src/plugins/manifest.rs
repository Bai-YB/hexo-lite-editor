use crate::domain::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: String,
    pub entry: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    #[serde(default)]
    pub contributes: serde_json::Value,
}

impl PluginManifest {
    pub fn validate(&self) -> AppResult<()> {
        if self.api_version != "0.1" {
            return Err(AppError::new(
                "plugin_api_incompatible",
                "插件 API 版本不兼容。",
                true,
            ));
        }
        if self.id.is_empty()
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        {
            return Err(AppError::invalid("插件 id 无效。"));
        }
        let entry = Path::new(&self.entry);
        if entry.is_absolute()
            || self.entry.contains("..")
            || entry.extension().and_then(|v| v.to_str()) != Some("js")
        {
            return Err(AppError::invalid("插件入口必须是包内 JavaScript 文件。"));
        }
        for permission in &self.permissions {
            super::permissions::validate_permission(permission)?;
        }
        Ok(())
    }
}
