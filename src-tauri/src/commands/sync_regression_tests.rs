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

fn write_fixture(root: &Path, path: &str, content: &str) {
    let target = root.join(path);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(target, content).unwrap();
}

/// Projects copied from a Windows archive, share, or disk keep the read-only
/// attribute, which used to fail every macOS sync with
/// `Permission denied (os error 13)`.
fn make_read_only(path: &Path) {
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(path, permissions).unwrap();
}

fn register_fixture(state: &AppState, root: &Path, remote: &Path) {
    let mut item = record(root);
    item.repository = remote.to_string_lossy().into_owned();
    save_registry(
        state,
        &mut SyncRegistry {
            records: vec![item],
            baseline: vec![],
        },
    )
    .unwrap();
}

#[test]
fn first_sync_reports_same_path_conflicts_then_preserves_unique_files() {
    let temp = tempfile::TempDir::new().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    let remote = temp.path().join("remote.git");
    super::tests::git_test_init_bare(&remote);
    let state_a = AppState::new(&temp.path().join("state-a"));
    let state_b = AppState::new(&temp.path().join("state-b"));
    write_fixture(&a, "_config.yml", "from A");
    write_fixture(&a, "source/_posts/a.md", "article A");
    write_fixture(&b, "_config.yml", "from B");
    write_fixture(&b, "source/_redirects", "redirect B");
    register_fixture(&state_a, &a, &remote);
    register_fixture(&state_b, &b, &remote);
    assert_eq!(
        run_sync_for_root(&state_a, &a, "auto").status,
        ContentSyncStatus::Synced
    );
    let conflict = run_sync_for_root(&state_b, &b, "auto");
    assert_eq!(conflict.status, ContentSyncStatus::Conflict);
    assert_eq!(conflict.conflicts, vec!["_config.yml"]);
    assert_eq!(fs::read_to_string(b.join("_config.yml")).unwrap(), "from B");
    let registry = load_registry(&state_b).unwrap();
    let item = &registry.records[0];
    let cache = record_cache_dir(&state_b, &b, item);
    let local = local_snapshot(&b, "source/images").unwrap();
    let remote = snapshot_from_manifest(&cache, &read_manifest(&cache).unwrap()).unwrap();
    apply_conflict_choices(
        &state_b,
        &b,
        &cache,
        item,
        &local,
        &remote,
        &BTreeMap::from([("_config.yml".into(), "remote".into())]),
    )
    .unwrap();
    assert_eq!(fs::read_to_string(b.join("_config.yml")).unwrap(), "from A");
    assert!(b.join("source/_posts/a.md").exists());
    assert!(b.join("source/_redirects").exists());
}

#[test]
fn two_machines_merge_complete_sources_and_handle_deletions_without_losing_local_edits() {
    let temp = tempfile::TempDir::new().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    let remote = temp.path().join("remote.git");
    super::tests::git_test_init_bare(&remote);
    let state_a = AppState::new(&temp.path().join("state-a"));
    let state_b = AppState::new(&temp.path().join("state-b"));
    write_fixture(&a, "source/_posts/a.md", "article from A");
    write_fixture(&a, "source/_redirects", "/old /new");
    write_fixture(&a, ".gitattributes", "*.md text eol=crlf\n");
    write_fixture(&a, ".gitignore", "source/_posts/\n");
    write_fixture(&b, "_config.yml", "theme: quiet");
    write_fixture(&b, "themes/quiet/layout/module.ejs", "new module");
    register_fixture(&state_a, &a, &remote);
    register_fixture(&state_b, &b, &remote);
    let sync = |state: &AppState, root: &Path| {
        let result = run_sync_for_root(state, root, "auto");
        assert_eq!(result.status, ContentSyncStatus::Synced, "{result:?}");
    };
    sync(&state_a, &a);
    let cache_a = record_cache_dir(&state_a, &a, &load_registry(&state_a).unwrap().records[0]);
    write_fixture(&cache_a, "unrelated-cache.txt", "must remain local");
    sync(&state_b, &b);
    sync(&state_a, &a);
    for root in [&a, &b] {
        for path in [
            "source/_posts/a.md",
            "source/_redirects",
            "_config.yml",
            "themes/quiet/layout/module.ejs",
        ] {
            assert!(
                root.join(path).is_file(),
                "missing {} in {}",
                path,
                root.display()
            );
        }
    }
    write_fixture(&a, "source/_posts/a.md", "updated article");
    write_fixture(&b, "_config.yml", "theme: quiet\nurl: https://example.com");
    sync(&state_b, &b);
    let background = run_sync_for_root(&state_a, &a, "push");
    assert_eq!(
        background.status,
        ContentSyncStatus::RemoteAhead,
        "{background:?}"
    );
    sync(&state_a, &a);
    assert_eq!(
        fs::read_to_string(a.join("source/_posts/a.md")).unwrap(),
        "updated article"
    );
    assert!(fs::read_to_string(a.join("_config.yml"))
        .unwrap()
        .contains("https://example.com"));
    assert!(
        !git(&cache_a, &["ls-tree", "-r", "--name-only", "HEAD"], false)
            .unwrap()
            .contains("unrelated-cache.txt")
    );
    assert_eq!(
        fs::read_to_string(b.join(".gitattributes")).unwrap(),
        "*.md text eol=crlf\n"
    );
    sync(&state_b, &b);
    fs::remove_file(a.join("source/_redirects")).unwrap();
    sync(&state_a, &a);
    write_fixture(&b, "source/_posts/local-only.md", "new local article");
    sync(&state_b, &b);
    assert!(!b.join("source/_redirects").exists());
    assert!(b.join("source/_posts/local-only.md").is_file());
    fs::remove_file(a.join("source/_posts/a.md")).unwrap();
    sync(&state_a, &a);
    write_fixture(&b, "source/_posts/a.md", "local edit must survive");
    let conflict = run_sync_for_root(&state_b, &b, "auto");
    assert_eq!(conflict.status, ContentSyncStatus::Conflict);
    assert_eq!(conflict.conflicts, vec!["source/_posts/a.md"]);
    assert_eq!(
        fs::read_to_string(b.join("source/_posts/a.md")).unwrap(),
        "local edit must survive"
    );
}

#[test]
fn successful_empty_sync_remains_a_baseline() {
    let temp = tempfile::TempDir::new().unwrap();
    let mut item = record(temp.path());
    assert!(!has_sync_baseline(&item));
    item.last_synced_at = Some("2026-09-14T00:00:00Z".into());
    assert!(has_sync_baseline(&item));
    assert!(summarize_files(std::iter::empty(), Some(&item)).baseline_available);
}

#[test]
fn post_upload_scan_failure_keeps_successful_baseline_but_requires_a_retry() {
    let temp = tempfile::TempDir::new().unwrap();
    let mut item = record(temp.path());
    item.status = ContentSyncStatus::Synced;
    item.base_files
        .insert("source/a.md".into(), "uploaded-hash".into());
    item.last_synced_at = Some("2026-09-14T00:00:00Z".into());
    verify_local_after_upload(&mut item, Err(AppError::invalid("scan failed")));
    assert_eq!(item.status, ContentSyncStatus::LocalPending);
    assert_eq!(item.base_files["source/a.md"], "uploaded-hash");
    assert!(item.last_synced_at.is_some());
    assert!(item
        .message
        .unwrap()
        .contains("已上传，无法确认本地最新状态"));
}

#[test]
fn cache_file_directory_swaps_prune_only_empty_managed_parents() {
    let temp = tempfile::TempDir::new().unwrap();
    let cache = temp.path().join("cache");
    let nested = Snapshot::from([(
        "source/a/file.md".into(),
        FileSnapshot {
            hash: hash_bytes(b"nested"),
            bytes: (&b"nested"[..]).into(),
        },
    )]);
    let flat = Snapshot::from([(
        "source/a".into(),
        FileSnapshot {
            hash: hash_bytes(b"flat"),
            bytes: (&b"flat"[..]).into(),
        },
    )]);
    let save = |snapshot: &Snapshot| {
        write_manifest(
            &cache,
            &SyncManifest {
                schema_version: PROJECT_MANIFEST_SCHEMA,
                image_dir: "source/images".into(),
                files: hash_map(snapshot),
            },
        )
        .unwrap()
    };
    copy_snapshot_incrementally(&cache, &nested).unwrap();
    save(&nested);
    copy_snapshot_incrementally(&cache, &flat).unwrap();
    assert_eq!(fs::read(cache.join("source/a")).unwrap(), b"flat");
    save(&flat);
    copy_snapshot_incrementally(&cache, &nested).unwrap();
    assert_eq!(fs::read(cache.join("source/a/file.md")).unwrap(), b"nested");
    save(&nested);
    write_fixture(&cache, "source/a/untracked.txt", "keep");
    let error = copy_snapshot_incrementally(&cache, &flat).unwrap_err();
    assert!(error.message.contains("未跟踪内容"));
    assert_eq!(
        fs::read(cache.join("source/a/untracked.txt")).unwrap(),
        b"keep"
    );
}

#[test]
fn remote_file_directory_swaps_fail_before_any_local_write_or_transaction() {
    for remote_is_directory in [true, false] {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path().join("project");
        let cache = temp.path().join("cache");
        let state = AppState::new(&temp.path().join("state"));
        let (local_path, remote_path) = if remote_is_directory {
            ("source/a", "source/a/file.md")
        } else {
            ("source/a/file.md", "source/a")
        };
        write_fixture(&root, local_path, "local preserved");
        write_fixture(&root, "source/unrelated.md", "unrelated preserved");
        write_fixture(&cache, remote_path, "remote");
        let local = local_snapshot(&root, "source/images").unwrap();
        let remote = local_snapshot(&cache, "source/images").unwrap();
        let error =
            apply_remote(&state, &root, &cache, &local, &remote, &hash_map(&local)).unwrap_err();
        assert!(error.message.contains("文件与文件夹同名"));
        assert_eq!(
            fs::read_to_string(root.join(local_path)).unwrap(),
            "local preserved"
        );
        assert_eq!(
            fs::read_to_string(root.join("source/unrelated.md")).unwrap(),
            "unrelated preserved"
        );
        assert!(!state
            .sync_cache_dir
            .join(cache_key(&path_key(&root)))
            .join("apply-transaction.json")
            .exists());
    }
}

#[test]
fn cache_keeps_unchanged_files_and_untracked_entries_while_removing_tracked_deletions() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("root");
    let cache = temp.path().join("cache");
    write_fixture(&root, "source/_posts/keep.md", "same");
    write_fixture(&root, "source/_posts/remove.md", "remove");
    let first = local_snapshot(&root, "source/images").unwrap();
    copy_snapshot_to_plain_cache(&root, &cache, &first).unwrap();
    write_manifest(
        &cache,
        &SyncManifest {
            schema_version: PROJECT_MANIFEST_SCHEMA,
            image_dir: "source/images".into(),
            files: hash_map(&first),
        },
    )
    .unwrap();
    write_fixture(&cache, "notes.txt", "untracked cache entry");
    let fixed = std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    fs::File::options()
        .write(true)
        .open(cache.join("source/_posts/keep.md"))
        .unwrap()
        .set_modified(fixed)
        .unwrap();
    fs::remove_file(root.join("source/_posts/remove.md")).unwrap();
    write_fixture(&root, "source/_redirects", "new redirect");
    copy_snapshot_to_plain_cache(
        &root,
        &cache,
        &local_snapshot(&root, "source/images").unwrap(),
    )
    .unwrap();
    assert_eq!(
        fs::metadata(cache.join("source/_posts/keep.md"))
            .unwrap()
            .modified()
            .unwrap(),
        fixed
    );
    assert!(!cache.join("source/_posts/remove.md").exists());
    assert!(cache.join("source/_redirects").exists());
    assert_eq!(
        fs::read_to_string(cache.join("notes.txt")).unwrap(),
        "untracked cache entry"
    );
}

#[test]
fn excluded_large_files_do_not_block_sync_but_included_large_files_do() {
    let temp = tempfile::TempDir::new().unwrap();
    write_fixture(temp.path(), "source/_posts/hello.md", "hello");
    let excluded = fs::File::create(temp.path().join("debug.log")).unwrap();
    excluded.set_len(MAX_SYNC_FILE_BYTES + 1).unwrap();
    assert_eq!(
        local_snapshot(temp.path(), "source/images").unwrap().len(),
        1
    );
    let included = fs::File::create(temp.path().join("large.dat")).unwrap();
    included.set_len(MAX_SYNC_FILE_BYTES + 1).unwrap();
    assert_eq!(
        local_snapshot(temp.path(), "source/images")
            .unwrap_err()
            .code,
        "sync_file_too_large"
    );
}

#[test]
fn webdav_transfer_count_is_unique_unknown_objects() {
    let known_hash = hash_bytes(b"known");
    let new_hash = hash_bytes(b"new");
    let manifest = SyncManifest {
        schema_version: PROJECT_MANIFEST_SCHEMA,
        image_dir: "source/images".into(),
        files: BTreeMap::from([
            ("source/a.md".into(), known_hash.clone()),
            ("source/b.md".into(), new_hash.clone()),
            ("source/c.md".into(), new_hash.clone()),
        ]),
    };
    let pending = pending_webdav_objects(&manifest, &BTreeSet::from([known_hash]));
    assert_eq!(pending.len(), 1);
    assert!(pending.contains_key(new_hash.as_str()));
}

#[test]
fn summary_counts_full_source_scope_and_only_logical_changes() {
    let temp = tempfile::TempDir::new().unwrap();
    let mut item = record(temp.path());
    item.base_files = BTreeMap::from([
        ("source/_posts/a.md".into(), "same".into()),
        ("removed.md".into(), "gone".into()),
    ]);
    let summary = summarize_files(
        [
            ("source/_posts/a.md", "same", 10),
            ("_config.yml", "changed", 20),
            ("source/_redirects", "new", 30),
            ("themes/quiet/layout/module.ejs", "module", 40),
            ("source/images/a.png", "image", 50),
        ]
        .into_iter(),
        Some(&item),
    );
    assert_eq!((summary.file_count, summary.total_bytes), (5, 150));
    assert_eq!(
        (
            summary.pending_file_count,
            summary.pending_bytes,
            summary.deleted_file_count
        ),
        (4, 140, 1)
    );
    assert_eq!(
        summary
            .categories
            .iter()
            .map(|item| item.file_count)
            .collect::<Vec<_>>(),
        vec![1, 2, 1, 1]
    );
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
fn apply_replaces_a_read_only_project_file() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("project");
    let cache = temp.path().join("cache");
    let state = AppState::new(&temp.path().join("config"));
    write_fixture(&root, "source/_posts/post.md", "local");
    write_fixture(&cache, "source/_posts/post.md", "remote");
    let local = local_snapshot(&root, "source/images").unwrap();
    let remote = local_snapshot(&cache, "source/images").unwrap();
    make_read_only(&root.join("source/_posts/post.md"));

    apply_remote(&state, &root, &cache, &local, &remote, &hash_map(&local)).unwrap();

    assert_eq!(
        fs::read_to_string(root.join("source/_posts/post.md")).unwrap(),
        "remote"
    );
}

#[test]
fn recovery_restores_a_read_only_project_file() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("project");
    let state = AppState::new(&temp.path().join("config"));
    write_fixture(&root, "source/_posts/hello.md", "partial remote");
    let key = cache_key(&path_key(&root));
    let backup = state.sync_backup_dir.join(&key).join("backup-1");
    let transaction = state
        .sync_cache_dir
        .join(&key)
        .join("apply-transaction.json");
    fs::create_dir_all(backup.join("source/_posts")).unwrap();
    fs::create_dir_all(transaction.parent().unwrap()).unwrap();
    fs::write(backup.join("source/_posts/hello.md"), "local before sync").unwrap();
    let journal = ApplyTransaction {
        backup_name: "backup-1".to_string(),
        operations: BTreeSet::from(["source/_posts/hello.md".to_string()]),
    };
    fs::write(&transaction, serde_json::to_vec(&journal).unwrap()).unwrap();
    make_read_only(&root.join("source/_posts/hello.md"));

    recover_pending_transaction(&state, &root).unwrap();

    assert_eq!(
        fs::read_to_string(root.join("source/_posts/hello.md")).unwrap(),
        "local before sync"
    );
    assert!(!transaction.exists());
}

#[test]
fn recovery_reports_the_path_that_cannot_be_restored() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path().join("project");
    fs::create_dir_all(root.join("source/_posts")).unwrap();
    let state = AppState::new(&temp.path().join("config"));
    let key = cache_key(&path_key(&root));
    let backup = state.sync_backup_dir.join(&key).join("backup-1");
    fs::create_dir_all(backup.join("source/_posts")).unwrap();
    // The backup holds a file, but the project path is a directory now, so the
    // rollback cannot write and has to name the path it failed on.
    fs::write(backup.join("source/_posts/hello.md"), "local").unwrap();
    fs::create_dir_all(root.join("source/_posts/hello.md")).unwrap();

    let error = restore_backup(
        &root,
        &backup,
        &BTreeSet::from(["source/_posts/hello.md".to_string()]),
    )
    .unwrap_err();

    assert_eq!(error.code, "io_error");
    assert!(error.message.contains("恢复同步备份失败"));
    assert!(error.message.contains("source/_posts/hello.md"));
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
