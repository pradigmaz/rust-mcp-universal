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
    };

    let hits = engine.search(&query)?;
    assert!(!hits.is_empty(), "fail_open must keep lexical shortlist");
    let report = engine.build_report(&query, 20_000, 6_000)?;
    assert_eq!(report.confidence.signals.semantic_outcome, "failed");

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
    };

    let err = engine
        .search(&query)
        .expect_err("fail_closed must propagate semantic errors");
    assert!(err.to_string().contains("invalid semantic vector"));

    cleanup_project(&project_dir);
    Ok(())
}
