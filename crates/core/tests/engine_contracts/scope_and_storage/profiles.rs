use super::*;

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

    assert_eq!(engine.index_status()?.files, 2);
    assert!(
        engine
            .search(&QueryOptions {
                query: "profile_docs_only_marker".to_string(),
                limit: 5,
                detailed: false,
                semantic: false,
                semantic_fail_mode: SemanticFailMode::FailOpen,
                privacy_mode: PrivacyMode::Off,
                context_mode: None,
            })?
            .is_empty()
    );

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

    assert_eq!(engine.index_status()?.files, 1);
    for query in ["mixed_profile_docs_kept", "mixed_profile_generated"] {
        assert!(
            engine
                .search(&QueryOptions {
                    query: query.to_string(),
                    limit: 5,
                    detailed: false,
                    semantic: false,
                    semantic_fail_mode: SemanticFailMode::FailOpen,
                    privacy_mode: PrivacyMode::Off,
                    context_mode: None,
                })?
                .is_empty()
        );
    }

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

    assert_eq!(engine.index_status()?.files, 4);
    assert!(
        engine
            .search(&QueryOptions {
                query: "docs_heavy_code_marker".to_string(),
                limit: 5,
                detailed: false,
                semantic: false,
                semantic_fail_mode: SemanticFailMode::FailOpen,
                privacy_mode: PrivacyMode::Off,
                context_mode: None,
            })?
            .is_empty()
    );

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
