use super::*;

#[test]
fn agent_bootstrap_without_query_returns_workspace_snapshot() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let payload = engine.agent_bootstrap(None, 20, false, 12_000, 3_000)?;

    assert!(payload.brief.index_status.files >= 1);
    assert!(payload.query_bundle.is_none());

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn agent_bootstrap_with_query_returns_hits_context_and_report() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let payload = engine.agent_bootstrap(Some("alpha_beta_gamma"), 10, true, 12_000, 3_000)?;

    let bundle = payload
        .query_bundle
        .expect("query bundle should be present");
    assert!(bundle.hits.iter().any(|hit| {
        hit.path.ends_with("src/main.rs")
            || hit.path == "src/main.rs"
            || hit.path.ends_with("src\\main.rs")
    }));
    assert!(bundle.report.confidence.overall > 0.0);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn agent_bootstrap_without_query_on_empty_read_only_project_returns_zero_snapshot()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-agent-read-only-empty");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn bootstrap_read_only_empty() {}\n",
    )?;

    let db_path = project_dir.join(".rmu/index.db");
    let engine = Engine::new_read_only(project_dir.clone(), Some(db_path.clone()))?;
    let payload = engine.agent_bootstrap_with_auto_index(None, 20, false, 12_000, 3_000, false)?;

    assert_eq!(payload.brief.index_status.files, 0);
    assert!(payload.brief.languages.is_empty());
    assert!(payload.brief.top_symbols.is_empty());
    assert!(payload.query_bundle.is_none());
    assert!(!db_path.exists());
    assert!(!project_dir.join(".rmu").exists());

    cleanup_project(&project_dir);
    Ok(())
}
