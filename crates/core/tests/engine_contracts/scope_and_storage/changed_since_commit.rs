use super::*;

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
