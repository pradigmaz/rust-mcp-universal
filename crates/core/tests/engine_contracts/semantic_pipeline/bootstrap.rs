#[test]
fn agent_bootstrap_auto_index_flag_controls_empty_index_behavior() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-agent-auto-index");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        "fn bootstrap_auto_index_symbol() {}\n",
    )?;
    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;

    let err = engine
        .agent_bootstrap_with_auto_index(
            Some("bootstrap_auto_index_symbol"),
            5,
            true,
            12_000,
            3_000,
            false,
        )
        .expect_err("auto_index=false must reject empty index");
    assert!(err.to_string().contains("index is empty"));

    let payload = engine.agent_bootstrap_with_auto_index(
        Some("bootstrap_auto_index_symbol"),
        5,
        true,
        12_000,
        3_000,
        true,
    )?;
    assert!(payload.brief.index_status.files >= 1);
    assert!(payload.query_bundle.is_some());

    cleanup_project(&project_dir);
    Ok(())
}
