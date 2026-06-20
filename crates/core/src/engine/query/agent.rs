use std::time::Instant;

use anyhow::Result;

use crate::engine_quality::load_quality_summary;
use crate::model::{
    AgentBootstrap, AgentBootstrapIncludeOptions, AgentBootstrapTimings, AgentIntentMode,
    BootstrapProfile, PrivacyMode, SemanticFailMode, WorkspaceBrief,
};
use crate::report::helpers::degradation;

use super::super::Engine;
use super::agent_query::{AgentQueryBuildInput, build_agent_query_bundle};

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
        reason = "public compatibility for MCP callers"
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
        reason = "public compatibility for MCP callers"
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
        agent_intent_mode: Option<AgentIntentMode>,
        include: AgentBootstrapIncludeOptions,
    ) -> Result<AgentBootstrap> {
        let started = Instant::now();
        let effective_profile = effective_bootstrap_profile(include);
        let (include_report, include_investigation_summary) =
            profile_surface_flags(effective_profile);
        let normalized_query = query
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string);
        let query_requested = normalized_query.is_some();
        let mut timings = AgentBootstrapTimings::default();
        let mut bootstrap_degradation_reasons = Vec::new();

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
            .map(|value| {
                let (bundle, degradation_reasons) =
                    build_agent_query_bundle(AgentQueryBuildInput {
                        engine: self,
                        query: value,
                        limit,
                        semantic,
                        semantic_fail_mode,
                        privacy_mode,
                        max_chars,
                        max_tokens,
                        agent_intent_mode,
                        effective_profile,
                        include_report,
                        include_investigation_summary,
                        index_status: &brief.index_status,
                        timings: &mut timings,
                    })?;
                bootstrap_degradation_reasons = degradation_reasons;
                Ok::<_, anyhow::Error>(bundle)
            })
            .transpose()?;

        let degradation_reasons = bootstrap_degradation_reasons;

        let deepen_available =
            degradation::deepen_available(Some(effective_profile), &degradation_reasons);
        let deepen_hint = degradation::deepen_hint(Some(effective_profile), &degradation_reasons);

        timings.total_ms = elapsed_ms(started);
        Ok(AgentBootstrap {
            brief,
            profile: effective_profile,
            degradation_reasons,
            deepen_available,
            deepen_hint,
            query_bundle,
            timings,
        })
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "public compatibility for MCP callers"
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
            None,
            AgentBootstrapIncludeOptions::default(),
        )
    }
}

pub(super) fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn effective_bootstrap_profile(include: AgentBootstrapIncludeOptions) -> BootstrapProfile {
    if let Some(profile) = include.profile {
        return profile;
    }
    match (
        include.include_report,
        include.include_investigation_summary,
    ) {
        (true, true) => BootstrapProfile::Full,
        (true, false) => BootstrapProfile::Report,
        (false, true) => BootstrapProfile::InvestigationSummary,
        (false, false) => BootstrapProfile::Fast,
    }
}

fn profile_surface_flags(profile: BootstrapProfile) -> (bool, bool) {
    match profile {
        BootstrapProfile::Fast => (false, false),
        BootstrapProfile::InvestigationSummary => (false, true),
        BootstrapProfile::Report => (true, false),
        BootstrapProfile::Full => (true, true),
    }
}
