#[test]
fn semantic_fail_open_degrades_to_lexical_on_corrupted_vector_payload() -> Result<(), Box<dyn Error>>
{
    let (project_dir, engine) = setup_indexed_project()?;
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "UPDATE semantic_vectors
         SET vector_json = '[1,2,3]'
         WHERE rowid = (SELECT rowid FROM semantic_vectors LIMIT 1)",
        [],
    )?;

    let query = QueryOptions {
        query: "alpha_beta_gamma".to_string(),
        limit: 10,
        detailed: false,
        semantic: true,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
        agent_intent_mode: None,
    };

    let hits = engine.search(&query)?;
    assert!(!hits.is_empty(), "fail_open must keep lexical shortlist");
    let report = engine.build_report(&query, 20_000, 6_000)?;
    assert_eq!(report.confidence.signals.semantic_outcome, "failed");

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn semantic_fail_open_uses_healthy_vectors_when_one_row_is_corrupted() -> Result<(), Box<dyn Error>>
{
    let project_dir = temp_project_dir("rmu-core-tests-semantic-partial-corruption");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        r#"
fn alpha_beta_gamma() {
    println!("alpha beta gamma");
}
"#,
    )?;
    fs::write(
        project_dir.join("src/helper.rs"),
        r#"
fn helper_noise_signal() {
    println!("omega sigma tau");
}
"#,
    )?;
    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "UPDATE semantic_vectors
         SET vector_json = '[1,2,3]'
         WHERE path = 'src/helper.rs'",
        [],
    )?;

    let query = QueryOptions {
        query: "alpha_beta_gamma".to_string(),
        limit: 10,
        detailed: false,
        semantic: true,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
        agent_intent_mode: None,
    };

    let hits = engine.search(&query)?;
    assert!(!hits.is_empty(), "healthy semantic rows must still participate");
    let report = engine.build_report(&query, 20_000, 6_000)?;
    assert_ne!(report.confidence.signals.semantic_outcome, "failed");
    assert!(report.confidence.signals.semantic_outcome.starts_with("applied_"));

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn semantic_fail_closed_returns_error_on_corrupted_vector_payload() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "UPDATE semantic_vectors
         SET vector_json = '[1,2,3]'
         WHERE rowid = (SELECT rowid FROM semantic_vectors LIMIT 1)",
        [],
    )?;

    let query = QueryOptions {
        query: "alpha_beta_gamma".to_string(),
        limit: 10,
        detailed: false,
        semantic: true,
        semantic_fail_mode: SemanticFailMode::FailClosed,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
        agent_intent_mode: None,
    };

    let err = engine
        .search(&query)
        .expect_err("fail_closed must propagate semantic errors");
    assert!(err.to_string().contains("invalid semantic vector"));

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn reindex_repairs_corrupted_chunk_embedding_cache_entry() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "UPDATE chunk_embeddings
         SET vector_json = 'not-json'
         WHERE rowid = (SELECT rowid FROM chunk_embeddings LIMIT 1)",
        [],
    )?;
    drop(conn);

    engine.index_path()?;

    let conn = Connection::open(&db_path)?;
    let (dim, raw): (i64, String) = conn.query_row(
        "SELECT dim, vector_json FROM chunk_embeddings LIMIT 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let vector = serde_json::from_str::<Vec<f32>>(&raw)?;
    assert_eq!(vector.len(), usize::try_from(dim).unwrap_or(0));
    assert_eq!(vector.len(), 192);

    cleanup_project(&project_dir);
    Ok(())
}
