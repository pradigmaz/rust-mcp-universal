use super::*;

#[test]
fn ensure_index_ready_auto_indexes_once() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-ready");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn auto_ready_symbol_name() { println!(\"ok\"); }\n",
    )?;
    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;

    let first = engine.ensure_index_ready()?;
    let second = engine.ensure_index_ready()?;
    let status = engine.index_status()?;

    assert!(first);
    assert!(!second);
    assert!(status.files >= 1);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn workspace_brief_contains_status_and_recommendations() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let brief = engine.workspace_brief()?;

    assert!(brief.index_status.files >= 1);
    assert!(!brief.recommendations.is_empty());
    assert!(brief.recommendations.iter().all(|item| {
        !item.contains("semantic-search")
            && !item.contains("--semantic")
            && !item.contains("semantic_index")
    }));

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn read_only_workspace_brief_requires_index_without_creating_db() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-brief-read-only");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn brief_read_only_probe() {}\n",
    )?;

    let db_path = project_dir.join(".rmu/index.db");
    let engine = Engine::new_read_only(project_dir.clone(), Some(db_path.clone()))?;
    let err = engine
        .workspace_brief_with_policy(false)
        .expect_err("read-only brief should require an index");

    assert!(err.to_string().contains("index is empty"));
    assert!(!db_path.exists());
    assert!(!project_dir.join(".rmu").exists());

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn index_status_fails_when_meta_values_are_invalid() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "UPDATE meta SET value = 'not-a-number' WHERE key = 'last_embedding_cache_hits'",
        [],
    )?;

    let err = engine
        .index_status()
        .expect_err("invalid meta value should fail index_status");
    assert!(
        err.to_string()
            .contains("meta key `last_embedding_cache_hits` contains non-u64 value")
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn read_only_index_status_returns_zero_snapshot_without_creating_db() -> Result<(), Box<dyn Error>>
{
    let project_dir = temp_project_dir("rmu-core-tests-status-read-only");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn status_read_only_probe() {}\n",
    )?;

    let db_path = project_dir.join(".rmu/index.db");
    let engine = Engine::new_read_only(project_dir.clone(), None)?;
    let status = engine.index_status()?;

    assert_eq!(status.files, 0);
    assert_eq!(status.symbols, 0);
    assert_eq!(std::path::PathBuf::from(&status.db_path), db_path);
    assert!(!db_path.exists());
    assert!(!project_dir.join(".rmu").exists());

    cleanup_project(&project_dir);
    Ok(())
}
