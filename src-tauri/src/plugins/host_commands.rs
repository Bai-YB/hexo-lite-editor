use super::{
    errors, package,
    registry::{PluginRegistry, PluginView},
};
use crate::{
    app::AppState,
    domain::{AppError, AppResult},
};
use std::{fs, path::PathBuf};
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use url::Url;

const PLUGIN_HTTP_MAX_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHttpRequest {
    pub plugin_id: String,
    pub url: String,
    #[serde(default = "default_http_method")]
    pub method: String,
    #[serde(default)]
    pub headers: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<Vec<u8>>,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginHttpResponse {
    pub status: u16,
    pub headers: std::collections::BTreeMap<String, String>,
    pub body: Vec<u8>,
}

fn default_http_method() -> String {
    "GET".into()
}

fn reload(state: &AppState) -> AppResult<()> {
    let next = PluginRegistry::load(&state.plugins_dir)?;
    *state
        .plugin_registry
        .write()
        .map_err(|_| AppError::new("state_poisoned", "插件状态不可用。", false))? = next;
    Ok(())
}

#[tauri::command]
pub fn list_plugins(state: State<'_, AppState>) -> AppResult<Vec<PluginView>> {
    Ok(state
        .plugin_registry
        .read()
        .map_err(|_| AppError::new("state_poisoned", "插件状态不可用。", false))?
        .views())
}
#[tauri::command]
pub fn install_plugin(
    source_directory: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<PluginView>> {
    package::install_directory(&PathBuf::from(source_directory), &state.plugins_dir)?;
    reload(&state)?;
    list_plugins(state)
}

#[tauri::command]
pub fn choose_and_install_plugin(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Vec<PluginView>> {
    let Some(directory) = app
        .dialog()
        .file()
        .set_title("选择插件目录")
        .blocking_pick_folder()
    else {
        return list_plugins(state);
    };
    let path = directory
        .into_path()
        .map_err(|error| AppError::invalid(error.to_string()))?;
    package::install_directory(&path, &state.plugins_dir)?;
    reload(&state)?;
    list_plugins(state)
}
#[tauri::command]
pub fn uninstall_plugin(
    plugin_id: String,
    state: State<'_, AppState>,
) -> AppResult<Vec<PluginView>> {
    let path = state.plugins_dir.join(&plugin_id);
    if !path.exists() {
        return Err(errors::not_found());
    }
    fs::remove_dir_all(path).map_err(|error| AppError::io("卸载插件失败", error))?;
    reload(&state)?;
    list_plugins(state)
}
#[tauri::command]
pub fn enable_plugin(plugin_id: String, state: State<'_, AppState>) -> AppResult<Vec<PluginView>> {
    toggle(&plugin_id, true, &state)?;
    list_plugins(state)
}
#[tauri::command]
pub fn disable_plugin(plugin_id: String, state: State<'_, AppState>) -> AppResult<Vec<PluginView>> {
    toggle(&plugin_id, false, &state)?;
    list_plugins(state)
}
fn toggle(plugin_id: &str, enabled: bool, state: &AppState) -> AppResult<()> {
    let path = state.plugins_dir.join(plugin_id).join(".enabled");
    if !path.parent().is_some_and(|parent| parent.exists()) {
        return Err(errors::not_found());
    }
    if enabled {
        fs::write(&path, b"0.1").map_err(|error| AppError::io("启用插件失败", error))?;
    } else if path.exists() {
        fs::remove_file(path).map_err(|error| AppError::io("禁用插件失败", error))?;
    }
    reload(state)
}
#[tauri::command]
pub fn get_plugin_settings(
    plugin_id: String,
    state: State<'_, AppState>,
) -> AppResult<serde_json::Value> {
    let path = state.plugins_dir.join(plugin_id).join("settings.json");
    if !path.exists() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_str(
        &fs::read_to_string(path).map_err(|error| AppError::io("读取插件设置失败", error))?,
    )
    .map_err(|error| AppError::new("plugin_settings_invalid", error.to_string(), true))
}

#[tauri::command]
pub fn get_plugin_settings_schema(
    plugin_id: String,
    state: State<'_, AppState>,
) -> AppResult<Option<serde_json::Value>> {
    let registry = state
        .plugin_registry
        .read()
        .map_err(|_| AppError::new("state_poisoned", "插件状态不可用。", false))?;
    let (manifest, _, root) = registry
        .plugins
        .get(&plugin_id)
        .ok_or_else(errors::not_found)?;
    let Some(relative) = manifest
        .contributes
        .get("settings")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };
    let relative_path = std::path::Path::new(relative);
    if relative_path.is_absolute()
        || relative.contains("..")
        || relative.contains('\\')
        || relative_path.extension().and_then(|value| value.to_str()) != Some("json")
    {
        return Err(AppError::invalid("插件设置 schema 路径无效。"));
    }
    let content = fs::read_to_string(root.join(relative_path))
        .map_err(|error| AppError::io("读取插件设置 schema 失败", error))?;
    serde_json::from_str(&content)
        .map(Some)
        .map_err(|error| AppError::new("plugin_settings_schema_invalid", error.to_string(), true))
}
#[tauri::command]
pub fn save_plugin_settings(
    plugin_id: String,
    settings: serde_json::Value,
    state: State<'_, AppState>,
) -> AppResult<()> {
    if !settings.is_object() {
        return Err(AppError::invalid("插件设置必须是对象。"));
    }
    let path = state.plugins_dir.join(plugin_id).join("settings.json");
    if !path.parent().is_some_and(|parent| parent.exists()) {
        return Err(errors::not_found());
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&settings).unwrap_or_default(),
    )
    .map_err(|error| AppError::io("保存插件设置失败", error))
}

#[tauri::command]
pub async fn plugin_http_request(
    request: PluginHttpRequest,
    state: State<'_, AppState>,
) -> AppResult<PluginHttpResponse> {
    let url = Url::parse(&request.url).map_err(|_| AppError::invalid("插件请求地址无效。"))?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
        return Err(AppError::new(
            "plugin_network_invalid_url",
            "插件只能请求 HTTPS 地址。",
            true,
        ));
    }
    let origin = url.origin().ascii_serialization();
    let permission = format!("network:{origin}");
    let allowed = state
        .plugin_registry
        .read()
        .map_err(|_| AppError::new("state_poisoned", "插件状态不可用。", false))?
        .plugins
        .get(&request.plugin_id)
        .is_some_and(|(manifest, enabled, _)| {
            *enabled && manifest.permissions.contains(&permission)
        });
    if !allowed {
        return Err(errors::permission_denied());
    }
    let method = reqwest::Method::from_bytes(request.method.as_bytes())
        .map_err(|_| AppError::invalid("插件 HTTP 方法无效。"))?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|error| AppError::new("plugin_network_failed", error.to_string(), true))?;
    let mut builder = client.request(method, url);
    for (name, value) in request.headers {
        let lower = name.to_ascii_lowercase();
        if matches!(lower.as_str(), "authorization" | "cookie" | "host") {
            return Err(AppError::new(
                "plugin_header_denied",
                "插件不能设置敏感请求头。",
                true,
            ));
        }
        builder = builder.header(name, value);
    }
    if let Some(body) = request.body {
        builder = builder.body(body);
    }
    let response = builder
        .send()
        .await
        .map_err(|error| AppError::new("plugin_network_failed", error.to_string(), true))?;
    if response.status().is_redirection() {
        return Err(AppError::new(
            "plugin_redirect_denied",
            "插件网络请求不自动跟随重定向。",
            true,
        ));
    }
    if response
        .content_length()
        .is_some_and(|size| size > PLUGIN_HTTP_MAX_BYTES as u64)
    {
        return Err(AppError::new(
            "plugin_response_too_large",
            "插件网络响应过大。",
            true,
        ));
    }
    let status = response.status().as_u16();
    let headers = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.to_string(), value.to_string()))
        })
        .collect();
    let body = response
        .bytes()
        .await
        .map_err(|error| AppError::new("plugin_network_failed", error.to_string(), true))?;
    if body.len() > PLUGIN_HTTP_MAX_BYTES {
        return Err(AppError::new(
            "plugin_response_too_large",
            "插件网络响应过大。",
            true,
        ));
    }
    Ok(PluginHttpResponse {
        status,
        headers,
        body: body.to_vec(),
    })
}
