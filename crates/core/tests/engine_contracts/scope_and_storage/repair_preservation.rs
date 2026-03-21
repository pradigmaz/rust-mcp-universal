use super::*;

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
