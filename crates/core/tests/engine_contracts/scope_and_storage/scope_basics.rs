use super::*;

#[test]
fn unicode_canonical_equivalence_hits_via_fallback_when_fts_misses() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-unicode-fallback");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/uni.rs"),
        "pub fn unicode_probe() { let s = \"Cafe\\u{301}\"; println!(\"{s}\"); }\n",
    )?;
    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let hits = engine.search(&QueryOptions {
        query: "\u{00C9}".to_string(),
        limit: 10,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?;

    assert!(
        hits.iter()
            .any(|hit| hit.path.ends_with("src/uni.rs") || hit.path == "src/uni.rs")
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn indexing_scope_allows_include_and_exclude_prefixes() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-scope");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("vendor"))?;
    fs::write(project_dir.join("src/main.rs"), "fn kept_symbol() {}\n")?;
    fs::write(
        project_dir.join("vendor/junk.rs"),
        "fn dropped_symbol() {}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["src".to_string()],
        exclude_paths: vec![],
        reindex: true,
    })?;

    assert_eq!(engine.index_status()?.files, 1);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn delete_index_storage_removes_database_file() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-delete-index");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn delete_index_storage_symbol() { println!(\"ok\"); }\n",
    )?;

    let db_path = project_dir.join(".rmu/index.db");
    let engine = Engine::new(project_dir.clone(), Some(db_path.clone()))?;
    let _ = engine.index_path()?;
    assert!(db_path.exists());

    let deleted = engine.delete_index_storage()?;
    assert!(deleted.removed_count >= 1);
    assert!(!db_path.exists());

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn indexing_scope_supports_glob_patterns() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-scope-glob");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("vendor"))?;
    fs::create_dir_all(project_dir.join("scripts"))?;
    fs::write(project_dir.join("src/main.rs"), "fn keep_src() {}\n")?;
    fs::write(project_dir.join("vendor/lib.rs"), "fn drop_vendor() {}\n")?;
    fs::write(project_dir.join("scripts/run.py"), "print('x')\n")?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["**/*.rs".to_string()],
        exclude_paths: vec!["vendor/**".to_string()],
        reindex: true,
    })?;

    assert_eq!(engine.index_status()?.files, 1);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn scope_preview_without_db_is_read_only_and_reports_scope_buckets() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-scope-preview-read-only");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("vendor"))?;
    fs::create_dir_all(project_dir.join("target"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn kept_preview_symbol() {}\n",
    )?;
    fs::write(
        project_dir.join("vendor/skip.rs"),
        "fn skipped_preview_symbol() {}\n",
    )?;
    fs::write(
        project_dir.join("target/generated.rs"),
        "fn ignored_preview_symbol() {}\n",
    )?;

    let engine = Engine::new_read_only_with_migration_mode(
        project_dir.clone(),
        Some(project_dir.join(".rmu/index.db")),
        MigrationMode::Auto,
    )?;
    let preview = engine.scope_preview_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["**/*.rs".to_string()],
        exclude_paths: vec!["vendor/**".to_string()],
        reindex: false,
    })?;

    assert_eq!(preview.candidate_paths, vec!["src/main.rs"]);
    assert_eq!(preview.excluded_by_scope_paths, vec!["vendor/skip.rs"]);
    assert_eq!(preview.ignored_paths, vec!["target/generated.rs"]);
    assert!(!project_dir.join(".rmu").exists());

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn scoped_incremental_index_prunes_out_of_scope_entries() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-scope-incremental");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("vendor"))?;
    fs::write(project_dir.join("src/main.rs"), "fn keep_src() {}\n")?;
    fs::write(project_dir.join("vendor/lib.rs"), "fn keep_vendor() {}\n")?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path()?;
    assert_eq!(engine.index_status()?.files, 2);

    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["src".to_string()],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert_eq!(engine.index_status()?.files, 1);

    cleanup_project(&project_dir);
    Ok(())
}
