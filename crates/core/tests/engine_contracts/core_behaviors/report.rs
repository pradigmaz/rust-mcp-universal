use super::*;

#[test]
fn report_marks_semantic_skipped_on_low_signal_query() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;

    let report = engine.build_report(
        &QueryOptions {
            query: "a".to_string(),
            limit: 20,
            detailed: true,
            semantic: true,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        agent_intent_mode: None,
        },
        10_000,
        3_000,
    )?;

    assert!(
        report
            .retrieval_pipeline
            .iter()
            .any(|stage| stage.stage == "semantic_vector_rerank(skipped_no_signal)")
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn report_selected_context_contains_chunk_line_metadata_and_excerpt() -> Result<(), Box<dyn Error>>
{
    let project_dir = temp_project_dir("rmu-core-tests-chunk-context");
    fs::create_dir_all(project_dir.join("src"))?;
    let mut long_source = String::new();
    for idx in 0..120 {
        long_source.push_str(&format!("fn filler_{idx}() {{ let _ = {idx}; }}\n"));
    }
    for idx in 0..20 {
        long_source.push_str(&format!(
            "fn target_symbol_{idx}() {{ println!(\"needle_chunk_marker\"); }}\n"
        ));
    }
    fs::write(project_dir.join("src/main.rs"), long_source)?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;
    let report = engine.build_report(
        &QueryOptions {
            query: "needle_chunk_marker".to_string(),
            limit: 5,
            detailed: true,
            semantic: true,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        agent_intent_mode: None,
        },
        20_000,
        6_000,
    )?;

    let first = report
        .selected_context
        .first()
        .expect("expected at least one selected context item");
    assert!((report.confidence.signals.explain_coverage - 1.0).abs() < 1e-6);
    assert!(first.start_line > 0);
    assert!(first.end_line >= first.start_line);
    assert!(first.chunk_idx < 1000);

    let payload = engine.build_context_under_budget(
        &QueryOptions {
            query: "needle_chunk_marker".to_string(),
            limit: 5,
            detailed: false,
            semantic: true,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        agent_intent_mode: None,
        },
        20_000,
        6_000,
    )?;
    assert!(
        payload
            .files
            .first()
            .is_some_and(|item| item.excerpt.contains("needle_chunk_marker"))
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn report_exposes_graph_stage_breakdown_for_connected_file() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-report-graph-stage");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        r#"
mod worker;

pub fn anchor_entry() {
    let note = "graph_report_anchor";
    worker::render_worker();
    println!("{note}");
}
"#,
    )?;
    fs::write(
        project_dir.join("src/worker.rs"),
        r#"
pub fn render_worker() {
    println!("worker implementation only");
}
"#,
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let report = engine.build_report(
        &QueryOptions {
            query: "graph_report_anchor".to_string(),
            limit: 5,
            detailed: true,
            semantic: false,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        agent_intent_mode: None,
        },
        12_000,
        3_000,
    )?;

    assert!(
        report
            .retrieval_pipeline
            .iter()
            .any(|stage| stage.stage == "graph_neighbor_pool(file_graph_edges)" && stage.kept > 0)
    );
    assert!(
        report
            .retrieval_pipeline
            .iter()
            .any(|stage| stage.stage == "candidate_refusion(lexical+semantic+graph)")
    );

    let connected = report
        .selected_context
        .iter()
        .find(|item| item.path == "src/worker.rs")
        .expect("expected connected worker file in selected context");
    assert!(connected.explain.graph > 0.0);
    assert!(connected.explain.graph_rrf > 0.0);
    assert_eq!(connected.explain.graph_seed_path, "src/main.rs");
    assert_eq!(connected.explain.graph_hops, 1);
    assert!(
        connected
            .explain
            .graph_edge_kinds
            .iter()
            .any(|kind| kind.ends_with(":ref_exact") || kind.ends_with(":ref_tail_unique"))
    );
    let investigation = report
        .investigation_summary
        .as_ref()
        .expect("embedded investigation summary should be present");
    let timings = report
        .timings
        .as_ref()
        .expect("report timings should be present");
    assert_eq!(investigation.surface_kind, "embedded_investigation_hints");
    assert!(investigation.concept_cluster.variant_count >= 1);
    assert!(!investigation.concept_cluster.top_variants.is_empty());
    assert!(investigation.route_trace.best_route_segment_count >= 1);
    assert!(timings.total_ms >= timings.search_ms);
    assert!(timings.total_ms >= timings.context_ms);
    assert!(timings.total_ms >= timings.investigation_ms);
    assert!(timings.investigation.route_ms <= timings.investigation_ms);
    assert!(timings.investigation.cluster_ms <= timings.investigation_ms);
    assert!(
        investigation.constraint_evidence.total
            >= investigation.constraint_evidence.strong + investigation.constraint_evidence.weak
    );
    if let Some(divergence) = &investigation.divergence {
        assert_eq!(divergence.surface_kind, "divergence_preview");
        assert_eq!(divergence.authoritative_tool, "divergence_report");
        assert!(divergence.preview_only);
        assert!(divergence.signal_count >= 1);
    }

    cleanup_project(&project_dir);
    Ok(())
}
