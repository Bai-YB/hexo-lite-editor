use super::manifest::PluginManifest;
use crate::domain::{AppError, AppResult};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn install_directory(source: &Path, plugins_root: &Path) -> AppResult<PathBuf> {
    let source = source
        .canonicalize()
        .map_err(|error| AppError::io("读取插件包失败", error))?;
    let manifest: PluginManifest = serde_json::from_str(
        &fs::read_to_string(source.join("manifest.json"))
            .map_err(|error| AppError::io("读取插件 manifest 失败", error))?,
    )
    .map_err(|error| AppError::new("plugin_manifest_invalid", error.to_string(), true))?;
    manifest.validate()?;
    if !source.join(&manifest.entry).is_file() {
        return Err(AppError::new(
            "plugin_entry_missing",
            "插件入口不存在。",
            true,
        ));
    }
    let target = plugins_root.join(&manifest.id);
    if target.exists() {
        return Err(AppError::new("plugin_exists", "插件已经安装。", true));
    }
    let staging = plugins_root.join(format!(".install-{}", uuid::Uuid::new_v4()));
    let result = (|| {
        copy_safe_tree(&source, &staging)?;
        // Installation always requires an explicit enable action in the host.
        if staging.join(".enabled").exists() {
            fs::remove_file(staging.join(".enabled"))
                .map_err(|error| AppError::io("初始化插件状态失败", error))?;
        }
        let retained = retained_settings_path(plugins_root, &manifest.id);
        if retained.is_file() {
            fs::copy(retained, staging.join("settings.json"))
                .map_err(|error| AppError::io("恢复插件设置失败", error))?;
        }
        fs::rename(&staging, &target).map_err(|error| AppError::io("完成插件安装失败", error))
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result?;
    Ok(target)
}

fn retained_settings_path(plugins_root: &Path, plugin_id: &str) -> PathBuf {
    plugins_root
        .join(".retained-settings")
        .join(format!("{plugin_id}.json"))
}

pub fn uninstall_directory(
    path: &Path,
    plugins_root: &Path,
    plugin_id: &str,
    preserve_settings: bool,
) -> AppResult<()> {
    let canonical_root = plugins_root
        .canonicalize()
        .map_err(|error| AppError::io("读取插件目录失败", error))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|error| AppError::io("读取插件目录失败", error))?;
    if canonical_path.parent() != Some(canonical_root.as_path()) {
        return Err(AppError::invalid("插件目录无效。"));
    }
    let retained = retained_settings_path(plugins_root, plugin_id);
    let settings = path.join("settings.json");
    if preserve_settings && settings.is_file() {
        let bytes = fs::read(settings).map_err(|error| AppError::io("保留插件设置失败", error))?;
        crate::platform::atomic_write(&retained, &bytes)?;
    } else if !preserve_settings && retained.exists() {
        fs::remove_file(retained).map_err(|error| AppError::io("删除保留的插件设置失败", error))?;
    }
    fs::remove_dir_all(canonical_path).map_err(|error| AppError::io("卸载插件失败", error))
}

fn copy_safe_tree(source: &Path, target: &Path) -> AppResult<()> {
    fs::create_dir_all(target).map_err(|error| AppError::io("创建插件目录失败", error))?;
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry = entry
            .map_err(|error| AppError::new("plugin_package_invalid", error.to_string(), true))?;
        if entry.file_type().is_symlink() {
            return Err(AppError::new(
                "plugin_package_invalid",
                "插件包不能包含符号链接。",
                false,
            ));
        }
        let relative = entry
            .path()
            .strip_prefix(source)
            .map_err(|_| AppError::invalid("插件路径无效。"))?;
        let destination = target.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&destination)
                .map_err(|error| AppError::io("创建插件目录失败", error))?;
        } else {
            fs::copy(entry.path(), destination)
                .map_err(|error| AppError::io("复制插件文件失败", error))?;
        }
    }
    Ok(())
}
