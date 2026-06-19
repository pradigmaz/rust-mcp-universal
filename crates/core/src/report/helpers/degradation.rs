use crate::model::{BootstrapProfile, ContextSelection, DegradationReason, InvestigationSummary};
use crate::vector_rank::SemanticRerankOutcome;

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
