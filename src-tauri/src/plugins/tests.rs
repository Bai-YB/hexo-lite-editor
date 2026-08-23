use super::manifest::PluginManifest;

#[test]
fn rejects_incompatible_api_and_undeclared_native_permissions() {
    let mut manifest = PluginManifest {
        id: "com.example.test".into(),
        name: "Test".into(),
        version: "1.0.0".into(),
        api_version: "1.0".into(),
        entry: "index.js".into(),
        permissions: vec![],
        contributes: serde_json::json!({}),
    };
    assert_eq!(
        manifest.validate().unwrap_err().code,
        "plugin_api_incompatible"
    );
    manifest.api_version = "0.1".into();
    manifest.permissions = vec!["system:command".into()];
    assert_eq!(
        manifest.validate().unwrap_err().code,
        "plugin_permission_invalid"
    );
}

#[test]
fn enabled_image_provider_exposes_only_its_worker_entry() {
    use super::registry::PluginRegistry;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    let manifest = PluginManifest {
        id: "com.example.imagebed".into(),
        name: "Example".into(),
        version: "1.0.0".into(),
        api_version: "0.1".into(),
        entry: "index.js".into(),
        permissions: vec!["image:read-selected".into()],
        contributes: serde_json::json!({ "imageBedProviders": ["example"] }),
    };
    let mut plugins = BTreeMap::new();
    plugins.insert(
        manifest.id.clone(),
        (manifest, true, PathBuf::from("plugin")),
    );
    let registry = PluginRegistry::from_plugins(plugins);
    let view = registry.views().pop().unwrap();
    assert!(view
        .entry_url
        .unwrap()
        .ends_with("/com.example.imagebed/index.js"));
}
