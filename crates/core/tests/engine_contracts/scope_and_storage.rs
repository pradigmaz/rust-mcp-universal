use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration as StdDuration;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use rmu_core::{
    Engine, IndexProfile, IndexingOptions, MigrationMode, PrivacyMode, QueryOptions,
    SemanticFailMode,
};
use rusqlite::Connection;
use time::{Duration, OffsetDateTime};

use crate::common::{cleanup_project, temp_project_dir};

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

    // Single-char query intentionally skips FTS token path and exercises search_like fallback.
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

    let status = engine.index_status()?;
    assert_eq!(status.files, 1);

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

    let status = engine.index_status()?;
    assert_eq!(status.files, 1);

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

    let status_before = engine.index_status()?;
    assert_eq!(status_before.files, 2);

    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["src".to_string()],
        exclude_paths: vec![],
        reindex: false,
    })?;

    let status_after = engine.index_status()?;
    assert_eq!(status_after.files, 1);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn rust_monorepo_profile_indexes_workspace_scope_without_docs() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-rust-monorepo-profile");
    fs::create_dir_all(project_dir.join("crates/core/src"))?;
    fs::create_dir_all(project_dir.join("docs"))?;
    fs::write(
        project_dir.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/core\"]\n",
    )?;
    fs::write(
        project_dir.join("crates/core/src/lib.rs"),
        "pub fn workspace_symbol() {}\n",
    )?;
    fs::write(
        project_dir.join("docs/guide.md"),
        "profile_docs_only_marker\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: Some(IndexProfile::RustMonorepo),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let status = engine.index_status()?;
    assert_eq!(status.files, 2);

    let docs_hits = engine.search(&QueryOptions {
        query: "profile_docs_only_marker".to_string(),
        limit: 5,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?;
    assert!(docs_hits.is_empty());

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn mixed_profile_excludes_generated_directories() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-mixed-profile");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("docs"))?;
    fs::create_dir_all(project_dir.join(".next/cache"))?;
    fs::create_dir_all(project_dir.join("dist"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn mixed_profile_kept() {}\n",
    )?;
    fs::write(
        project_dir.join("docs/guide.md"),
        "mixed_profile_docs_kept\n",
    )?;
    fs::write(
        project_dir.join(".next/cache/build.js"),
        "mixed_profile_generated\n",
    )?;
    fs::write(project_dir.join("dist/bundle.js"), "mixed_profile_bundle\n")?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: Some(IndexProfile::Mixed),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let status = engine.index_status()?;
    assert_eq!(status.files, 1);

    let docs_hits = engine.search(&QueryOptions {
        query: "mixed_profile_docs_kept".to_string(),
        limit: 5,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?;
    assert!(docs_hits.is_empty());

    let generated_hits = engine.search(&QueryOptions {
        query: "mixed_profile_generated".to_string(),
        limit: 5,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?;
    assert!(generated_hits.is_empty());

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn docs_heavy_profile_indexes_docs_without_code_roots() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-docs-heavy-profile");
    fs::create_dir_all(project_dir.join("docs"))?;
    fs::create_dir_all(project_dir.join("schemas"))?;
    fs::create_dir_all(project_dir.join("config"))?;
    fs::create_dir_all(project_dir.join("crates/core/src"))?;
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(project_dir.join("README.md"), "docs_heavy_readme_marker\n")?;
    fs::write(project_dir.join("docs/guide.md"), "docs_heavy_doc_marker\n")?;
    fs::write(
        project_dir.join("schemas/api.json"),
        "{\"marker\":\"docs_heavy_schema_marker\"}\n",
    )?;
    fs::write(
        project_dir.join("config/app.toml"),
        "name = \"docs_heavy_config_marker\"\n",
    )?;
    fs::write(
        project_dir.join("crates/core/src/lib.rs"),
        "pub fn docs_heavy_code_marker() {}\n",
    )?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn docs_heavy_root_code_marker() {}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: Some(IndexProfile::DocsHeavy),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let status = engine.index_status()?;
    assert_eq!(status.files, 4);

    let code_hits = engine.search(&QueryOptions {
        query: "docs_heavy_code_marker".to_string(),
        limit: 5,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?;
    assert!(code_hits.is_empty());

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn scoped_incremental_prune_uses_effective_profile_scope() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-profile-prune");
    fs::create_dir_all(project_dir.join("crates/core/src"))?;
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("crates/core/src/lib.rs"),
        "pub fn keep_crate() {}\n",
    )?;
    fs::write(project_dir.join("src/main.rs"), "fn prune_me() {}\n")?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path()?;
    assert_eq!(engine.index_status()?.files, 2);

    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: Some(IndexProfile::RustMonorepo),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec!["crates".to_string()],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert_eq!(engine.index_status()?.files, 1);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn delete_index_storage_removes_sidecars_for_custom_db_extensions() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-delete-index-sidecars");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn delete_index_storage_sidecar_symbol() { println!(\"ok\"); }\n",
    )?;

    let db_path = project_dir.join(".rmu/index.sqlite");
    let engine = Engine::new(project_dir.clone(), Some(db_path.clone()))?;
    let _ = engine.index_path()?;
    let wal_path = PathBuf::from(format!("{}-wal", db_path.display()));
    let shm_path = PathBuf::from(format!("{}-shm", db_path.display()));
    fs::write(&wal_path, b"wal")?;
    fs::write(&shm_path, b"shm")?;

    let deleted = engine.delete_index_storage()?;
    assert!(deleted.removed_count >= 3);
    assert!(!db_path.exists());
    assert!(!wal_path.exists());
    assert!(!shm_path.exists());

    cleanup_project(&project_dir);
    Ok(())
}

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

#[cfg(unix)]
#[test]
fn unreadable_file_preserves_previous_snapshot_rows() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-unreadable-file");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/private.rs"),
        "fn preserved_unreadable_file_symbol() {}\n",
    )?;

    let db_path = project_dir.join(".rmu/index.db");
    let engine = Engine::new(project_dir.clone(), Some(db_path.clone()))?;
    engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let private_path = project_dir.join("src/private.rs");
    let original_permissions = fs::metadata(&private_path)?.permissions();
    let mut unreadable_permissions = original_permissions.clone();
    unreadable_permissions.set_mode(0o000);
    fs::set_permissions(&private_path, unreadable_permissions)?;

    let result = (|| -> Result<(), Box<dyn Error>> {
        let summary = engine.index_path_with_options(&IndexingOptions {
            profile: None,
            changed_since: None,
            changed_since_commit: None,
            include_paths: vec![],
            exclude_paths: vec![],
            reindex: false,
        })?;

        assert_eq!(summary.deleted, 0);
        assert_eq!(engine.index_status()?.files, 1);
        assert_eq!(file_row_count(&db_path, "src/private.rs")?, 1);
        assert_eq!(
            search_hit_count(&engine, "preserved_unreadable_file_symbol")?,
            1
        );
        Ok(())
    })();

    fs::set_permissions(&private_path, original_permissions)?;
    cleanup_project(&project_dir);
    result
}

#[cfg(unix)]
#[test]
fn unreadable_walk_subtree_preserves_previous_snapshot_rows() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-unreadable-subtree");
    fs::create_dir_all(project_dir.join("src/private"))?;
    fs::write(
        project_dir.join("src/private/secret.rs"),
        "fn preserved_unreadable_subtree_symbol() {}\n",
    )?;

    let db_path = project_dir.join(".rmu/index.db");
    let engine = Engine::new(project_dir.clone(), Some(db_path.clone()))?;
    engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let private_dir = project_dir.join("src/private");
    let original_permissions = fs::metadata(&private_dir)?.permissions();
    let mut unreadable_permissions = original_permissions.clone();
    unreadable_permissions.set_mode(0o000);
    fs::set_permissions(&private_dir, unreadable_permissions)?;

    let result = (|| -> Result<(), Box<dyn Error>> {
        let summary = engine.index_path_with_options(&IndexingOptions {
            profile: None,
            changed_since: None,
            changed_since_commit: None,
            include_paths: vec![],
            exclude_paths: vec![],
            reindex: false,
        })?;

        assert_eq!(summary.deleted, 0);
        assert_eq!(engine.index_status()?.files, 1);
        assert_eq!(file_row_count(&db_path, "src/private/secret.rs")?, 1);
        assert_eq!(
            search_hit_count(&engine, "preserved_unreadable_subtree_symbol")?,
            1
        );
        Ok(())
    })();

    fs::set_permissions(&private_dir, original_permissions)?;
    cleanup_project(&project_dir);
    result
}

#[test]
fn changed_since_commit_reindexes_git_delta_and_reports_merge_base() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-changed-since-commit");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/stable.rs"),
        "fn stable_commit_symbol() {}\n",
    )?;
    fs::write(
        project_dir.join("src/fresh.rs"),
        "fn fresh_commit_symbol() {}\n",
    )?;

    run_git(&project_dir, &["init"])?;
    run_git(&project_dir, &["config", "user.email", "codex@example.com"])?;
    run_git(&project_dir, &["config", "user.name", "Codex"])?;
    run_git(&project_dir, &["add", "."])?;
    run_git(&project_dir, &["commit", "-m", "initial"])?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    fs::write(
        project_dir.join("src/fresh.rs"),
        "fn fresh_commit_symbol() { println!(\"updated\"); }\n",
    )?;

    let summary = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: Some("HEAD".to_string()),
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert_eq!(summary.indexed, 1);
    assert_eq!(summary.changed, 1);
    assert_eq!(summary.changed_since_commit.as_deref(), Some("HEAD"));
    assert!(summary.resolved_merge_base_commit.is_some());
    assert_eq!(engine.index_status()?.files, 2);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn scope_preview_changed_since_commit_reports_candidates_and_deletes() -> Result<(), Box<dyn Error>>
{
    let project_dir = temp_project_dir("rmu-core-tests-scope-preview-commit");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/stable.rs"),
        "fn stable_preview_commit_symbol() {}\n",
    )?;
    fs::write(
        project_dir.join("src/fresh.rs"),
        "fn fresh_preview_commit_symbol() {}\n",
    )?;
    fs::write(
        project_dir.join("src/drop.rs"),
        "fn drop_preview_commit_symbol() {}\n",
    )?;

    run_git(&project_dir, &["init"])?;
    run_git(&project_dir, &["config", "user.email", "codex@example.com"])?;
    run_git(&project_dir, &["config", "user.name", "Codex"])?;
    run_git(&project_dir, &["add", "."])?;
    run_git(&project_dir, &["commit", "-m", "initial"])?;

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

    fs::write(
        project_dir.join("src/fresh.rs"),
        "fn fresh_preview_commit_symbol() { println!(\"updated\"); }\n",
    )?;
    fs::remove_file(project_dir.join("src/drop.rs"))?;

    let preview_engine = Engine::new_read_only_with_migration_mode(
        project_dir.clone(),
        Some(db_path),
        MigrationMode::Auto,
    )?;
    let preview = preview_engine.scope_preview_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: Some("HEAD".to_string()),
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert_eq!(preview.changed_since_commit.as_deref(), Some("HEAD"));
    assert!(preview.resolved_merge_base_commit.is_some());
    assert_eq!(preview.candidate_paths, vec!["src/fresh.rs"]);
    assert_eq!(preview.deleted_paths, vec!["src/drop.rs"]);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn changed_since_commit_prunes_git_deleted_paths() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-changed-since-commit-delete");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/keep.rs"),
        "fn keep_commit_symbol() {}\n",
    )?;
    fs::write(
        project_dir.join("src/drop.rs"),
        "fn drop_commit_symbol() {}\n",
    )?;

    run_git(&project_dir, &["init"])?;
    run_git(&project_dir, &["config", "user.email", "codex@example.com"])?;
    run_git(&project_dir, &["config", "user.name", "Codex"])?;
    run_git(&project_dir, &["add", "."])?;
    run_git(&project_dir, &["commit", "-m", "initial"])?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    fs::remove_file(project_dir.join("src/drop.rs"))?;

    let summary = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: Some("HEAD".to_string()),
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert_eq!(summary.deleted, 1);
    assert_eq!(engine.index_status()?.files, 1);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn changed_since_commit_indexes_untracked_files_for_nested_project_roots()
-> Result<(), Box<dyn Error>> {
    let repo_dir = temp_project_dir("rmu-core-tests-changed-since-commit-nested");
    let project_dir = repo_dir.join("apps/demo");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn nested_commit_symbol() {}\n",
    )?;

    run_git(&repo_dir, &["init"])?;
    run_git(&repo_dir, &["config", "user.email", "codex@example.com"])?;
    run_git(&repo_dir, &["config", "user.name", "Codex"])?;
    run_git(&repo_dir, &["add", "."])?;
    run_git(&repo_dir, &["commit", "-m", "initial"])?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    fs::write(
        project_dir.join("src/untracked.rs"),
        "pub fn nested_untracked_symbol() {}\n",
    )?;

    let summary = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: Some("HEAD".to_string()),
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert_eq!(summary.indexed, 1);
    assert_eq!(engine.index_status()?.files, 2);

    cleanup_project(&repo_dir);
    Ok(())
}

#[cfg(windows)]
#[test]
fn call_path_accepts_absolute_windows_paths_with_case_only_differences()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-call-path-windows-case");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn windows_case_symbol() {}\n",
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

    let absolute_path = project_dir
        .join("src/lib.rs")
        .to_string_lossy()
        .to_uppercase();
    let result = engine.call_path(&absolute_path, &absolute_path, 3)?;

    assert!(result.found);
    assert_eq!(result.hops, 0);
    assert_eq!(result.path, vec!["src/lib.rs"]);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn changed_since_run_repairs_corrupted_file_graph_edges_without_forced_full_reindex()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-graph-edge-repair");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        r#"
mod worker;

pub fn anchor_entry() {
    let note = "graph_repair_anchor";
    worker::render_worker();
    println!("{note}");
}
"#,
    )?;
    fs::write(
        project_dir.join("src/worker.rs"),
        r#"
pub fn render_worker() {
    println!("worker implementation only");
}
"#,
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

    let original_edge_count = file_graph_edge_count(&db_path)?;
    assert!(original_edge_count > 0, "expected indexed graph edges");

    {
        let conn = Connection::open(&db_path)?;
        conn.execute("DELETE FROM file_graph_edges", [])?;
        conn.execute(
            "UPDATE files
             SET graph_edge_out_count = NULL,
                 graph_edge_in_count = NULL,
                 graph_edge_hash = NULL,
                 graph_edge_fingerprint_version = NULL
             WHERE path IN (?1, ?2)",
            ["src/main.rs", "src/worker.rs"],
        )?;
    }

    let summary = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: Some(OffsetDateTime::now_utc() + Duration::hours(1)),
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert!(
        summary.indexed >= 1,
        "expected explicit incremental index run to repair corrupted graph metadata"
    );
    assert_eq!(file_graph_edge_count(&db_path)?, original_edge_count);
    assert!(graph_edge_metadata_present(&db_path, "src/main.rs")?);
    assert!(graph_edge_metadata_present(&db_path, "src/worker.rs")?);

    cleanup_project(&project_dir);
    Ok(())
}

fn source_mtime_for_path(db_path: &PathBuf, path: &str) -> Result<Option<i64>, Box<dyn Error>> {
    let conn = Connection::open(db_path)?;
    let value = conn.query_row(
        "SELECT source_mtime_unix_ms FROM files WHERE path = ?1",
        [path],
        |row| row.get::<_, Option<i64>>(0),
    )?;
    Ok(value)
}

fn run_git(project_dir: &std::path::Path, args: &[&str]) -> Result<(), Box<dyn Error>> {
    let status = Command::new("git")
        .current_dir(project_dir)
        .args(args)
        .status()?;
    assert!(status.success(), "git {:?} failed", args);
    Ok(())
}

fn file_graph_edge_count(db_path: &PathBuf) -> Result<i64, Box<dyn Error>> {
    let conn = Connection::open(db_path)?;
    let count = conn.query_row("SELECT COUNT(*) FROM file_graph_edges", [], |row| {
        row.get(0)
    })?;
    Ok(count)
}

#[cfg(unix)]
fn file_row_count(db_path: &PathBuf, path: &str) -> Result<i64, Box<dyn Error>> {
    let conn = Connection::open(db_path)?;
    let count = conn.query_row(
        "SELECT COUNT(1) FROM files WHERE path = ?1",
        [path],
        |row| row.get(0),
    )?;
    Ok(count)
}

#[cfg(unix)]
fn search_hit_count(engine: &Engine, query: &str) -> Result<usize, Box<dyn Error>> {
    let hits = engine.search(&QueryOptions {
        query: query.to_string(),
        limit: 10,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
    })?;
    Ok(hits.len())
}

fn graph_edge_metadata_present(db_path: &PathBuf, path: &str) -> Result<bool, Box<dyn Error>> {
    let conn = Connection::open(db_path)?;
    let metadata = conn.query_row(
        "SELECT graph_edge_out_count, graph_edge_in_count, graph_edge_hash, graph_edge_fingerprint_version
         FROM files
         WHERE path = ?1",
        [path],
        |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?,
                row.get::<_, Option<i64>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<i64>>(3)?,
            ))
        },
    )?;
    Ok(
        metadata.0.is_some()
            && metadata.1.is_some()
            && metadata.2.is_some()
            && metadata.3.is_some(),
    )
}
