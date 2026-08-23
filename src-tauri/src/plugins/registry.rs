use super::manifest::PluginManifest;
use crate::domain::{AppError, AppResult};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginView {
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub entry_url: Option<String>,
}

#[derive(Default)]
pub struct PluginRegistry {
    pub plugins: BTreeMap<String, (PluginManifest, bool, PathBuf)>,
}

impl PluginRegistry {
    #[cfg(test)]
    pub(crate) fn from_plugins(plugins: BTreeMap<String, (PluginManifest, bool, PathBuf)>) -> Self {
        Self { plugins }
    }
    pub fn load(root: &Path) -> AppResult<Self> {
        let mut registry = Self::default();
        let entries =
            fs::read_dir(root).map_err(|error| AppError::io("读取插件目录失败", error))?;
        for entry in entries.flatten().filter(|entry| entry.path().is_dir()) {
            let path = entry.path();
            let manifest_path = path.join("manifest.json");
            let Ok(content) = fs::read_to_string(manifest_path) else {
                continue;
            };
            let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) else {
                continue;
            };
            if manifest.validate().is_ok() && path.join(&manifest.entry).is_file() {
                let enabled = path.join(".enabled").exists();
                registry
                    .plugins
                    .insert(manifest.id.clone(), (manifest, enabled, path));
            }
        }
        Ok(registry)
    }
    pub fn views(&self) -> Vec<PluginView> {
        self.plugins
            .values()
            .map(|(manifest, enabled, _)| PluginView {
                manifest: manifest.clone(),
                enabled: *enabled,
                entry_url: enabled.then(|| {
                    if cfg!(windows) {
                        format!(
                            "http://hlex-plugin.localhost/{}/{}",
                            manifest.id, manifest.entry
                        )
                    } else {
                        format!("hlex-plugin://{}/{}", manifest.id, manifest.entry)
                    }
                }),
            })
            .collect()
    }
}
