use std::{collections::HashMap, path::Path};

use anyhow::Result;
use time::OffsetDateTime;

use crate::model::{
    BudgetInfo, ConfidenceInfo, ContextSelection, IndexTelemetry, PrivacyMode, QueryReport,
    RankExplainBreakdown, SearchHit, SelectedContextItem,
};
use crate::vector_rank::SemanticRerankOutcome;
use crate::{sanitize_path_text, sanitize_value_for_privacy};

mod confidence;
mod helpers;
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
    pub semantic_requested: bool,
    pub semantic_outcome: SemanticRerankOutcome,
    pub explain_entries: &'a [ResultExplainEntry],
    pub stage_counts: Option<RetrievalStageCounts>,
    pub index_telemetry: IndexTelemetry,
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
        semantic_requested,
        semantic_outcome,
        explain_entries,
        stage_counts,
        index_telemetry,
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
        })
        .collect::<Vec<_>>();

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
        budget: BudgetInfo {
            max_tokens,
            used_estimate: context.estimated_tokens,
            hard_truncated: context.truncated,
        },
        retrieval_pipeline,
        selected_context,
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
    };
    let mut report_value = serde_json::to_value(&report)?;
    sanitize_value_for_privacy(privacy_mode, &mut report_value);
    report = serde_json::from_value(report_value)?;
    Ok(report)
}

#[cfg(test)]
mod tests;
