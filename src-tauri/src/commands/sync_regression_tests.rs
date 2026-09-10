use super::*;
use crate::app::ProjectSession;
use std::collections::HashMap;

fn record(root: &Path) -> SyncRecord {
    serde_json::from_value(serde_json::json!({
        "projectPath": path_key(root), "provider": "github", "repository": "fixture",
        "branch": DEFAULT_BRANCH, "imageDir": "source/images", "enabled": true,
        "visibility": "private", "status": "localPending"
    }))
    .unwrap()
}

#[test]
fn obsolete_schedule_cleanup_keeps_the_latest_receiver_alive() {
    let temp = tempfile::TempDir::new().unwrap();
    let state = AppState::new(temp.path());
    let old_id = Uuid::new_v4();
    let current_id = Uuid::new_v4();
    let (old_sender, mut old_receiver) = tokio::sync::oneshot::channel();
    let (new_sender, mut new_receiver) = tokio::sync::oneshot::channel();
    state
        .sync_schedules
        .lock()
        .unwrap()
        .insert("project".into(), (old_id, old_sender));
    let (_, previous) = state
        .sync_schedules
        .lock()
        .unwrap()
        .insert("project".into(), (current_id, new_sender))
        .unwrap();
    previous.send(()).unwrap();
    assert_eq!(old_receiver.try_recv(), Ok(()));
    finish_sync_schedule(&state, "project", old_id);
    assert!(matches!(
        new_receiver.try_recv(),
        Err(tokio::sync::oneshot::error::TryRecvError::Empty)
    ));
    assert_eq!(
        state.sync_schedules.lock().unwrap()["project"].0,
        current_id
    );
    finish_sync_schedule(&state, "project", current_id);
    assert!(state.sync_schedules.lock().unwrap().is_empty());
}

#[test]
fn legacy_initialization_preserves_config_theme_and_drafts() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("project");
    let cache = temp.path().join("cache");
    let state = AppState::new(&temp.path().join("config"));
    for folder in ["source/_posts", "source/_drafts", "themes/test"] {
        fs::create_dir_all(root.join(folder)).unwrap();
    }
    fs::create_dir_all(cache.join("source/_posts")).unwrap();
    for path in [
        "_config.yml",
        "package.json",
        "themes/test/_config.yml",
        "source/_drafts/private.md",
        "source/_posts/post.md",
    ] {
        fs::write(root.join(path), "local").unwrap();
    }
    fs::write(cache.join("source/_posts/post.md"), "remote").unwrap();
    let local = local_snapshot(&root, "source/images").unwrap();
    let remote = local_snapshot(&cache, "source/images").unwrap();
    let base = overwrite_base_for_manifest(&local, &remote, &record(&root), LEGACY_MANIFEST_SCHEMA);
    apply_remote(&state, &root, &cache, &local, &remote, &base).unwrap();
    for path in [
        "_config.yml",
        "package.json",
        "themes/test/_config.yml",
        "source/_drafts/private.md",
    ] {
        assert_eq!(fs::read_to_string(root.join(path)).unwrap(), "local");
    }
    assert_eq!(
        fs::read_to_string(root.join("source/_posts/post.md")).unwrap(),
        "remote"
    );
}

#[test]
fn upload_uses_captured_bytes_after_the_editor_saves_again() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("project");
    let cache = temp.path().join("cache");
    fs::create_dir_all(root.join("source/_posts")).unwrap();
    fs::write(root.join("source/_posts/post.md"), "captured").unwrap();
    let snapshot = local_snapshot(&root, "source/images").unwrap();
    fs::write(root.join("source/_posts/post.md"), "newly saved").unwrap();
    copy_snapshot_to_plain_cache(&root, &cache, &snapshot).unwrap();
    let copied = fs::read(cache.join("source/_posts/post.md")).unwrap();
    assert_eq!(copied, b"captured");
    assert_eq!(hash_bytes(&copied), snapshot["source/_posts/post.md"].hash);
    assert_ne!(
        hash_map(&local_snapshot(&root, "source/images").unwrap()),
        hash_map(&snapshot)
    );
}

#[test]
fn remote_apply_rejects_changes_saved_after_the_decision_snapshot() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("project");
    let cache = temp.path().join("cache");
    let state = AppState::new(&temp.path().join("config"));
    fs::create_dir_all(root.join("source/_posts")).unwrap();
    fs::create_dir_all(cache.join("source/_posts")).unwrap();
    fs::write(root.join("source/_posts/post.md"), "initial").unwrap();
    fs::write(cache.join("source/_posts/post.md"), "remote").unwrap();
    let local = local_snapshot(&root, "source/images").unwrap();
    let remote = local_snapshot(&cache, "source/images").unwrap();
    fs::write(root.join("source/_posts/post.md"), "new input").unwrap();
    assert!(apply_remote(&state, &root, &cache, &local, &remote, &hash_map(&local)).is_err());
    assert_eq!(
        fs::read_to_string(root.join("source/_posts/post.md")).unwrap(),
        "new input"
    );
}

#[test]
fn registry_merges_projects_and_rejects_a_stale_reenable() {
    let temp = tempfile::TempDir::new().unwrap();
    let state = AppState::new(temp.path());
    let a = record(&temp.path().join("a"));
    let b = record(&temp.path().join("b"));
    save_registry(
        &state,
        &mut SyncRegistry {
            records: vec![a.clone(), b.clone()],
            baseline: vec![],
        },
    )
    .unwrap();
    let mut first = load_registry(&state).unwrap();
    let mut second = load_registry(&state).unwrap();
    first.records[0].message = Some("A completed".into());
    second.records[1].message = Some("B completed".into());
    save_registry(&state, &mut first).unwrap();
    save_registry(&state, &mut second).unwrap();
    let merged = load_registry(&state).unwrap();
    assert_eq!(
        merged
            .records
            .iter()
            .find(|item| item.project_path == a.project_path)
            .unwrap()
            .message
            .as_deref(),
        Some("A completed")
    );
    assert_eq!(
        merged
            .records
            .iter()
            .find(|item| item.project_path == b.project_path)
            .unwrap()
            .message
            .as_deref(),
        Some("B completed")
    );
    let mut disabled = load_registry(&state).unwrap();
    let mut stale = load_registry(&state).unwrap();
    disabled
        .records
        .retain(|item| item.project_path != a.project_path);
    save_registry(&state, &mut disabled).unwrap();
    stale
        .records
        .iter_mut()
        .find(|item| item.project_path == a.project_path)
        .unwrap()
        .status = ContentSyncStatus::Synced;
    assert!(save_registry(&state, &mut stale).is_err());
    assert!(!load_registry(&state)
        .unwrap()
        .records
        .iter()
        .any(|item| item.project_path == a.project_path));
}

#[test]
fn old_project_rescan_cannot_replace_the_active_project() {
    let temp = tempfile::TempDir::new().unwrap();
    let state = AppState::new(temp.path());
    let active_root = temp.path().join("active");
    *state.project.write().unwrap() = Some(ProjectSession {
        id: "active".into(),
        generation: 2,
        name: "Active".into(),
        root: active_root.clone(),
        warnings: vec![],
        article_summaries: vec![],
        articles: HashMap::new(),
        assets: HashMap::new(),
        remote_assets: HashMap::new(),
    });
    assert!(crate::commands::project::rescan_project_after_sync(
        &state,
        &temp.path().join("old"),
        "old",
        1
    )
    .is_err());
    assert!(crate::commands::project::rescan_project_after_sync(
        &state,
        &temp.path().join("old"),
        "active",
        2
    )
    .is_err());
    let current = state.project.read().unwrap();
    assert_eq!(current.as_ref().unwrap().root, active_root);
    assert_eq!(current.as_ref().unwrap().generation, 2);
}

fn open_fixture_session(state: &AppState, root: &Path) -> (String, u64, String) {
    fs::create_dir_all(root.join("source/_posts")).unwrap();
    fs::write(root.join("_config.yml"), "title: fixture\n").unwrap();
    fs::write(
        root.join("source/_posts/post.md"),
        "---\ntitle: Original\n---\nlocal",
    )
    .unwrap();
    let root = root.canonicalize().unwrap();
    let (summaries, mut articles, assets) = crate::engine::scan_articles(&root).unwrap();
    let article_id = summaries[0].article_id.clone();
    articles.get_mut(&article_id).unwrap().revision = 7;
    let project_id = "fixture-project".to_string();
    let generation = state.next_generation();
    *state.project.write().unwrap() = Some(ProjectSession {
        id: project_id.clone(),
        generation,
        name: "Fixture".into(),
        root,
        warnings: vec![],
        article_summaries: summaries,
        articles,
        assets,
        remote_assets: HashMap::new(),
    });
    (project_id, generation, article_id)
}

#[test]
fn rescan_preserves_surviving_article_identity_and_revision_with_real_files() {
    let temp = tempfile::TempDir::new().unwrap();
    let state = AppState::new(&temp.path().join("config"));
    let root = temp.path().join("project");
    let (project_id, generation, article_id) = open_fixture_session(&state, &root);
    let root = root.canonicalize().unwrap();
    fs::write(
        root.join("source/_posts/post.md"),
        "---\ntitle: Remote title\n---\nremote",
    )
    .unwrap();
    fs::write(
        root.join("source/_posts/new.md"),
        "---\ntitle: New article\n---\nnew",
    )
    .unwrap();
    let result =
        crate::commands::project::rescan_project_after_sync(&state, &root, &project_id, generation)
            .unwrap();
    assert_eq!(result.previous_generation, generation);
    assert!(result.generation > generation);
    assert_eq!(result.articles.len(), 2);
    assert_eq!(
        result
            .articles
            .iter()
            .find(|article| article.title == "Remote title")
            .unwrap()
            .article_id,
        article_id
    );
    let project = state.project.read().unwrap();
    let surviving = &project.as_ref().unwrap().articles[&article_id];
    assert_eq!(surviving.revision, 7);
    assert_eq!(surviving.canonical_path, root.join("source/_posts/post.md"));
}

#[test]
fn remote_apply_invalidates_old_saves_before_unlocking_and_keeps_ui_transition() {
    let temp = tempfile::TempDir::new().unwrap();
    let state = AppState::new(&temp.path().join("config"));
    let root = temp.path().join("project");
    let cache = temp.path().join("cache");
    let (project_id, generation, article_id) = open_fixture_session(&state, &root);
    let root = root.canonicalize().unwrap();
    fs::create_dir_all(cache.join("source/_posts")).unwrap();
    fs::write(
        cache.join("source/_posts/post.md"),
        "---\ntitle: Remote\n---\nremote",
    )
    .unwrap();
    let local = local_snapshot(&root, "source/images").unwrap();
    let remote = local_snapshot(&cache, "source/images").unwrap();
    apply_remote(&state, &root, &cache, &local, &remote, &BTreeMap::new()).unwrap();
    // This is exactly the file-lock/identity gate executed by queued save_document calls.
    let file_lock = state.project_file_lock(&root).unwrap();
    let _file_guard = file_lock.lock().unwrap();
    assert!(state
        .with_project(&project_id, Some(generation), |_| Ok(()))
        .is_err());
    let project = state.project.read().unwrap();
    let project = project.as_ref().unwrap();
    assert!(project.generation > generation);
    assert_eq!(project.articles[&article_id].revision, 7);
    let pending = state.pending_sync_rescans.lock().unwrap();
    let transition = &pending[&root];
    assert_eq!(transition.project_id, project_id);
    assert_eq!(transition.previous_generation, generation);
    assert_eq!(transition.generation, project.generation);
    assert_eq!(transition.articles[0].article_id, article_id);
    assert!(fs::read_to_string(root.join("source/_posts/post.md"))
        .unwrap()
        .ends_with("remote"));
}

#[test]
fn failed_remote_rescan_rolls_back_disk_and_leaves_current_session_usable() {
    let temp = tempfile::TempDir::new().unwrap();
    let state = AppState::new(&temp.path().join("config"));
    let root = temp.path().join("project");
    let (project_id, generation, _) = open_fixture_session(&state, &root);
    let root = root.canonicalize().unwrap();
    let local = local_snapshot(&root, "source/images").unwrap();
    // Deleting the only Hexo config makes the resulting project invalid.
    let result = apply_remote(
        &state,
        &root,
        temp.path(),
        &local,
        &Snapshot::new(),
        &hash_map(&local),
    );
    assert!(result.is_err());
    assert_eq!(
        fs::read_to_string(root.join("_config.yml")).unwrap(),
        "title: fixture\n"
    );
    assert!(fs::read_to_string(root.join("source/_posts/post.md"))
        .unwrap()
        .ends_with("local"));
    assert!(state
        .with_project(&project_id, Some(generation), |_| Ok(()))
        .is_ok());
    assert!(state.pending_sync_rescans.lock().unwrap().is_empty());
}

#[test]
fn registry_updates_baseline_after_merge_and_surfaces_persistence_failure() {
    let temp = tempfile::TempDir::new().unwrap();
    let state = AppState::new(temp.path());
    let mut registry = SyncRegistry {
        records: vec![record(temp.path())],
        baseline: vec![],
    };
    save_registry(&state, &mut registry).unwrap();
    assert_eq!(registry.records, registry.baseline);
    registry.records[0].message = Some("first".into());
    save_registry(&state, &mut registry).unwrap();
    registry.records[0].message = Some("second".into());
    save_registry(&state, &mut registry).unwrap();
    assert_eq!(
        load_registry(&state).unwrap().records[0].message.as_deref(),
        Some("second")
    );
    fs::remove_file(&state.sync_registry_path).unwrap();
    fs::create_dir(&state.sync_registry_path).unwrap();
    registry.records[0].status = ContentSyncStatus::Synced;
    let view = view_from_record(&registry.records[0]);
    let result = persist_sync_view(&state, &mut registry, view);
    assert_eq!(result.status, ContentSyncStatus::Error);
    assert!(result.message.unwrap().contains("同步状态保存失败"));
    assert_ne!(registry.records, registry.baseline);
    assert_eq!(
        sync_before_open(&state, temp.path()).unwrap().status,
        ContentSyncStatus::Error
    );
}
