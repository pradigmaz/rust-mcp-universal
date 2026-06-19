use std::path::Path;
use std::time::Instant;

use anyhow::Result;

use crate::model::{
    AgentBootstrapTimings, BootstrapProfile, CanonicalProvenance, ContextSelection,
    DegradationReason, IndexTelemetry, InvestigationPhaseTimings, InvestigationSummary,
    PrivacyMode, QueryReport, QuerySurfaceTimings,
};
use crate::report::{QueryReportBuildInput, build_query_report, helpers as report_helpers};

use super::{SearchExecution, investigation_embed};

pub(super) struct BootstrapQuerySurface {
    pub(super) investigation_summary: Option<InvestigationSummary>,
    pub(super) report: Option<QueryReport>,
    pub(super) provenance: CanonicalProvenance,
    pub(super) degradation_reasons: Vec<DegradationReason>,
}

pub(super) struct BootstrapQuerySurfaceInput<'a> {
    pub(super) project_root: &'a Path,
    pub(super) query: &'a str,
    pub(super) requested_limit: usize,
    pub(super) semantic: bool,
    pub(super) privacy_mode: PrivacyMode,
    pub(super) max_tokens: usize,
    pub(super) effective_profile: BootstrapProfile,
    pub(super) include_report: bool,
    pub(super) include_investigation_summary: bool,
    pub(super) execution: &'a SearchExecution,
    pub(super) context: &'a ContextSelection,
    pub(super) index_telemetry: IndexTelemetry,
}

pub(super) fn build_bootstrap_query_surface(
    engine: &crate::engine::Engine,
    input: BootstrapQuerySurfaceInput<'_>,
    timings: &mut AgentBootstrapTimings,
) -> Result<BootstrapQuerySurface> {
    let shared_investigation = if input.include_investigation_summary || input.include_report {
        let phase_started = Instant::now();
        let snapshot = super::super::investigation::shared_query_investigation_snapshot(
            engine,
            input.query,
            input.requested_limit,
        )?;
        timings.investigation_ms = elapsed_ms(phase_started);
        Some(snapshot)
    } else {
        None
    };

    let embedded_investigation_summary =
        if input.include_investigation_summary || input.include_report {
            shared_investigation
                .as_ref()
                .map(investigation_embed::format_investigation_summary)
        } else {
            None
        };

    let investigation_summary = if input.include_investigation_summary {
        embedded_investigation_summary.clone()
    } else {
        None
    };

    let investigation_phase_timings = shared_investigation
        .as_ref()
        .map(|snapshot| snapshot.timings)
        .unwrap_or_else(InvestigationPhaseTimings::default);

    let selected_provenance = input
        .context
        .files
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let explain = input
                .execution
                .explain_entries
                .iter()
                .find(|entry| entry.path == item.path)
                .map(|entry| entry.breakdown.clone())
                .unwrap_or_else(|| {
                    report_helpers::default_breakdown(
                        idx + 1,
                        input.semantic,
                        input.execution.semantic_outcome,
                        item.score.max(0.0),
                    )
                });
            report_helpers::canonical_provenance_for_context_item(
                &item.chunk_source,
                explain,
                item.score,
            )
        })
        .collect::<Vec<_>>();
    let mut bundle_provenance_inputs = selected_provenance;
    if let Some(summary) = embedded_investigation_summary.as_ref() {
        bundle_provenance_inputs.push(summary.provenance.clone());
    }
    let provenance =
        report_helpers::summarize_provenance(&bundle_provenance_inputs, "agent_query_bundle");
    let degradation_reasons = report_helpers::derive_degradation_reasons(
        input.semantic,
        input.execution.semantic_outcome,
        input.context,
        embedded_investigation_summary.as_ref(),
        input.effective_profile != BootstrapProfile::Full,
    );

    let report = if input.include_report {
        let phase_started = Instant::now();
        let mut report = build_query_report(
            input.project_root,
            QueryReportBuildInput {
                shortlist: &input.execution.hits,
                context: input.context,
                max_tokens: input.max_tokens,
                privacy_mode: input.privacy_mode,
                resolved_mode: input.execution.resolved_mode,
                mode_source: input.execution.mode_source,
                semantic_requested: input.semantic,
                semantic_outcome: input.execution.semantic_outcome,
                explain_entries: &input.execution.explain_entries,
                stage_counts: Some(input.execution.stage_counts),
                index_telemetry: input.index_telemetry,
                investigation_summary: embedded_investigation_summary.clone(),
            },
        )?;
        timings.report_ms = elapsed_ms(phase_started);
        report.timings = Some(QuerySurfaceTimings {
            search_ms: timings.search_ms,
            context_ms: timings.context_ms,
            investigation_ms: timings.investigation_ms,
            format_ms: timings.report_ms,
            total_ms: timings
                .search_ms
                .saturating_add(timings.context_ms)
                .saturating_add(timings.investigation_ms)
                .saturating_add(timings.report_ms),
            investigation: investigation_phase_timings,
        });
        Some(report)
    } else {
        None
    };

    Ok(BootstrapQuerySurface {
        investigation_summary,
        report,
        provenance,
        degradation_reasons,
    })
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}
