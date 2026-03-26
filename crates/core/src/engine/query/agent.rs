use std::time::Instant;

use anyhow::Result;

use crate::engine_brief::index_not_ready_error;
use crate::engine_quality::load_quality_summary;
use crate::model::{
    AgentBootstrap, AgentBootstrapIncludeOptions, AgentBootstrapTimings, AgentQueryBundle,
    IndexTelemetry, InvestigationPhaseTimings, PrivacyMode, QueryOptions, QuerySurfaceTimings,
    SemanticFailMode, WorkspaceBrief,
};
use crate::report::{QueryReportBuildInput, build_query_report};

use super::super::Engine;

impl Engine {
    pub fn agent_bootstrap(
        &self,
        query: Option<&str>,
        limit: usize,
        semantic: bool,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<AgentBootstrap> {
        self.agent_bootstrap_with_mode(
            query,
            limit,
            semantic,
            SemanticFailMode::FailOpen,
            PrivacyMode::Off,
            max_chars,
            max_tokens,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "public compatibility for CLI and MCP callers"
    )]
    pub fn agent_bootstrap_with_mode(
        &self,
        query: Option<&str>,
        limit: usize,
        semantic: bool,
        semantic_fail_mode: SemanticFailMode,
        privacy_mode: PrivacyMode,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<AgentBootstrap> {
        self.agent_bootstrap_with_auto_index_and_mode(
            query,
            limit,
            semantic,
            semantic_fail_mode,
            privacy_mode,
            max_chars,
            max_tokens,
            true,
        )
    }

    pub fn agent_bootstrap_with_auto_index(
        &self,
        query: Option<&str>,
        limit: usize,
        semantic: bool,
        max_chars: usize,
        max_tokens: usize,
        auto_index: bool,
    ) -> Result<AgentBootstrap> {
        self.agent_bootstrap_with_auto_index_and_mode(
            query,
            limit,
            semantic,
            SemanticFailMode::FailOpen,
            PrivacyMode::Off,
            max_chars,
            max_tokens,
            auto_index,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "public compatibility for CLI and MCP callers"
    )]
    pub fn agent_bootstrap_with_auto_index_and_options(
        &self,
        query: Option<&str>,
        limit: usize,
        semantic: bool,
        semantic_fail_mode: SemanticFailMode,
        privacy_mode: PrivacyMode,
        max_chars: usize,
        max_tokens: usize,
        auto_index: bool,
        include: AgentBootstrapIncludeOptions,
    ) -> Result<AgentBootstrap> {
        let started = Instant::now();
        let normalized_query = query
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let query_requested = normalized_query.is_some();
        let mut timings = AgentBootstrapTimings::default();

        if query_requested {
            let phase_started = Instant::now();
            let _ = self.ensure_index_ready_with_policy(auto_index)?;
            timings.index_ready_ms = elapsed_ms(phase_started);
        }

        let phase_started = Instant::now();
        let brief = if auto_index || query_requested {
            self.workspace_brief_with_policy(auto_index)?
        } else {
            let status = self.index_status()?;
            WorkspaceBrief {
                auto_indexed: false,
                index_status: status.clone(),
                languages: super::brief::load_top_languages_for_brief(self)?,
                top_symbols: super::brief::load_top_symbols_for_brief(self)?,
                quality_summary: load_quality_summary(self)?,
                recommendations: super::brief::make_brief_recommendations(&status),
                repair_hint: None,
            }
        };
        timings.brief_ms = elapsed_ms(phase_started);

        let query_bundle = normalized_query
            .as_deref()
            .map(|value| -> Result<AgentQueryBundle> {
                if brief.index_status.files == 0 {
                    return Err(index_not_ready_error());
                }
                let requested_limit = limit.max(1);
                let options = QueryOptions {
                    query: value.to_string(),
                    limit: requested_limit,
                    detailed: true,
                    semantic,
                    semantic_fail_mode,
                    privacy_mode,
                    context_mode: None,
                };

                let phase_started = Instant::now();
                let execution = self.search_with_meta(&options)?;
                timings.search_ms = elapsed_ms(phase_started);

                let phase_started = Instant::now();
                let context = self.context_for_hits_with_chunks(
                    value,
                    &execution.hits,
                    Some(&execution.chunk_by_path),
                    None,
                    max_chars,
                    max_tokens,
                )?;
                timings.context_ms = elapsed_ms(phase_started);
                let (chunk_coverage, chunk_source) = super::derive_chunk_telemetry(&context);

                let shared_investigation =
                    if include.include_investigation_summary || include.include_report {
                        let phase_started = Instant::now();
                        let snapshot = super::super::investigation::shared_query_investigation_snapshot(
                            self,
                            value,
                            requested_limit,
                        )?;
                        timings.investigation_ms = elapsed_ms(phase_started);
                        Some(snapshot)
                    } else {
                        None
                    };

                let investigation_summary = if include.include_investigation_summary {
                    shared_investigation
                        .as_ref()
                        .map(super::investigation_embed::format_investigation_summary)
                } else {
                    None
                };

                let investigation_phase_timings = shared_investigation
                    .as_ref()
                    .map(|snapshot| snapshot.timings)
                    .unwrap_or_else(InvestigationPhaseTimings::default);

                let report = if include.include_report {
                    let phase_started = Instant::now();
                    let mut report = build_query_report(
                        &self.project_root,
                        QueryReportBuildInput {
                            shortlist: &execution.hits,
                            context: &context,
                            max_tokens,
                            privacy_mode,
                            semantic_requested: semantic,
                            semantic_outcome: execution.semantic_outcome,
                            explain_entries: &execution.explain_entries,
                            stage_counts: Some(execution.stage_counts),
                            index_telemetry: IndexTelemetry {
                                last_index_lock_wait_ms: brief.index_status.last_index_lock_wait_ms,
                                last_embedding_cache_hits: brief
                                    .index_status
                                    .last_embedding_cache_hits,
                                last_embedding_cache_misses: brief
                                    .index_status
                                    .last_embedding_cache_misses,
                                chunk_coverage,
                                chunk_source,
                            },
                            investigation_summary: investigation_summary.clone(),
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

                Ok(AgentQueryBundle {
                    query: value.to_string(),
                    limit: requested_limit,
                    semantic,
                    max_chars,
                    max_tokens,
                    hits: execution.hits,
                    context,
                    investigation_summary,
                    report,
                })
            })
            .transpose()?;

        timings.total_ms = elapsed_ms(started);
        Ok(AgentBootstrap {
            brief,
            query_bundle,
            timings,
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "public compatibility for CLI and MCP callers"
    )]
    pub fn agent_bootstrap_with_auto_index_and_mode(
        &self,
        query: Option<&str>,
        limit: usize,
        semantic: bool,
        semantic_fail_mode: SemanticFailMode,
        privacy_mode: PrivacyMode,
        max_chars: usize,
        max_tokens: usize,
        auto_index: bool,
    ) -> Result<AgentBootstrap> {
        self.agent_bootstrap_with_auto_index_and_options(
            query,
            limit,
            semantic,
            semantic_fail_mode,
            privacy_mode,
            max_chars,
            max_tokens,
            auto_index,
            AgentBootstrapIncludeOptions::default(),
        )
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
