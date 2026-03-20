use std::collections::HashSet;

use crate::model::{ConfidenceSignals, ContextSelection, SearchHit};
use crate::vector_rank::SemanticRerankOutcome;

use super::ResultExplainEntry;
use super::helpers::semantic_outcome_code;

pub(super) fn confidence_signals(
    shortlist: &[SearchHit],
    context: &ContextSelection,
    semantic_requested: bool,
    semantic_outcome: SemanticRerankOutcome,
    explain_entries: &[ResultExplainEntry],
) -> ConfidenceSignals {
    let explain_paths = explain_entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect::<HashSet<_>>();
    let explain_coverage = if shortlist.is_empty() {
        1.0
    } else {
        let covered = shortlist
            .iter()
            .filter(|hit| explain_paths.contains(hit.path.as_str()))
            .count();
        covered as f32 / shortlist.len() as f32
    };

    let margin_top1_top2 = match shortlist {
        [] => 0.0,
        [single] => single.score.max(0.0),
        [first, second, ..] => (first.score - second.score).max(0.0),
    };

    let semantic_coverage = if explain_entries.is_empty() {
        if semantic_requested
            && matches!(
                semantic_outcome,
                SemanticRerankOutcome::AppliedRrfIndexed
                    | SemanticRerankOutcome::AppliedRrfFallback
                    | SemanticRerankOutcome::AppliedRrfMixed
            )
        {
            1.0
        } else {
            0.0
        }
    } else {
        let with_semantic = explain_entries
            .iter()
            .filter(|entry| entry.breakdown.semantic_source != "none")
            .count();
        with_semantic as f32 / explain_entries.len() as f32
    };

    let stage_drop_ratio = if shortlist.is_empty() {
        0.0
    } else {
        let dropped = shortlist.len().saturating_sub(context.files.len());
        dropped as f32 / shortlist.len() as f32
    };

    ConfidenceSignals {
        margin_top1_top2,
        explain_coverage: explain_coverage.clamp(0.0, 1.0),
        semantic_coverage: semantic_coverage.clamp(0.0, 1.0),
        semantic_outcome: semantic_outcome_code(semantic_requested, semantic_outcome).to_string(),
        stage_drop_ratio: stage_drop_ratio.clamp(0.0, 1.0),
        hard_truncated: context.truncated,
    }
}

pub(super) fn confidence_overall(
    shortlist: &[SearchHit],
    context: &ContextSelection,
    signals: &ConfidenceSignals,
) -> f32 {
    if shortlist.is_empty() {
        return 0.0;
    }

    // Saturating margin curve avoids hard thresholds and keeps score fully signal-driven.
    let margin_signal =
        (signals.margin_top1_top2 / (signals.margin_top1_top2 + 0.30)).clamp(0.0, 1.0);
    let semantic_outcome_signal = semantic_outcome_signal(signals.semantic_outcome.as_str());
    let semantic_signal = ((0.35 + (0.65 * signals.semantic_coverage.clamp(0.0, 1.0)))
        * semantic_outcome_signal)
        .clamp(0.0, 1.0);
    let retention_signal = (1.0 - signals.stage_drop_ratio).clamp(0.0, 1.0);
    let preview_fallback_ratio = preview_fallback_ratio(context);
    let planning_ratio = planning_ratio(context);
    let context_quality_signal =
        (1.0 - (0.55 * preview_fallback_ratio) - (0.75 * planning_ratio)).clamp(0.25, 1.0);

    let mut score = (0.45 * margin_signal) + (0.35 * semantic_signal) + (0.20 * retention_signal);
    score *= context_quality_signal;
    if shortlist
        .first()
        .is_some_and(|top| is_hidden_planning_path(&top.path))
    {
        score *= 0.55;
    }
    if signals.hard_truncated {
        let truncation_factor = (0.85 - (0.25 * signals.stage_drop_ratio)).clamp(0.45, 0.85);
        score *= truncation_factor;
    }
    score.clamp(0.0, 1.0)
}

pub(super) fn confidence_reasons(
    semantic_requested: bool,
    semantic_outcome: SemanticRerankOutcome,
    shortlist: &[SearchHit],
    context: &ContextSelection,
    signals: &ConfidenceSignals,
) -> Vec<String> {
    let margin_signal =
        (signals.margin_top1_top2 / (signals.margin_top1_top2 + 0.30)).clamp(0.0, 1.0);
    let semantic_outcome_signal = semantic_outcome_signal(signals.semantic_outcome.as_str());
    let semantic_signal = ((0.35 + (0.65 * signals.semantic_coverage.clamp(0.0, 1.0)))
        * semantic_outcome_signal)
        .clamp(0.0, 1.0);
    let retention_signal = (1.0 - signals.stage_drop_ratio).clamp(0.0, 1.0);
    let preview_fallback_ratio = preview_fallback_ratio(context);
    let planning_ratio = planning_ratio(context);
    let context_quality_signal =
        (1.0 - (0.55 * preview_fallback_ratio) - (0.75 * planning_ratio)).clamp(0.25, 1.0);

    let mut reasons = vec![
        format!("top1-top2 margin={:.4}", signals.margin_top1_top2),
        format!(
            "explain_coverage={:.2}, semantic coverage={:.2}, outcome={}",
            signals.explain_coverage, signals.semantic_coverage, signals.semantic_outcome
        ),
        format!(
            "stage_drop_ratio={:.2}, hard_truncated={}",
            signals.stage_drop_ratio, signals.hard_truncated
        ),
        format!(
            "component_signals margin={:.3}, semantic={:.3}, retention={:.3}, context={:.3}",
            margin_signal, semantic_signal, retention_signal, context_quality_signal
        ),
    ];
    if preview_fallback_ratio > 0.0 {
        reasons.push(format!(
            "preview_fallback_ratio={preview_fallback_ratio:.2} lowered confidence"
        ));
    }
    if planning_ratio > 0.0 {
        reasons.push(format!(
            "planning_path_ratio={planning_ratio:.2} lowered confidence"
        ));
    }
    if shortlist
        .first()
        .is_some_and(|top| is_hidden_planning_path(&top.path))
    {
        reasons.push("top-ranked result is a hidden planning path".to_string());
    }
    match (semantic_requested, semantic_outcome) {
        (true, SemanticRerankOutcome::AppliedRrfIndexed) => {
            reasons.push("RRF fusion applied with indexed semantic vectors".to_string());
        }
        (true, SemanticRerankOutcome::AppliedRrfFallback) => {
            reasons.push("RRF fusion applied with fallback semantic vectors".to_string());
        }
        (true, SemanticRerankOutcome::AppliedRrfMixed) => {
            reasons.push("RRF fusion applied with mixed semantic sources".to_string());
        }
        (true, SemanticRerankOutcome::ShortCircuitedLexical) => {
            reasons.push("semantic stage short-circuited by strong lexical signal".to_string());
        }
        (true, SemanticRerankOutcome::Failed) => {
            reasons.push("semantic stage failed and was not applied".to_string());
        }
        (true, SemanticRerankOutcome::NotApplied) => {
            reasons.push("semantic rerank requested but not applied".to_string());
        }
        _ => {}
    }
    reasons
}

fn semantic_outcome_signal(outcome: &str) -> f32 {
    match outcome {
        "applied_indexed" => 1.0,
        "applied_mixed" => 0.92,
        "applied_fallback" => 0.84,
        "short_circuit_lexical" => 0.74,
        "not_requested" => 0.66,
        "not_applied" => 0.52,
        "failed" => 0.12,
        _ => 0.40,
    }
}

fn preview_fallback_ratio(context: &ContextSelection) -> f32 {
    if context.files.is_empty() {
        return 0.0;
    }
    let preview_fallback_count = context
        .files
        .iter()
        .filter(|item| item.chunk_source == "preview_fallback")
        .count();
    preview_fallback_count as f32 / context.files.len() as f32
}

fn planning_ratio(context: &ContextSelection) -> f32 {
    if context.files.is_empty() {
        return 0.0;
    }
    let planning_count = context
        .files
        .iter()
        .filter(|item| is_hidden_planning_path(&item.path))
        .count();
    planning_count as f32 / context.files.len() as f32
}

fn is_hidden_planning_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized.starts_with(".codex-planning/") || normalized.contains("/.codex-planning/")
}
