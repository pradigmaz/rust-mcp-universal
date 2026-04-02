use std::{collections::HashMap, path::Path};

use anyhow::Result;
use time::OffsetDateTime;

use crate::model::{
    AgentIntentMode, BudgetInfo, ConfidenceInfo, ContextSelection, IndexTelemetry,
    InvestigationSummary, ModeResolutionSource, PrivacyMode, QueryReport,
    RankExplainBreakdown, SearchHit, SelectedContextItem,
};
use crate::vector_rank::SemanticRerankOutcome;
use crate::{sanitize_path_text, sanitize_value_for_privacy};

mod confidence;
pub(crate) mod helpers;
mod pipeline;

#[derive(Debug, Clone, Copy)]
pub struct RetrievalStageCounts {
    pub lexical_candidates: usize,
    pub semantic_file_candidates: usize,
    pub semantic_chunk_candidates: usize,
    pub semantic_candidates: usize,
    pub fused_candidates: usize,
    pub graph_candidates: usize,
    pub shortlist_candidates: usize,
}

#[derive(Debug, Clone)]
pub struct ResultExplainEntry {
    pub path: String,
    pub breakdown: RankExplainBreakdown,
}

#[derive(Debug, Clone)]
pub struct QueryReportBuildInput<'a> {
    pub shortlist: &'a [SearchHit],
    pub context: &'a ContextSelection,
    pub max_tokens: usize,
    pub privacy_mode: PrivacyMode,
    pub resolved_mode: AgentIntentMode,
    pub mode_source: ModeResolutionSource,
    pub semantic_requested: bool,
    pub semantic_outcome: SemanticRerankOutcome,
    pub explain_entries: &'a [ResultExplainEntry],
    pub stage_counts: Option<RetrievalStageCounts>,
    pub index_telemetry: IndexTelemetry,
    pub investigation_summary: Option<InvestigationSummary>,
}

pub(crate) fn build_query_report(
    project_root: &Path,
    input: QueryReportBuildInput<'_>,
) -> Result<QueryReport> {
    let QueryReportBuildInput {
        shortlist,
        context,
        max_tokens,
        privacy_mode,
        resolved_mode,
        mode_source,
        semantic_requested,
        semantic_outcome,
        explain_entries,
        stage_counts,
        index_telemetry,
        investigation_summary,
    } = input;

    let explain_by_path = explain_entries
        .iter()
        .map(|entry| (entry.path.clone(), entry.breakdown.clone()))
        .collect::<HashMap<_, _>>();

    let selected_context = context
        .files
        .iter()
        .enumerate()
        .map(|(idx, item)| SelectedContextItem {
            path: sanitize_path_text(privacy_mode, &item.path),
            score: item.score,
            chars: item.excerpt.chars().count(),
            chunk_idx: item.chunk_idx,
            start_line: item.start_line,
            end_line: item.end_line,
            chunk_source: item.chunk_source.clone(),
            why: helpers::context_reasons(semantic_requested, semantic_outcome),
            explain: explain_by_path.get(&item.path).cloned().unwrap_or_else(|| {
                helpers::default_breakdown(
                    idx + 1,
                    semantic_requested,
                    semantic_outcome,
                    item.score.max(0.0),
                )
            }),
            provenance: helpers::canonical_provenance_for_context_item(
                &item.chunk_source,
                explain_by_path.get(&item.path).cloned().unwrap_or_else(|| {
                    helpers::default_breakdown(
                        idx + 1,
                        semantic_requested,
                        semantic_outcome,
                        item.score.max(0.0),
                    )
                }),
                item.score,
            ),
        })
        .collect::<Vec<_>>();

    let mut provenance_inputs = selected_context
        .iter()
        .map(|item| item.provenance.clone())
        .collect::<Vec<_>>();
    if let Some(summary) = investigation_summary.as_ref() {
        provenance_inputs.push(summary.provenance.clone());
    }
    let provenance = helpers::summarize_provenance(&provenance_inputs, "query_report");
    let degradation_reasons = helpers::derive_degradation_reasons(
        semantic_requested,
        semantic_outcome,
        context,
        investigation_summary.as_ref(),
        false,
    );

    let retrieval_pipeline = pipeline::build_retrieval_pipeline(
        shortlist.len(),
        context.chunk_candidates,
        context.files.len(),
        semantic_requested,
        semantic_outcome,
        stage_counts,
    );

    let signals = confidence::confidence_signals(
        shortlist,
        context,
        semantic_requested,
        semantic_outcome,
        explain_entries,
    );

    let mut report = QueryReport {
        query_id: format!("q-{}", OffsetDateTime::now_utc().unix_timestamp_nanos()),
        timestamp_utc: OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)?,
        project_root: sanitize_path_text(privacy_mode, &project_root.display().to_string()),
        resolved_mode,
        mode_source,
        budget: BudgetInfo {
            max_tokens,
            used_estimate: context.estimated_tokens,
            hard_truncated: context.truncated,
        },
        retrieval_pipeline,
        selected_context,
        provenance,
        confidence: ConfidenceInfo {
            overall: confidence::confidence_overall(shortlist, context, &signals),
            reasons: confidence::confidence_reasons(
                semantic_requested,
                semantic_outcome,
                shortlist,
                context,
                &signals,
            ),
            signals,
        },
        gaps: helpers::gap_reasons(semantic_requested, semantic_outcome),
        index_telemetry,
        degradation_reasons: degradation_reasons.clone(),
        deepen_available: helpers::deepen_available(None, &degradation_reasons),
        deepen_hint: helpers::deepen_hint(None, &degradation_reasons),
        investigation_summary,
        timings: None,
    };
    let mut report_value = serde_json::to_value(&report)?;
    sanitize_value_for_privacy(privacy_mode, &mut report_value);
    report = serde_json::from_value(report_value)?;
    Ok(report)
}

#[cfg(test)]
mod tests;
