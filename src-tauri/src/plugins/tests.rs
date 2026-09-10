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

#[test]
fn uninstall_preserves_settings_and_reinstall_starts_disabled() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    let root = temp.path().join("plugins");
    std::fs::create_dir_all(&source).unwrap();
    std::fs::write(source.join("manifest.json"), r#"{"id":"com.example.test","name":"Test","version":"1","apiVersion":"0.1","entry":"index.js"}"#).unwrap();
    std::fs::write(source.join("index.js"), "self.onmessage = () => {};").unwrap();
    std::fs::write(source.join(".enabled"), "0.1").unwrap();
    let installed = super::package::install_directory(&source, &root).unwrap();
    assert!(!installed.join(".enabled").exists());
    std::fs::write(
        installed.join("settings.json"),
        r#"{"endpoint":"https://example.com"}"#,
    )
    .unwrap();
    super::package::uninstall_directory(&installed, &root, "com.example.test", true).unwrap();
    assert!(!installed.exists());
    let installed = super::package::install_directory(&source, &root).unwrap();
    assert!(std::fs::read_to_string(installed.join("settings.json"))
        .unwrap()
        .contains("https://example.com"));
    super::package::uninstall_directory(&installed, &root, "com.example.test", false).unwrap();
    let installed = super::package::install_directory(&source, &root).unwrap();
    assert!(!installed.join("settings.json").exists());
}
