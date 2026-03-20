use crate::model::RankExplainBreakdown;
use crate::vector_rank::SemanticRerankOutcome;

pub(super) fn default_breakdown(
    rank: usize,
    semantic_requested: bool,
    semantic_outcome: SemanticRerankOutcome,
    lexical_score: f32,
) -> RankExplainBreakdown {
    RankExplainBreakdown {
        lexical: lexical_score,
        graph: 0.0,
        semantic: 0.0,
        rrf: 0.0,
        graph_rrf: 0.0,
        rank_before: rank,
        rank_after: rank,
        semantic_source: "none".to_string(),
        semantic_outcome: semantic_outcome_code(semantic_requested, semantic_outcome).to_string(),
        graph_seed_path: String::new(),
        graph_edge_kinds: Vec::new(),
        graph_hops: 0,
    }
}

pub(super) fn semantic_outcome_code(
    semantic_requested: bool,
    semantic_outcome: SemanticRerankOutcome,
) -> &'static str {
    if !semantic_requested {
        return "not_requested";
    }
    match semantic_outcome {
        SemanticRerankOutcome::AppliedRrfIndexed => "applied_indexed",
        SemanticRerankOutcome::AppliedRrfFallback => "applied_fallback",
        SemanticRerankOutcome::AppliedRrfMixed => "applied_mixed",
        SemanticRerankOutcome::ShortCircuitedLexical => "short_circuit_lexical",
        SemanticRerankOutcome::Failed => "failed",
        SemanticRerankOutcome::NotApplied => "not_applied",
    }
}

pub(super) fn semantic_stage_name(
    semantic_requested: bool,
    semantic_outcome: SemanticRerankOutcome,
) -> Option<String> {
    if !semantic_requested {
        return None;
    }
    let name = match semantic_outcome {
        SemanticRerankOutcome::AppliedRrfIndexed => "semantic_vector_rerank(local_dense_index_rrf)",
        SemanticRerankOutcome::AppliedRrfFallback => {
            "semantic_vector_rerank(fallback_in_memory_rrf)"
        }
        SemanticRerankOutcome::AppliedRrfMixed => {
            "semantic_vector_rerank(mixed_index_and_fallback_rrf)"
        }
        SemanticRerankOutcome::ShortCircuitedLexical => {
            "semantic_vector_rerank(short_circuit_strong_lexical)"
        }
        SemanticRerankOutcome::Failed => "semantic_vector_rerank(failed)",
        SemanticRerankOutcome::NotApplied => "semantic_vector_rerank(skipped_no_signal)",
    };
    Some(name.to_string())
}

pub(super) fn context_reasons(
    semantic_requested: bool,
    semantic_outcome: SemanticRerankOutcome,
) -> Vec<String> {
    let mut reasons = vec![
        "matched lexical/fts query".to_string(),
        "within explicit budget cut".to_string(),
    ];
    match (semantic_requested, semantic_outcome) {
        (true, SemanticRerankOutcome::AppliedRrfIndexed) => {
            reasons.push("ranked by RRF fusion (lexical + indexed semantic)".to_string());
        }
        (true, SemanticRerankOutcome::AppliedRrfFallback) => {
            reasons.push("ranked by RRF fusion (lexical + fallback semantic)".to_string());
        }
        (true, SemanticRerankOutcome::AppliedRrfMixed) => {
            reasons.push("ranked by RRF fusion (mixed semantic sources)".to_string());
        }
        (true, SemanticRerankOutcome::ShortCircuitedLexical) => {
            reasons.push("kept lexical ranking due strong lexical short-circuit".to_string());
        }
        (true, SemanticRerankOutcome::Failed) => {
            reasons.push("semantic rerank failed; lexical ranking retained".to_string());
        }
        (true, SemanticRerankOutcome::NotApplied) => {
            reasons.push("semantic rerank requested but skipped due low-signal query".to_string());
        }
        _ => {}
    }
    reasons
}

pub(super) fn gap_reasons(
    semantic_requested: bool,
    semantic_outcome: SemanticRerankOutcome,
) -> Vec<String> {
    let mut gaps = vec!["symbol/dependency extraction is heuristic in MVP".to_string()];
    if !semantic_requested {
        gaps.push("semantic rerank disabled for this query".to_string());
        return gaps;
    }

    gaps.push(
        "semantic embeddings use configured backend with deterministic projection to local dense space"
            .to_string(),
    );
    if semantic_outcome == SemanticRerankOutcome::ShortCircuitedLexical {
        gaps.push("semantic rerank skipped due strong lexical confidence".to_string());
    }
    if semantic_outcome == SemanticRerankOutcome::Failed {
        gaps.push("semantic rerank failed; check local embedding backend/runtime".to_string());
    }
    if semantic_outcome == SemanticRerankOutcome::NotApplied {
        gaps.push("semantic rerank was requested but skipped due low-signal query".to_string());
    }
    gaps
}
