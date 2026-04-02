use super::{QueryReportBuildInput, RetrievalStageCounts, build_query_report};
use crate::model::{
    AgentIntentMode, ConfidenceSignals, ContextFile, ContextSelection, IndexTelemetry,
    ModeResolutionSource, PrivacyMode, RankExplainBreakdown, SearchHit,
};
use crate::report::ResultExplainEntry;
use crate::vector_rank::SemanticRerankOutcome;
use std::path::Path;

fn hit(path: &str, score: f32) -> SearchHit {
    SearchHit {
        path: path.to_string(),
        preview: "preview".to_string(),
        score,
        size_bytes: 0,
        language: "rust".to_string(),
    }
}

fn context_with_chunk_source(paths: &[&str], chunk_source: &str) -> ContextSelection {
    ContextSelection {
        files: paths
            .iter()
            .enumerate()
            .map(|(idx, path)| ContextFile {
                path: (*path).to_string(),
                excerpt: "fn probe() {}".to_string(),
                score: 1.0 - (idx as f32 * 0.1),
                chunk_idx: idx,
                start_line: 1,
                end_line: 2,
                chunk_source: chunk_source.to_string(),
            })
            .collect(),
        total_chars: 12,
        estimated_tokens: 3,
        truncated: false,
        chunk_candidates: paths.len(),
        chunk_selected: paths.len(),
    }
}

#[test]
fn confidence_penalizes_truncation_and_stage_drop() {
    let shortlist = vec![hit("src/a.rs", 1.0), hit("src/b.rs", 0.6)];
    let context = context_with_chunk_source(&["src/a.rs", "src/b.rs"], "chunk_embedding_index");
    let baseline = ConfidenceSignals {
        margin_top1_top2: 0.4,
        explain_coverage: 1.0,
        semantic_coverage: 1.0,
        semantic_outcome: "applied_indexed".to_string(),
        stage_drop_ratio: 0.0,
        hard_truncated: false,
    };
    let truncated = ConfidenceSignals {
        hard_truncated: true,
        stage_drop_ratio: 0.7,
        ..baseline.clone()
    };

    let baseline_score = super::confidence::confidence_overall(&shortlist, &context, &baseline);
    let truncated_score = super::confidence::confidence_overall(&shortlist, &context, &truncated);
    assert!(baseline_score > truncated_score);
}

#[test]
fn confidence_improves_for_stronger_margin_and_semantic_signals() {
    let shortlist = vec![hit("src/a.rs", 1.0), hit("src/b.rs", 0.7)];
    let context = context_with_chunk_source(&["src/a.rs", "src/b.rs"], "chunk_embedding_index");
    let weak = ConfidenceSignals {
        margin_top1_top2: 0.03,
        explain_coverage: 0.5,
        semantic_coverage: 0.0,
        semantic_outcome: "failed".to_string(),
        stage_drop_ratio: 0.4,
        hard_truncated: false,
    };
    let strong = ConfidenceSignals {
        margin_top1_top2: 0.45,
        explain_coverage: 1.0,
        semantic_coverage: 1.0,
        semantic_outcome: "applied_indexed".to_string(),
        stage_drop_ratio: 0.1,
        hard_truncated: false,
    };

    let weak_score = super::confidence::confidence_overall(&shortlist, &context, &weak);
    let strong_score = super::confidence::confidence_overall(&shortlist, &context, &strong);
    assert!(strong_score > weak_score);
}

#[test]
fn selected_context_uses_per_result_explain_breakdown_with_ranks() {
    let shortlist = vec![hit("src/lib.rs", 1.0)];
    let context = ContextSelection {
        files: vec![ContextFile {
            path: "src/lib.rs".to_string(),
            excerpt: "fn main() {}".to_string(),
            score: 1.0,
            chunk_idx: 2,
            start_line: 11,
            end_line: 20,
            chunk_source: "chunk_embedding_index".to_string(),
        }],
        total_chars: 12,
        estimated_tokens: 3,
        truncated: false,
        chunk_candidates: 1,
        chunk_selected: 1,
    };

    let explain_entries = vec![ResultExplainEntry {
        path: "src/lib.rs".to_string(),
        breakdown: RankExplainBreakdown {
            lexical: 0.77,
            graph: 0.11,
            semantic: 0.22,
            rrf: 0.33,
            graph_rrf: 0.07,
            rank_before: 2,
            rank_after: 1,
            semantic_source: "indexed".to_string(),
            semantic_outcome: "applied_indexed".to_string(),
            graph_seed_path: "src/seed.rs".to_string(),
            graph_edge_kinds: vec!["incoming:ref_exact".to_string()],
            graph_hops: 1,
        },
    }];

    let report = build_query_report(
        Path::new("."),
        QueryReportBuildInput {
            shortlist: &shortlist,
            context: &context,
            max_tokens: 64,
            privacy_mode: PrivacyMode::Off,
            resolved_mode: AgentIntentMode::EntrypointMap,
            mode_source: ModeResolutionSource::Default,
            semantic_requested: true,
            semantic_outcome: SemanticRerankOutcome::AppliedRrfIndexed,
            explain_entries: &explain_entries,
            stage_counts: Some(RetrievalStageCounts {
                lexical_candidates: 4,
                semantic_file_candidates: 2,
                semantic_chunk_candidates: 1,
                semantic_candidates: 3,
                fused_candidates: 5,
                graph_candidates: 2,
                shortlist_candidates: 1,
            }),
            index_telemetry: IndexTelemetry {
                last_index_lock_wait_ms: 0,
                last_embedding_cache_hits: 0,
                last_embedding_cache_misses: 0,
                chunk_coverage: 1.0,
                chunk_source: "chunk_embedding_index".to_string(),
            },
            investigation_summary: None,
        },
    )
    .expect("report must build");

    let explain = &report.selected_context[0].explain;
    assert_eq!(report.selected_context[0].chunk_idx, 2);
    assert_eq!(report.selected_context[0].start_line, 11);
    assert_eq!(report.selected_context[0].end_line, 20);
    assert_eq!(
        report.selected_context[0].chunk_source,
        "chunk_embedding_index"
    );
    assert_eq!(explain.rank_before, 2);
    assert_eq!(explain.rank_after, 1);
    assert!((explain.lexical - 0.77).abs() < 1e-6);
    assert!((explain.rrf - 0.33).abs() < 1e-6);
    assert!((explain.graph_rrf - 0.07).abs() < 1e-6);
    assert_eq!(explain.graph_seed_path, "src/seed.rs");
    assert!(report.retrieval_pipeline.iter().any(|stage| stage.stage
        == "semantic_candidate_pool(local_dense_index)"
        && stage.candidates == 3
        && stage.kept == 3));
    assert!(report.retrieval_pipeline.iter().any(|stage| stage.stage
        == "graph_neighbor_pool(file_graph_edges)"
        && stage.candidates == 5
        && stage.kept == 2));
    assert!(report.retrieval_pipeline.iter().any(|stage| stage.stage
        == "semantic_chunk_candidate_pool(file_chunks)"
        && stage.candidates == 1
        && stage.kept == 1));
}

#[test]
fn confidence_signals_report_full_explain_coverage_for_top_k() {
    let shortlist = vec![hit("src/a.rs", 1.0), hit("src/b.rs", 0.8)];
    let context = ContextSelection {
        files: vec![ContextFile {
            path: "src/a.rs".to_string(),
            excerpt: "fn a() {}".to_string(),
            score: 1.0,
            chunk_idx: 0,
            start_line: 1,
            end_line: 1,
            chunk_source: "chunk_embedding_index".to_string(),
        }],
        total_chars: 8,
        estimated_tokens: 2,
        truncated: false,
        chunk_candidates: 2,
        chunk_selected: 1,
    };
    let explain_entries = vec![
        ResultExplainEntry {
            path: "src/a.rs".to_string(),
            breakdown: RankExplainBreakdown {
                lexical: 0.5,
                graph: 0.1,
                semantic: 0.2,
                rrf: 0.3,
                graph_rrf: 0.0,
                rank_before: 1,
                rank_after: 1,
                semantic_source: "indexed".to_string(),
                semantic_outcome: "applied_indexed".to_string(),
                graph_seed_path: String::new(),
                graph_edge_kinds: Vec::new(),
                graph_hops: 0,
            },
        },
        ResultExplainEntry {
            path: "src/b.rs".to_string(),
            breakdown: RankExplainBreakdown {
                lexical: 0.4,
                graph: 0.1,
                semantic: 0.2,
                rrf: 0.2,
                graph_rrf: 0.0,
                rank_before: 2,
                rank_after: 2,
                semantic_source: "indexed".to_string(),
                semantic_outcome: "applied_indexed".to_string(),
                graph_seed_path: String::new(),
                graph_edge_kinds: Vec::new(),
                graph_hops: 0,
            },
        },
    ];

    let signals = super::confidence::confidence_signals(
        &shortlist,
        &context,
        true,
        SemanticRerankOutcome::AppliedRrfIndexed,
        &explain_entries,
    );
    assert!((signals.explain_coverage - 1.0).abs() < 1e-6);
}
