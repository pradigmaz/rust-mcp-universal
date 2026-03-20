#[test]
fn semantic_search_unions_lexical_and_semantic_candidates() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-dual-candidates");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/a.rs"),
        "pub fn anchor_keyword_symbol() -> i32 { 1 }\n",
    )?;
    fs::write(
        project_dir.join("src/b.rs"),
        "pub fn unrelated_file() -> i32 { 2 }\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;
    let status = engine.index_status()?;
    let model = status.semantic_model;
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "UPDATE semantic_vectors
         SET vector_json = (
             SELECT vector_json FROM semantic_vectors WHERE path = 'src/a.rs' AND model = ?1
         )
         WHERE path = 'src/b.rs' AND model = ?1",
        [model],
    )?;

    let hits = engine.search(&QueryOptions {
        query: "anchor_keyword_symbol".to_string(),
        limit: 10,
        detailed: false,
        semantic: true,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?;
    assert!(
        hits.iter()
            .any(|hit| hit.path.ends_with("src/b.rs") || hit.path == "src/b.rs")
    );

    let report = engine.build_report(
        &QueryOptions {
            query: "anchor_keyword_symbol".to_string(),
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
    let lexical_stage = report
        .retrieval_pipeline
        .iter()
        .find(|stage| stage.stage == "lexical_fts_or_like")
        .expect("lexical stage must exist");
    let semantic_pool = report
        .retrieval_pipeline
        .iter()
        .find(|stage| stage.stage == "semantic_candidate_pool(local_dense_index)")
        .expect("semantic pool stage must exist");
    let fusion_stage = report
        .retrieval_pipeline
        .iter()
        .find(|stage| stage.stage == "candidate_fusion(lexical+semantic_union)")
        .expect("fusion stage must exist");
    assert!(semantic_pool.candidates >= 1);
    assert!(fusion_stage.kept >= lexical_stage.kept);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn semantic_query_prefers_source_implementation_over_planning_notes_and_tests(
) -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-semantic-path-role-prior");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("tests"))?;
    fs::create_dir_all(project_dir.join(".codex-planning"))?;

    let phrase = "Rust symbol references extracted and linked to line/column positions in this project";
    fs::write(
        project_dir.join(".codex-planning/findings.md"),
        format!("# Findings\n{phrase}.\n{phrase}.\n"),
    )?;
    fs::write(
        project_dir.join("tests/navigation.rs"),
        format!(
            "// {phrase}.\n#[test]\nfn navigation_probe() {{ let _ = \"{phrase}\"; }}\n"
        ),
    )?;
    fs::write(
        project_dir.join("src/graph.rs"),
        format!(
            "pub fn extract_rust_symbol_refs() {{\n    let _ = \"{phrase}\";\n}}\n"
        ),
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let report = engine.build_report(
        &QueryOptions {
            query: "How are Rust symbol references extracted and linked to line/column positions in this project?".to_string(),
            limit: 5,
            detailed: true,
            semantic: true,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        },
        20_000,
        6_000,
    )?;

    assert_ne!(report.confidence.signals.semantic_outcome, "short_circuit_lexical");
    let first = report
        .selected_context
        .first()
        .expect("expected at least one selected result");
    assert!(first.path == "src/graph.rs" || first.path.ends_with("src/graph.rs"));

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn explicit_semantic_query_runs_semantic_stage_even_with_many_lexical_hits(
) -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-semantic-explicit-long-query");
    fs::create_dir_all(project_dir.join("src"))?;
    for file_name in ["alpha.rs", "beta.rs", "gamma.rs", "delta.rs"] {
        fs::write(
            project_dir.join("src").join(file_name),
            "pub fn explain_symbols() {\n    let _ = \"rust symbol references extracted linked line column positions\";\n}\n",
        )?;
    }

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let report = engine.build_report(
        &QueryOptions {
            query: "How are Rust symbol references extracted and linked to line and column positions in this project?".to_string(),
            limit: 5,
            detailed: true,
            semantic: true,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        },
        20_000,
        6_000,
    )?;

    let semantic_pool = report
        .retrieval_pipeline
        .iter()
        .find(|stage| stage.stage == "semantic_candidate_pool(local_dense_index)")
        .expect("semantic pool stage must exist");
    assert!(semantic_pool.candidates > 0);
    assert_ne!(
        report.confidence.signals.semantic_outcome,
        "short_circuit_lexical"
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn model_drift_forces_reindex_even_when_file_hash_is_same() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;
    conn.execute(
        "UPDATE semantic_vectors SET model = 'stale-model-marker'",
        [],
    )?;

    let summary = engine.index_path()?;
    assert!(summary.changed >= 1 || summary.indexed >= 1);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn indexing_populates_semantic_ann_buckets() -> Result<(), Box<dyn Error>> {
    let (project_dir, _engine) = setup_indexed_project()?;
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;

    let buckets: i64 = conn.query_row("SELECT COUNT(1) FROM semantic_ann_buckets", [], |row| {
        row.get(0)
    })?;
    assert!(buckets >= 1);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn semantic_candidate_retrieval_falls_back_when_ann_table_empty() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let db_path = project_dir.join(".rmu/index.db");
    let conn = Connection::open(&db_path)?;
    conn.execute("DELETE FROM semantic_ann_buckets", [])?;

    let hits = engine.search(&QueryOptions {
        query: "alpha_beta_gamma".to_string(),
        limit: 10,
        detailed: false,
        semantic: true,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?;
    assert!(
        hits.iter()
            .any(|hit| hit.path.ends_with("src/main.rs") || hit.path == "src/main.rs")
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn reindex_backfills_ann_buckets_for_unchanged_files() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let db_path = project_dir.join(".rmu/index.db");
    {
        let conn = Connection::open(&db_path)?;
        conn.execute("DELETE FROM semantic_ann_buckets", [])?;
    }

    let summary = engine.index_path()?;
    assert!(summary.changed >= 1 || summary.indexed >= 1);

    let conn = Connection::open(&db_path)?;
    let buckets: i64 = conn.query_row("SELECT COUNT(1) FROM semantic_ann_buckets", [], |row| {
        row.get(0)
    })?;
    assert!(buckets >= 1);

    cleanup_project(&project_dir);
    Ok(())
}
