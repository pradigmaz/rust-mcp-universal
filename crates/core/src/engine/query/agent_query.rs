use std::time::Instant;

use anyhow::Result;

use crate::DegradationReason;
use crate::engine::Engine;
use crate::engine_brief::index_not_ready_error;
use crate::model::{
    AgentBootstrapTimings, AgentIntentMode, AgentQueryBundle, BootstrapProfile, IndexStatus,
    IndexTelemetry, PrivacyMode, QueryOptions, SemanticFailMode,
};

use super::agent_surface::{BootstrapQuerySurfaceInput, build_bootstrap_query_surface};
use super::intent::SearchIntent;

pub(super) struct AgentQueryBuildInput<'a> {
    pub(super) engine: &'a Engine,
    pub(super) query: &'a str,
    pub(super) limit: usize,
    pub(super) semantic: bool,
    pub(super) semantic_fail_mode: SemanticFailMode,
    pub(super) privacy_mode: PrivacyMode,
    pub(super) max_chars: usize,
    pub(super) max_tokens: usize,
    pub(super) agent_intent_mode: Option<AgentIntentMode>,
    pub(super) effective_profile: BootstrapProfile,
    pub(super) include_report: bool,
    pub(super) include_investigation_summary: bool,
    pub(super) index_status: &'a IndexStatus,
    pub(super) timings: &'a mut AgentBootstrapTimings,
}

pub(super) fn build_agent_query_bundle(
    input: AgentQueryBuildInput<'_>,
) -> Result<(AgentQueryBundle, Vec<DegradationReason>)> {
    if input.index_status.files == 0 {
        return Err(index_not_ready_error());
    }
    let requested_limit = input.limit.max(1);
    let options = QueryOptions {
        query: input.query.to_string(),
        limit: requested_limit,
        detailed: true,
        semantic: input.semantic,
        semantic_fail_mode: input.semantic_fail_mode,
        privacy_mode: input.privacy_mode,
        context_mode: None,
        agent_intent_mode: input.agent_intent_mode,
    };

    let phase_started = Instant::now();
    let execution = input.engine.search_with_meta(&options)?;
    input.timings.search_ms = elapsed_ms(phase_started);
    let followup_intent = input
        .agent_intent_mode
        .map(SearchIntent::from_agent_mode)
        .unwrap_or_else(|| SearchIntent::from_query(input.query));
    let followups = followup_intent.bootstrap_followups(&execution.hits);

    let phase_started = Instant::now();
    let context = input.engine.context_for_hits_with_chunks(
        input.query,
        &execution.hits,
        Some(&execution.chunk_by_path),
        None,
        input.max_chars,
        input.max_tokens,
    )?;
    input.timings.context_ms = elapsed_ms(phase_started);
    let (chunk_coverage, chunk_source) = super::context_telemetry::derive_chunk_telemetry(&context);

    let surface = build_bootstrap_query_surface(
        input.engine,
        BootstrapQuerySurfaceInput {
            project_root: &input.engine.project_root,
            query: input.query,
            requested_limit,
            semantic: input.semantic,
            privacy_mode: input.privacy_mode,
            max_tokens: input.max_tokens,
            effective_profile: input.effective_profile,
            include_report: input.include_report,
            include_investigation_summary: input.include_investigation_summary,
            execution: &execution,
            context: &context,
            index_telemetry: IndexTelemetry {
                last_index_lock_wait_ms: input.index_status.last_index_lock_wait_ms,
                last_embedding_cache_hits: input.index_status.last_embedding_cache_hits,
                last_embedding_cache_misses: input.index_status.last_embedding_cache_misses,
                chunk_coverage,
                chunk_source,
            },
        },
        input.timings,
    )?;
    let degradation_reasons = surface.degradation_reasons.clone();

    Ok((
        AgentQueryBundle {
            query: input.query.to_string(),
            limit: requested_limit,
            semantic: input.semantic,
            resolved_mode: execution.resolved_mode,
            mode_source: execution.mode_source,
            max_chars: input.max_chars,
            max_tokens: input.max_tokens,
            hits: execution.hits,
            context,
            provenance: surface.provenance,
            followups,
            investigation_summary: surface.investigation_summary,
            report: surface.report,
        },
        degradation_reasons,
    ))
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
