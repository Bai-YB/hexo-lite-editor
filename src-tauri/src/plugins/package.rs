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
    copy_safe_tree(&source, &target)?;
    Ok(target)
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
