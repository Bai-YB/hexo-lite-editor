use crate::{
    app::AppState,
    data::{load_config, save_config_file},
    domain::{AppConfigV3, AppError, AppResult, ConfigLoadResult, CredentialStatus},
    platform::{
        cloudflare_status, delete_cloudflare_token, legacy_cloudflare_token_available,
        migrate_legacy_cloudflare_token, set_cloudflare_token,
    },
};
use tauri::State;

#[tauri::command]
pub fn load_app_config(state: State<'_, AppState>) -> AppResult<ConfigLoadResult> {
    let _guard = state
        .config_write_lock
        .lock()
        .map_err(|_| AppError::new("config_lock_poisoned", "配置存储不可用。", false))?;
    load_config(&state)
}

#[tauri::command]
pub fn save_app_config(config: AppConfigV3, state: State<'_, AppState>) -> AppResult<AppConfigV3> {
    let _guard = state
        .config_write_lock
        .lock()
        .map_err(|_| AppError::new("config_lock_poisoned", "配置存储不可用。", false))?;
    save_config_file(&state.config_path, &config)?;
    Ok(config)
}

#[tauri::command]
pub fn reset_app_config(state: State<'_, AppState>) -> AppResult<AppConfigV3> {
    let _guard = state
        .config_write_lock
        .lock()
        .map_err(|_| AppError::new("config_lock_poisoned", "配置存储不可用。", false))?;
    let config = AppConfigV3::default();
    save_config_file(&state.config_path, &config)?;
    Ok(config)
}

#[tauri::command]
pub fn credential_status(connection_id: String, base_url: String) -> CredentialStatus {
    cloudflare_status(&connection_id, &base_url)
}

#[tauri::command]
pub fn credential_set(
    connection_id: String,
    base_url: String,
    token: String,
) -> AppResult<CredentialStatus> {
    set_cloudflare_token(&connection_id, &base_url, &token)
}

#[tauri::command]
pub fn credential_delete(connection_id: String) -> AppResult<CredentialStatus> {
    delete_cloudflare_token(&connection_id)
}

#[tauri::command]
pub fn credential_legacy_available() -> bool {
    legacy_cloudflare_token_available()
}

#[tauri::command]
pub fn credential_migrate(connection_id: String, base_url: String) -> AppResult<CredentialStatus> {
    migrate_legacy_cloudflare_token(&connection_id, &base_url)
}
