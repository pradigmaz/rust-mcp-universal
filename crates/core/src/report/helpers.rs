use crate::model::{
    BootstrapProfile, CanonicalBasis, CanonicalFreshness, CanonicalProvenance, CanonicalStrength,
    ContextSelection, DegradationReason, InvestigationSummary, RankExplainBreakdown,
};
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

pub(crate) fn canonical_provenance_for_context_item(
    chunk_source: &str,
    explain: RankExplainBreakdown,
    score: f32,
) -> CanonicalProvenance {
    let preview_fallback = chunk_source == "preview_fallback";
    let graph_derived =
        explain.graph > 0.0 || explain.graph_hops > 0 || !explain.graph_seed_path.is_empty();
    let indexed = !chunk_source.is_empty() && !preview_fallback;
    let heuristic = explain.semantic_source == "none"
        || matches!(
            explain.semantic_outcome.as_str(),
            "not_requested" | "short_circuit_lexical" | "not_applied"
        );

    let basis = match (preview_fallback, graph_derived, indexed, heuristic) {
        (true, true, _, _) | (true, _, true, _) | (false, true, true, _) => CanonicalBasis::Mixed,
        (true, false, false, _) => CanonicalBasis::PreviewFallback,
        (false, true, false, _) => CanonicalBasis::GraphDerived,
        (false, false, true, _) => CanonicalBasis::Indexed,
        _ => CanonicalBasis::Heuristic,
    };
    let freshness = if preview_fallback {
        CanonicalFreshness::LiveRead
    } else if indexed || graph_derived {
        CanonicalFreshness::IndexSnapshot
    } else {
        CanonicalFreshness::Unknown
    };
    let strength = if preview_fallback {
        CanonicalStrength::FallbackOnly
    } else if matches!(
        explain.semantic_outcome.as_str(),
        "applied_indexed" | "applied_mixed"
    ) || graph_derived
    {
        CanonicalStrength::Strong
    } else if score >= 0.15 || explain.semantic_source != "none" {
        CanonicalStrength::Moderate
    } else {
        CanonicalStrength::Weak
    };

    let mut reasons = Vec::new();
    reasons.push(format!("chunk_source:{chunk_source}"));
    reasons.push(format!("semantic_source:{}", explain.semantic_source));
    reasons.push(format!("semantic_outcome:{}", explain.semantic_outcome));
    if explain.graph_hops > 0 {
        reasons.push(format!("graph_hops:{}", explain.graph_hops));
    }
    if !explain.graph_seed_path.is_empty() {
        reasons.push(format!("graph_seed_path:{}", explain.graph_seed_path));
    }

    CanonicalProvenance {
        basis,
        derivation: "context_selection".to_string(),
        freshness,
        strength,
        reasons,
    }
}

pub(crate) fn summarize_provenance(
    inputs: &[CanonicalProvenance],
    derivation: &str,
) -> CanonicalProvenance {
    if inputs.is_empty() {
        return CanonicalProvenance {
            basis: CanonicalBasis::Heuristic,
            derivation: derivation.to_string(),
            freshness: CanonicalFreshness::Unknown,
            strength: CanonicalStrength::Weak,
            reasons: vec!["no_surface_evidence".to_string()],
        };
    }

    let basis = {
        let mut counts = std::collections::HashMap::<CanonicalBasis, usize>::new();
        for item in inputs {
            *counts.entry(item.basis).or_default() += 1;
        }
        if counts.len() > 1 {
            CanonicalBasis::Mixed
        } else {
            counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(basis, _)| basis)
                .unwrap_or(CanonicalBasis::Heuristic)
        }
    };

    let freshness = if inputs
        .iter()
        .all(|item| item.freshness == CanonicalFreshness::LiveRead)
    {
        CanonicalFreshness::LiveRead
    } else if inputs
        .iter()
        .any(|item| item.freshness == CanonicalFreshness::IndexSnapshot)
    {
        CanonicalFreshness::IndexSnapshot
    } else {
        CanonicalFreshness::Unknown
    };

    let strength = if inputs
        .iter()
        .all(|item| item.strength == CanonicalStrength::FallbackOnly)
    {
        CanonicalStrength::FallbackOnly
    } else if inputs
        .iter()
        .any(|item| item.strength == CanonicalStrength::Strong)
    {
        CanonicalStrength::Strong
    } else if inputs
        .iter()
        .any(|item| item.strength == CanonicalStrength::Moderate)
    {
        CanonicalStrength::Moderate
    } else {
        CanonicalStrength::Weak
    };

    let mut reasons = inputs
        .iter()
        .flat_map(|item| item.reasons.iter().cloned())
        .fold(Vec::<String>::new(), |mut acc, reason| {
            if !acc.contains(&reason) {
                acc.push(reason);
            }
            acc
        });
    reasons.truncate(8);
    reasons.insert(
        0,
        format!("dominant_basis:{:?}", basis).to_ascii_lowercase(),
    );

    CanonicalProvenance {
        basis,
        derivation: derivation.to_string(),
        freshness,
        strength,
        reasons,
    }
}

pub(crate) fn derive_degradation_reasons(
    semantic_requested: bool,
    semantic_outcome: SemanticRerankOutcome,
    context: &ContextSelection,
    investigation_summary: Option<&InvestigationSummary>,
    profile_limited: bool,
) -> Vec<DegradationReason> {
    let mut reasons = Vec::new();

    if semantic_requested && semantic_outcome == SemanticRerankOutcome::Failed {
        reasons.push(DegradationReason::SemanticFailOpen);
    }
    if semantic_requested && semantic_outcome == SemanticRerankOutcome::NotApplied {
        reasons.push(DegradationReason::SemanticLowSignalSkip);
    }
    if context
        .files
        .iter()
        .any(|item| item.chunk_source == "preview_fallback")
    {
        reasons.push(DegradationReason::ChunkPreviewFallback);
    }
    if context.truncated {
        reasons.push(DegradationReason::BudgetTruncated);
    }
    if profile_limited {
        reasons.push(DegradationReason::ProfileLimited);
    }
    if investigation_summary
        .map(investigation_summary_has_unsupported_sources)
        .unwrap_or(false)
    {
        reasons.push(DegradationReason::UnsupportedSourcesPresent);
    }

    reasons
}

pub(crate) fn deepen_available(
    profile: Option<BootstrapProfile>,
    reasons: &[DegradationReason],
) -> bool {
    profile.is_some_and(|value| value != BootstrapProfile::Full) || !reasons.is_empty()
}

pub(crate) fn deepen_hint(
    profile: Option<BootstrapProfile>,
    reasons: &[DegradationReason],
) -> Option<String> {
    if profile.is_some_and(|value| value != BootstrapProfile::Full) {
        return Some(
            "rerun agent_bootstrap with profile=full to include both report and investigation summary"
                .to_string(),
        );
    }
    if reasons.contains(&DegradationReason::BudgetTruncated) {
        return Some("increase max_chars or max_tokens to reduce context truncation".to_string());
    }
    if reasons.contains(&DegradationReason::SemanticLowSignalSkip) {
        return Some("use a more specific query or pass an explicit mode".to_string());
    }
    if reasons.contains(&DegradationReason::ChunkPreviewFallback) {
        return Some("refresh the index or inspect the surfaced source files directly".to_string());
    }
    if reasons.contains(&DegradationReason::SemanticFailOpen) {
        return Some(
            "check the embedding backend or rerun with fail_closed to surface the error"
                .to_string(),
        );
    }
    if reasons.contains(&DegradationReason::UnsupportedSourcesPresent) {
        return Some(
            "inspect unsupported sources with symbol_body, route_trace, or constraint_evidence"
                .to_string(),
        );
    }
    None
}

fn investigation_summary_has_unsupported_sources(summary: &InvestigationSummary) -> bool {
    !summary.route_trace.unsupported_sources.is_empty()
        || !summary.constraint_evidence.unsupported_sources.is_empty()
        || summary
            .divergence
            .as_ref()
            .map(|divergence| !divergence.unsupported_sources.is_empty())
            .unwrap_or(false)
        || summary
            .provenance
            .reasons
            .iter()
            .any(|reason| reason == "unsupported_sources_present")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CanonicalBasis, CanonicalFreshness, CanonicalProvenance, CanonicalStrength, ContextFile,
        ContextSelection,
    };

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
        assert!(summary.reasons.iter().any(|reason| reason == "indexed_chunk"));
        assert!(summary.reasons.iter().any(|reason| reason == "live_crosscheck"));
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
