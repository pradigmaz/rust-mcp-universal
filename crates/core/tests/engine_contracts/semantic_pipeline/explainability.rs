#[test]
fn report_reflects_fallback_semantic_outcome_and_sources() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;
    conn.execute("DELETE FROM semantic_ann_buckets", [])?;
    conn.execute("DELETE FROM chunk_embeddings", [])?;

    let report = engine.build_report(
        &QueryOptions {
            query: "alpha_beta_gamma".to_string(),
            limit: 10,
            detailed: true,
            semantic: true,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        },
        20_000,
        6_000,
    )?;

    assert_eq!(
        report.confidence.signals.semantic_outcome,
        "applied_fallback"
    );
    assert!(
        report
            .selected_context
            .iter()
            .any(|item| item.explain.semantic_source == "fallback")
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn explain_breakdown_is_reproducible_for_same_query_inputs() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let options = QueryOptions {
        query: "alpha_beta_gamma".to_string(),
        limit: 10,
        detailed: true,
        semantic: true,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    };

    let first = engine.build_report(&options, 20_000, 6_000)?;
    let second = engine.build_report(&options, 20_000, 6_000)?;

    assert_eq!(first.selected_context.len(), second.selected_context.len());
    for (a, b) in first
        .selected_context
        .iter()
        .zip(second.selected_context.iter())
    {
        assert_eq!(a.path, b.path);
        assert_eq!(a.explain.rank_before, b.explain.rank_before);
        assert_eq!(a.explain.rank_after, b.explain.rank_after);
        assert_eq!(a.explain.semantic_source, b.explain.semantic_source);
        assert_eq!(a.explain.semantic_outcome, b.explain.semantic_outcome);
        assert!((a.explain.lexical - b.explain.lexical).abs() < 1e-6);
        assert!((a.explain.graph - b.explain.graph).abs() < 1e-6);
        assert!((a.explain.semantic - b.explain.semantic).abs() < 1e-6);
        assert!((a.explain.rrf - b.explain.rrf).abs() < 1e-6);
    }

    assert!((first.confidence.signals.explain_coverage - 1.0).abs() < 1e-6);
    assert!((second.confidence.signals.explain_coverage - 1.0).abs() < 1e-6);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn report_falls_back_when_chunk_embedding_payload_is_corrupted() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "UPDATE chunk_embeddings
         SET vector_json = '[1,2,3]'
         WHERE rowid = (SELECT rowid FROM chunk_embeddings LIMIT 1)",
        [],
    )?;

    let report = engine.build_report(
        &QueryOptions {
            query: "alpha_beta_gamma".to_string(),
            limit: 10,
            detailed: true,
            semantic: true,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        },
        20_000,
        6_000,
    )?;

    assert!(
        report
            .selected_context
            .iter()
            .any(|item| item.chunk_source == "chunk_embedding_fallback")
    );

    cleanup_project(&project_dir);
    Ok(())
}
