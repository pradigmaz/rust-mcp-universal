use std::collections::HashMap;

use crate::model::{RankExplainBreakdown, SearchHit};
use crate::report::ResultExplainEntry;
use crate::vector_rank::SemanticRerankOutcome;

use super::super::fusion::FusedExplainMeta;

pub(super) fn resolve_semantic_outcome(
    semantic_stage_failed: bool,
    semantic_requested: bool,
    low_signal_semantic: bool,
    semantic_enabled: bool,
    semantic_indexed: bool,
    semantic_fallback: bool,
) -> SemanticRerankOutcome {
    if semantic_stage_failed {
        SemanticRerankOutcome::Failed
    } else if !semantic_requested || low_signal_semantic {
        SemanticRerankOutcome::NotApplied
    } else if !semantic_enabled {
        SemanticRerankOutcome::ShortCircuitedLexical
    } else if semantic_indexed && semantic_fallback {
        SemanticRerankOutcome::AppliedRrfMixed
    } else if semantic_indexed {
        SemanticRerankOutcome::AppliedRrfIndexed
    } else if semantic_fallback {
        SemanticRerankOutcome::AppliedRrfFallback
    } else {
        SemanticRerankOutcome::NotApplied
    }
}

pub(super) fn build_explain_entries(
    hits: &[SearchHit],
    lexical_by_path: &HashMap<String, (f32, f32)>,
    lexical_rank_by_path: &HashMap<String, usize>,
    fused_explain: &HashMap<String, FusedExplainMeta>,
    semantic_outcome_label: &str,
) -> Vec<ResultExplainEntry> {
    hits.iter()
        .enumerate()
        .map(|(idx, hit)| {
            let (lexical, lexical_graph) = lexical_by_path
                .get(&hit.path)
                .copied()
                .unwrap_or((0.0, 0.0));
            let lexical_rank = lexical_rank_by_path
                .get(&hit.path)
                .copied()
                .unwrap_or(idx + 1);
            let semantic_entry = fused_explain.get(&hit.path);
            let semantic = semantic_entry
                .map(|entry| entry.semantic_score)
                .unwrap_or(0.0);
            let graph_stage = semantic_entry.map(|entry| entry.graph_score).unwrap_or(0.0);
            let rrf = semantic_entry.map(|entry| entry.rrf_score).unwrap_or(0.0);
            let graph_rrf = semantic_entry.map(|entry| entry.graph_rrf).unwrap_or(0.0);
            let semantic_source = semantic_entry
                .map(|entry| entry.semantic_source.clone())
                .unwrap_or_else(|| "none".to_string());
            let graph_seed_path = semantic_entry
                .map(|entry| entry.graph_seed_path.clone())
                .unwrap_or_default();
            let graph_edge_kinds = semantic_entry
                .map(|entry| entry.graph_edge_kinds.clone())
                .unwrap_or_default();
            let graph_hops = semantic_entry.map(|entry| entry.graph_hops).unwrap_or(0);
            let rank_before = semantic_entry
                .map(|entry| entry.rank_before)
                .unwrap_or(lexical_rank);
            let rank_after = semantic_entry
                .map(|entry| entry.rank_after)
                .unwrap_or(idx + 1);

            ResultExplainEntry {
                path: hit.path.clone(),
                breakdown: RankExplainBreakdown {
                    lexical,
                    graph: lexical_graph + graph_stage,
                    semantic,
                    rrf,
                    graph_rrf,
                    rank_before,
                    rank_after,
                    semantic_source,
                    semantic_outcome: semantic_outcome_label.to_string(),
                    graph_seed_path,
                    graph_edge_kinds,
                    graph_hops,
                },
            }
        })
        .collect()
}
