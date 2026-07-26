use crate::{
    app::AppState,
    domain::{AppConfigV3, AppError, AppResult, ConfigLoadResult, ImageProvider, ThemeMode},
    platform::{atomic_write, set_cloudflare_token},
};
use chrono::Local;
use serde_json::Value;
use std::{fs, path::Path};

pub fn load_config(state: &AppState) -> AppResult<ConfigLoadResult> {
    if state.config_path.exists() {
        let content = fs::read_to_string(&state.config_path)
            .map_err(|error| AppError::io("读取应用配置失败", error))?;
        match serde_json::from_str::<AppConfigV3>(&content) {
            Ok(config) => {
                config.validate()?;
                return Ok(ConfigLoadResult {
                    config,
                    warnings: Vec::new(),
                });
            }
            Err(error) => {
                let stamp = Local::now().format("%Y%m%d-%H%M%S");
                let corrupt = state
                    .config_path
                    .with_file_name(format!("config-v3.corrupt-{stamp}.json"));
                fs::rename(&state.config_path, &corrupt)
                    .map_err(|move_error| AppError::io("保留损坏配置失败", move_error))?;
                let config = AppConfigV3::default();
                save_config_file(&state.config_path, &config)?;
                return Ok(ConfigLoadResult {
                    config,
                    warnings: vec![format!(
                        "配置解析失败（{error}），损坏文件已保留为 {}，当前使用安全默认值。",
                        corrupt.display()
                    )],
                });
            }
        }
    }

    if state.v2_config_path.exists() {
        let content = fs::read_to_string(&state.v2_config_path)
            .map_err(|error| AppError::io("读取 1.0.2 配置失败", error))?;
        let mut config: AppConfigV3 = serde_json::from_str(&content)
            .map_err(|error| AppError::new("config_v2_corrupt", error.to_string(), true))?;
        config.schema_version = 3;
        config.validate()?;
        let backup = state
            .v2_config_path
            .with_file_name("config-v2.json.1.0.2.bak");
        if !backup.exists() {
            atomic_write(&backup, content.as_bytes())?;
        }
        save_config_file(&state.config_path, &config)?;
        return Ok(ConfigLoadResult {
            config,
            warnings: vec![
                "已迁移 1.0.2 配置；原配置已备份，预览与日志使用当前安全默认值。".to_string(),
            ],
        });
    }

    if state.legacy_config_path.exists() {
        let content = fs::read_to_string(&state.legacy_config_path)
            .map_err(|error| AppError::io("读取 1.0.1 配置失败", error))?;
        let value: Value = serde_json::from_str(&content)
            .map_err(|error| AppError::new("config_corrupt", error.to_string(), true))?;
        let config = migrate_v1(&value)?;
        let legacy_token = value
            .pointer("/uploader/token")
            .and_then(Value::as_str)
            .filter(|token| !token.trim().is_empty())
            .map(str::to_string);
        if let Some(token) = legacy_token.as_deref() {
            set_cloudflare_token(
                &config.image_bed.cloudflare_connection_id,
                &config.image_bed.cloudflare_api_url,
                token,
            )?;
        }
        let scrubbed = scrub_legacy_secrets(value);
        let scrubbed_bytes = serde_json::to_vec_pretty(&scrubbed)
            .map_err(|error| AppError::new("config_serialize", error.to_string(), false))?;
        let legacy_name = state
            .legacy_config_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("app-config.json");
        let backup_path = state
            .legacy_config_path
            .with_file_name(format!("{legacy_name}.1.0.1.bak"));
        atomic_write(&backup_path, &scrubbed_bytes)?;
        atomic_write(&state.legacy_config_path, &scrubbed_bytes)?;
        save_config_file(&state.config_path, &config)?;
        return Ok(ConfigLoadResult {
            config,
            warnings: vec![
                "已迁移 1.0.1 配置；旧配置已备份，明文凭据已移入系统凭据库。".to_string(),
            ],
        });
    }

    let config = AppConfigV3::default();
    save_config_file(&state.config_path, &config)?;
    Ok(ConfigLoadResult {
        config,
        warnings: Vec::new(),
    })
}

fn scrub_legacy_secrets(mut value: Value) -> Value {
    if let Some(uploader) = value.get_mut("uploader").and_then(Value::as_object_mut) {
        uploader.remove("token");
    }
    value
}

pub fn save_config_file(path: &Path, config: &AppConfigV3) -> AppResult<()> {
    config.validate()?;
    let bytes = serde_json::to_vec_pretty(config)
        .map_err(|error| AppError::new("config_serialize", error.to_string(), false))?;
    atomic_write(path, &bytes)
}

pub fn migrate_v1(value: &Value) -> AppResult<AppConfigV3> {
    let mut config = AppConfigV3::default();
    config.general.open_recent_project_on_start = boolean(
        value,
        "/general/openRecentProjectOnStart",
        config.general.open_recent_project_on_start,
    );
    config.general.auto_save = boolean(value, "/general/autoSave", config.general.auto_save);
    config.general.auto_save_delay_ms = integer(
        value,
        "/general/autoSaveInterval",
        config.general.auto_save_delay_ms,
    )
    .clamp(500, 30_000);
    config.general.backup_before_save = boolean(
        value,
        "/general/backupBeforeSave",
        config.general.backup_before_save,
    );
    config.appearance.theme_mode = match text(value, "/appearance/themeMode").as_deref() {
        Some("light") => ThemeMode::Light,
        Some("dark") => ThemeMode::Dark,
        _ => ThemeMode::System,
    };
    config.editor.font_size = integer(
        value,
        "/editor/fontSize",
        u64::from(config.editor.font_size),
    )
    .clamp(12, 28) as u8;
    config.editor.line_height = decimal(
        value,
        "/editor/lineHeight",
        config.editor.line_height.into(),
    )
    .clamp(1.2, 2.2) as f32;
    config.editor.show_line_numbers = boolean(
        value,
        "/editor/showLineNumbers",
        config.editor.show_line_numbers,
    );
    config.editor.line_wrapping =
        boolean(value, "/editor/lineWrapping", config.editor.line_wrapping);
    config.editor.highlight_active_line = boolean(
        value,
        "/editor/highlightActiveLine",
        config.editor.highlight_active_line,
    );
    config.editor.tab_size =
        match integer(value, "/editor/tabSize", u64::from(config.editor.tab_size)) {
            4 => 4,
            8 => 8,
            _ => 2,
        };
    config.article_list.show_cover = value
        .pointer("/articleList/showCover")
        .or_else(|| value.pointer("/postList/showPostCover"))
        .or_else(|| value.pointer("/postList/showCover"))
        .and_then(Value::as_bool)
        .unwrap_or(config.article_list.show_cover);
    config.layout.article_list_width = integer(
        value,
        "/layout/sidebarWidth",
        u64::from(config.layout.article_list_width),
    )
    .clamp(220, 420) as u16;
    let old_preview = integer(value, "/layout/previewWidth", 0);
    if old_preview >= 280 {
        config.layout.preview_width = old_preview.clamp(280, 720) as u16;
    }
    config.layout.preview_visible =
        boolean(value, "/layout/showPreview", config.layout.preview_visible);
    config.image_bed.default_provider = match text(value, "/uploader/defaultType").as_deref() {
        Some("cloudflare-imgbed") => ImageProvider::CloudflareImgbed,
        _ => ImageProvider::Local,
    };
    config.image_bed.cloudflare_api_url = text(value, "/uploader/apiUrl").unwrap_or_default();
    config.image_bed.auto_insert_markdown = boolean(
        value,
        "/uploader/autoInsertMarkdown",
        config.image_bed.auto_insert_markdown,
    );
    config.publish.save_before_run = boolean(
        value,
        "/publish/saveBeforePublish",
        config.publish.save_before_run,
    );
    config.publish.clean_before_generate = boolean(
        value,
        "/publish/cleanBeforeGenerate",
        config.publish.clean_before_generate,
    );
    config.publish.generate_before_deploy = boolean(
        value,
        "/publish/generateBeforeDeploy",
        config.publish.generate_before_deploy,
    );
    config.publish.git_push_after_deploy = boolean(
        value,
        "/publish/gitPushAfterDeploy",
        config.publish.git_push_after_deploy,
    );
    config.update.check_on_start = boolean(
        value,
        "/update/checkUpdateOnStart",
        config.update.check_on_start,
    );
    config.validate()?;
    Ok(config)
}

fn boolean(value: &Value, pointer: &str, fallback: bool) -> bool {
    value
        .pointer(pointer)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn integer(value: &Value, pointer: &str, fallback: u64) -> u64 {
    value
        .pointer(pointer)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
}

fn decimal(value: &Value, pointer: &str, fallback: f64) -> f64 {
    value
        .pointer(pointer)
        .and_then(Value::as_f64)
        .unwrap_or(fallback)
}

fn text(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn migrates_only_supported_fields() {
        let old = json!({
            "appearance": { "themeMode": "dark", "colorScheme": "monokai" },
            "editor": { "fontSize": 18, "tabSize": 4 },
            "uploader": { "defaultType": "custom", "token": "secret" },
            "publish": { "hexoDeployCommand": "danger", "cleanBeforeGenerate": true }
        });
        let migrated = migrate_v1(&old).unwrap();
        assert_eq!(migrated.schema_version, 3);
        assert_eq!(migrated.appearance.theme_mode, ThemeMode::Dark);
        assert_eq!(migrated.editor.font_size, 18);
        assert_eq!(migrated.image_bed.default_provider, ImageProvider::Local);
        assert!(migrated.publish.clean_before_generate);
        assert!(!serde_json::to_string(&migrated).unwrap().contains("danger"));
        assert!(!serde_json::to_string(&migrated).unwrap().contains("secret"));
        assert!(!scrub_legacy_secrets(old).to_string().contains("secret"));
    }

    #[test]
    fn migrates_v2_file_to_v3_with_safe_preview_and_log_defaults() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path());
        let mut value = serde_json::to_value(AppConfigV3::default()).unwrap();
        value["schemaVersion"] = json!(2);
        value.as_object_mut().unwrap().remove("diagnostics");
        let hexo = value["hexo"].as_object_mut().unwrap();
        hexo.remove("autoStartPreview");
        hexo.remove("previewDrafts");
        hexo.remove("defaultPreviewMode");
        hexo.insert("openBrowserAfterStart".to_string(), json!(true));
        fs::write(
            &state.v2_config_path,
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();

        let loaded = load_config(&state).unwrap();
        assert_eq!(loaded.config.schema_version, 3);
        assert!(!loaded.config.hexo.auto_start_preview);
        assert!(loaded.config.hexo.preview_drafts);
        assert_eq!(loaded.config.diagnostics.log_retention_days, 14);
        assert!(state.config_path.is_file());
        assert!(temp.path().join("config-v2.json.1.0.2.bak").is_file());
    }

    #[test]
    fn preserves_corrupt_v3_and_recovers_with_valid_defaults() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path());
        fs::write(&state.config_path, b"{not-valid-json").unwrap();

        let loaded = load_config(&state).unwrap();
        assert_eq!(loaded.config.schema_version, 3);
        assert_eq!(loaded.warnings.len(), 1);
        assert!(state.config_path.is_file());
        assert!(fs::read_dir(temp.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("config-v3.corrupt-")
        }));
    }

    #[test]
    fn loads_v103_config_with_incremental_image_defaults_without_corrupt_backup() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path());
        let mut value = serde_json::to_value(AppConfigV3::default()).unwrap();
        let image_bed = value["imageBed"].as_object_mut().unwrap();
        image_bed.remove("localImageDir");
        image_bed.remove("localMarkdownPrefix");
        image_bed.remove("cloudflareName");
        image_bed.remove("cloudflareConnectionId");
        image_bed.remove("cloudflareTokenId");
        image_bed.remove("uploadFolder");
        fs::write(
            &state.config_path,
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();

        let loaded = load_config(&state).unwrap();
        assert!(loaded.warnings.is_empty());
        assert_eq!(loaded.config.image_bed.local_image_dir, "source/images");
        assert_eq!(loaded.config.image_bed.local_markdown_prefix, "/images");
        assert_eq!(loaded.config.image_bed.upload_folder, "blog");
        assert_eq!(loaded.config.image_bed.cloudflare_connection_id, "primary");
        assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("config-v3.corrupt-")
        }));
    }

    #[test]
    fn ignores_removed_preview_mode_in_v103_config_without_corrupt_backup() {
        let temp = TempDir::new().unwrap();
        let state = AppState::new(temp.path());
        let mut value = serde_json::to_value(AppConfigV3::default()).unwrap();
        value["hexo"]["defaultPreviewMode"] = json!("theme");
        fs::write(
            &state.config_path,
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();

        let loaded = load_config(&state).unwrap();
        assert!(loaded.warnings.is_empty());
        assert_eq!(loaded.config.hexo.preview_port, 4_000);
        assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| {
            entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("config-v3.corrupt-")
        }));
    }
}
