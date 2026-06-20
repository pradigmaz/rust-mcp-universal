#[path = "helpers/degradation.rs"]
pub(crate) mod degradation;
#[path = "helpers/provenance.rs"]
pub(crate) mod provenance;

use crate::model::RankExplainBreakdown;
use crate::vector_rank::SemanticRerankOutcome;

pub(crate) fn default_breakdown(
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

#[cfg(test)]
mod tests {
    use super::degradation::{deepen_available, deepen_hint, derive_degradation_reasons};
    use super::provenance::summarize_provenance;
    use crate::model::{
        BootstrapProfile, CanonicalBasis, CanonicalFreshness, CanonicalProvenance,
        CanonicalStrength, ContextFile, ContextSelection, DegradationReason,
    };
    use crate::vector_rank::SemanticRerankOutcome;

    fn context_selection(chunk_source: &str, truncated: bool) -> ContextSelection {
        ContextSelection {
            files: vec![ContextFile {
                path: "src/lib.rs".to_string(),
                excerpt: "fn sample() {}".to_string(),
                score: 0.9,
                chunk_idx: 0,
                start_line: 1,
                end_line: 1,
                chunk_source: chunk_source.to_string(),
            }],
            total_chars: 14,
            estimated_tokens: 4,
            truncated,
            chunk_candidates: 1,
            chunk_selected: 1,
        }
    }

    #[test]
    fn summarize_provenance_emits_dominant_basis_reason() {
        let summary = summarize_provenance(
            &[
                CanonicalProvenance {
                    basis: CanonicalBasis::Indexed,
                    derivation: "context_selection".to_string(),
                    freshness: CanonicalFreshness::IndexSnapshot,
                    strength: CanonicalStrength::Strong,
                    reasons: vec!["indexed_chunk".to_string()],
                },
                CanonicalProvenance {
                    basis: CanonicalBasis::Indexed,
                    derivation: "investigation_summary".to_string(),
                    freshness: CanonicalFreshness::LiveRead,
                    strength: CanonicalStrength::Moderate,
                    reasons: vec!["live_crosscheck".to_string()],
                },
            ],
            "agent_query_bundle",
        );

        assert_eq!(summary.basis, CanonicalBasis::Indexed);
        assert_eq!(summary.derivation, "agent_query_bundle");
        assert_eq!(summary.strength, CanonicalStrength::Strong);
        assert_eq!(summary.reasons[0], "dominant_basis:indexed");
        assert!(
            summary
                .reasons
                .iter()
                .any(|reason| reason == "indexed_chunk")
        );
        assert!(
            summary
                .reasons
                .iter()
                .any(|reason| reason == "live_crosscheck")
        );
    }

    #[test]
    fn degradation_reasons_collect_profile_budget_preview_and_semantic_flags() {
        let reasons = derive_degradation_reasons(
            true,
            SemanticRerankOutcome::Failed,
            &context_selection("preview_fallback", true),
            None,
            true,
        );

        assert_eq!(
            reasons,
            vec![
                DegradationReason::SemanticFailOpen,
                DegradationReason::ChunkPreviewFallback,
                DegradationReason::BudgetTruncated,
                DegradationReason::ProfileLimited,
            ]
        );
    }

    #[test]
    fn deepen_contract_prefers_profile_rerun_for_non_full_bootstrap() {
        let reasons = vec![
            DegradationReason::ProfileLimited,
            DegradationReason::BudgetTruncated,
        ];
        assert!(deepen_available(Some(BootstrapProfile::Fast), &reasons));
        assert_eq!(
            deepen_hint(Some(BootstrapProfile::Fast), &reasons).as_deref(),
            Some(
                "rerun agent_bootstrap with profile=full to include both report and investigation summary"
            )
        );
    }

    #[test]
    fn deepen_contract_prefers_budget_hint_for_full_profiles() {
        let reasons = vec![
            DegradationReason::BudgetTruncated,
            DegradationReason::SemanticLowSignalSkip,
        ];
        assert!(deepen_available(Some(BootstrapProfile::Full), &reasons));
        assert_eq!(
            deepen_hint(Some(BootstrapProfile::Full), &reasons).as_deref(),
            Some("increase max_chars or max_tokens to reduce context truncation")
        );
        assert!(!deepen_available(Some(BootstrapProfile::Full), &[]));
        assert_eq!(deepen_hint(Some(BootstrapProfile::Full), &[]), None);
    }
}
