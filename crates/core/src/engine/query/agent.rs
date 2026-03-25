use anyhow::Result;

use crate::engine_brief::index_not_ready_error;
use crate::engine_quality::load_quality_summary;
use crate::model::{
    AgentBootstrap, AgentQueryBundle, IndexTelemetry, PrivacyMode, QueryOptions, SemanticFailMode,
    WorkspaceBrief,
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
        let normalized_query = query
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);

        let query_requested = normalized_query.is_some();
        if query_requested {
            let _ = self.ensure_index_ready_with_policy(auto_index)?;
        }

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
                let execution = self.search_with_meta(&options)?;
                let context = self.context_for_hits_with_chunks(
                    value,
                    &execution.hits,
                    Some(&execution.chunk_by_path),
                    None,
                    max_chars,
                    max_tokens,
                )?;
                let (chunk_coverage, chunk_source) = super::derive_chunk_telemetry(&context);
                let investigation_summary =
                    super::investigation_embed::build_investigation_summary(
                        self,
                        value,
                        requested_limit,
                    )?;
                let report = build_query_report(
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
                            last_embedding_cache_hits: brief.index_status.last_embedding_cache_hits,
                            last_embedding_cache_misses: brief
                                .index_status
                                .last_embedding_cache_misses,
                            chunk_coverage,
                            chunk_source,
                        },
                        investigation_summary: Some(investigation_summary),
                    },
                )?;

                Ok(AgentQueryBundle {
                    query: value.to_string(),
                    limit: requested_limit,
                    semantic,
                    max_chars,
                    max_tokens,
                    hits: execution.hits,
                    context,
                    report,
                })
            })
            .transpose()?;

        Ok(AgentBootstrap {
            brief,
            query_bundle,
        })
    }
}
