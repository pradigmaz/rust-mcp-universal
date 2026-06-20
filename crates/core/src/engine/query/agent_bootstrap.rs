use std::time::Instant;

use anyhow::Result;

use crate::engine::Engine;
use crate::engine_quality::load_quality_summary;
use crate::model::{
    AgentBootstrap, AgentBootstrapIncludeOptions, AgentBootstrapTimings, AgentIntentMode,
    BootstrapProfile, PrivacyMode, SemanticFailMode, WorkspaceBrief,
};
use crate::report::helpers::degradation;

use super::agent_query::{AgentQueryBuildInput, build_agent_query_bundle};

pub(super) struct AgentBootstrapRequest<'a> {
    pub(super) query: Option<&'a str>,
    pub(super) limit: usize,
    pub(super) semantic: bool,
    pub(super) semantic_fail_mode: SemanticFailMode,
    pub(super) privacy_mode: PrivacyMode,
    pub(super) max_chars: usize,
    pub(super) max_tokens: usize,
    pub(super) auto_index: bool,
    pub(super) agent_intent_mode: Option<AgentIntentMode>,
    pub(super) include: AgentBootstrapIncludeOptions,
}

pub(super) fn build_agent_bootstrap(
    engine: &Engine,
    request: AgentBootstrapRequest<'_>,
) -> Result<AgentBootstrap> {
    let started = Instant::now();
    let effective_profile = effective_bootstrap_profile(request.include);
    let (include_report, include_investigation_summary) = profile_surface_flags(effective_profile);
    let normalized_query = request
        .query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let query_requested = normalized_query.is_some();
    let mut timings = AgentBootstrapTimings::default();
    let mut bootstrap_degradation_reasons = Vec::new();

    if query_requested {
        let phase_started = Instant::now();
        let _ = engine.ensure_index_ready_with_policy(request.auto_index)?;
        timings.index_ready_ms = elapsed_ms(phase_started);
    }

    let phase_started = Instant::now();
    let brief = if request.auto_index || query_requested {
        engine.workspace_brief_with_policy(request.auto_index)?
    } else {
        let status = engine.index_status()?;
        WorkspaceBrief {
            auto_indexed: false,
            index_status: status.clone(),
            languages: super::brief::load_top_languages_for_brief(engine)?,
            top_symbols: super::brief::load_top_symbols_for_brief(engine)?,
            quality_summary: load_quality_summary(engine)?,
            recommendations: super::brief::make_brief_recommendations(&status),
            repair_hint: None,
        }
    };
    timings.brief_ms = elapsed_ms(phase_started);

    let query_bundle = normalized_query
        .as_deref()
        .map(|value| {
            let (bundle, degradation_reasons) = build_agent_query_bundle(AgentQueryBuildInput {
                engine,
                query: value,
                limit: request.limit,
                semantic: request.semantic,
                semantic_fail_mode: request.semantic_fail_mode,
                privacy_mode: request.privacy_mode,
                max_chars: request.max_chars,
                max_tokens: request.max_tokens,
                agent_intent_mode: request.agent_intent_mode,
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
