use super::*;

#[test]
fn changed_since_only_reindexes_recent_files_and_reports_skips() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-changed-since");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(project_dir.join("src/stable.rs"), "fn stable_symbol() {}\n")?;
    fs::write(project_dir.join("src/fresh.rs"), "fn fresh_symbol() {}\n")?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let cutoff = OffsetDateTime::now_utc();
    sleep(StdDuration::from_millis(25));
    fs::write(
        project_dir.join("src/fresh.rs"),
        "fn fresh_symbol() { println!(\"updated\"); }\n",
    )?;

    let summary = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: Some(cutoff),
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert_eq!(summary.indexed, 1);
    assert_eq!(summary.changed, 1);
    assert_eq!(summary.skipped_before_changed_since, 1);
    assert_eq!(engine.index_status()?.files, 2);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn scope_preview_changed_since_reports_candidates_skips_and_repairs() -> Result<(), Box<dyn Error>>
{
    let project_dir = temp_project_dir("rmu-core-tests-scope-preview-changed-since");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/stable.rs"),
        "fn stable_preview_symbol() {}\n",
    )?;
    fs::write(
        project_dir.join("src/fresh.rs"),
        "fn fresh_preview_symbol() {}\n",
    )?;
    fs::write(
        project_dir.join("src/repair.rs"),
        "fn repair_preview_symbol() {}\n",
    )?;

    let db_path = project_dir.join(".rmu/index.db");
    let engine = Engine::new(project_dir.clone(), Some(db_path.clone()))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let cutoff = OffsetDateTime::now_utc();
    sleep(StdDuration::from_millis(25));
    fs::write(
        project_dir.join("src/fresh.rs"),
        "fn fresh_preview_symbol() { println!(\"updated\"); }\n",
    )?;
    {
        let conn = Connection::open(&db_path)?;
        conn.execute("DELETE FROM file_chunks WHERE path = ?1", ["src/repair.rs"])?;
    }

    let preview_engine = Engine::new_read_only_with_migration_mode(
        project_dir.clone(),
        Some(db_path),
        MigrationMode::Auto,
    )?;
    let preview = preview_engine.scope_preview_with_options(&IndexingOptions {
        profile: None,
        changed_since: Some(cutoff),
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert!(
        preview
            .candidate_paths
            .contains(&"src/fresh.rs".to_string())
    );
    assert!(
        preview
            .candidate_paths
            .contains(&"src/repair.rs".to_string())
    );
    assert_eq!(
        preview.skipped_before_changed_since_paths,
        vec!["src/stable.rs"]
    );
    assert_eq!(preview.repair_backfill_paths, vec!["src/repair.rs"]);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn changed_since_updates_source_mtime_without_rebuilding_same_hash() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-changed-since-same-hash");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/same.rs"),
        "fn same_hash_symbol() {}\n",
    )?;

    let db_path = project_dir.join(".rmu/index.db");
    let engine = Engine::new(project_dir.clone(), Some(db_path.clone()))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let before_mtime = source_mtime_for_path(&db_path, "src/same.rs")?.expect("mtime after index");
    let cutoff = OffsetDateTime::now_utc();
    sleep(StdDuration::from_millis(25));
    fs::write(
        project_dir.join("src/same.rs"),
        "fn same_hash_symbol() {}\n",
    )?;

    let summary = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: Some(cutoff),
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    let after_mtime = source_mtime_for_path(&db_path, "src/same.rs")?.expect("mtime after refresh");
    assert_eq!(summary.indexed, 0);
    assert_eq!(summary.unchanged, 1);
    assert_eq!(summary.changed, 0);
    assert!(after_mtime > before_mtime);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn changed_since_repairs_incomplete_chunk_state_even_before_cutoff() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-changed-since-repair");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(project_dir.join("src/repair.rs"), "fn repair_symbol() {}\n")?;

    let db_path = project_dir.join(".rmu/index.db");
    let engine = Engine::new(project_dir.clone(), Some(db_path.clone()))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    {
        let conn = Connection::open(&db_path)?;
        conn.execute("DELETE FROM file_chunks WHERE path = ?1", ["src/repair.rs"])?;
    }

    let summary = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: Some(OffsetDateTime::now_utc() + Duration::hours(1)),
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert_eq!(summary.indexed, 1);
    assert_eq!(summary.skipped_before_changed_since, 0);
    assert!(engine.index_status()?.file_chunks >= 1);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn changed_since_force_reindex_rebuilds_candidate_set_only() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-changed-since-force-reindex");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(project_dir.join("src/old.rs"), "fn old_symbol() {}\n")?;
    fs::write(
        project_dir.join("src/rebuild.rs"),
        "fn rebuild_symbol() {}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let cutoff = OffsetDateTime::now_utc();
    sleep(StdDuration::from_millis(25));
    fs::write(
        project_dir.join("src/rebuild.rs"),
        "fn rebuild_symbol() {}\n",
    )?;

    let summary = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: Some(cutoff),
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    assert_eq!(summary.indexed, 1);
    assert_eq!(summary.changed, 1);
    assert_eq!(summary.unchanged, 0);
    assert_eq!(summary.skipped_before_changed_since, 1);
    assert_eq!(engine.index_status()?.files, 2);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn changed_since_run_still_prunes_missing_paths() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-changed-since-prune");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(project_dir.join("src/prune.rs"), "fn prune_symbol() {}\n")?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let cutoff = OffsetDateTime::now_utc();
    fs::remove_file(project_dir.join("src/prune.rs"))?;

    let summary = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: Some(cutoff),
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert_eq!(summary.deleted, 1);
    assert_eq!(engine.index_status()?.files, 0);

    cleanup_project(&project_dir);
    Ok(())
}
