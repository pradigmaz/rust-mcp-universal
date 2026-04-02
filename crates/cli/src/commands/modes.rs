use anyhow::Result;
use rmu_core::{
    AgentIntentMode, BootstrapProfile, ConceptSeedKind, ContextMode, IgnoreInstallTarget,
    IndexProfile, MigrationMode, PrivacyMode, RolloutPhase, SemanticFailMode,
};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

pub(super) fn parse_semantic_fail_mode(raw: &str) -> Result<SemanticFailMode> {
    SemanticFailMode::parse(raw).ok_or_else(|| {
        anyhow::anyhow!(
            "`semantic_fail_mode` must be one of: fail_open, fail_closed (got `{}`)",
            raw
        )
    })
}

pub(super) fn parse_privacy_mode(raw: &str) -> Result<PrivacyMode> {
    PrivacyMode::parse(raw).ok_or_else(|| {
        anyhow::anyhow!(
            "`privacy_mode` must be one of: off, mask, hash; use `off` for unsanitized output (not `none` or `repo-only`) (got `{}`)",
            raw
        )
    })
}

pub(super) fn parse_context_mode(raw: &str) -> Result<ContextMode> {
    ContextMode::parse(raw).ok_or_else(|| {
        anyhow::anyhow!(
            "`mode` must be one of: code, design, bugfix (got `{}`)",
            raw
        )
    })
}

pub(super) fn parse_agent_intent_mode(raw: &str) -> Result<AgentIntentMode> {
    AgentIntentMode::parse(raw).ok_or_else(|| {
        anyhow::anyhow!(
            "`mode` must be one of: entrypoint_map, test_map, review_prep, api_contract_map, runtime_surface, refactor_surface (got `{}`)",
            raw
        )
    })
}

pub(super) fn parse_bootstrap_profile(raw: &str) -> Result<BootstrapProfile> {
    BootstrapProfile::parse(raw).ok_or_else(|| {
        anyhow::anyhow!(
            "`profile` must be one of: fast, investigation_summary, report, full (got `{}`)",
            raw
        )
    })
}

pub(super) fn parse_seed_kind(raw: &str) -> Result<ConceptSeedKind> {
    ConceptSeedKind::parse(raw).ok_or_else(|| {
        anyhow::anyhow!(
            "`seed_kind` must be one of: query, symbol, path, path_line (got `{}`)",
            raw
        )
    })
}

pub(super) fn parse_rollout_phase(raw: &str) -> Result<RolloutPhase> {
    RolloutPhase::parse(raw).ok_or_else(|| {
        anyhow::anyhow!(
            "`rollout_phase` must be one of: shadow, canary_5, canary_25, full_100 (got `{}`)",
            raw
        )
    })
}

pub(super) fn parse_migration_mode(raw: &str) -> Result<MigrationMode> {
    MigrationMode::parse(raw).ok_or_else(|| {
        anyhow::anyhow!("`migration_mode` must be one of: auto, off (got `{}`)", raw)
    })
}

pub(super) fn parse_index_profile(raw: &str) -> Result<IndexProfile> {
    IndexProfile::parse(raw).ok_or_else(|| {
        anyhow::anyhow!(
            "`profile` must be one of: rust-monorepo, mixed, docs-heavy (got `{}`)",
            raw
        )
    })
}

pub(super) fn parse_ignore_install_target(raw: &str) -> Result<IgnoreInstallTarget> {
    IgnoreInstallTarget::parse(raw).ok_or_else(|| {
        anyhow::anyhow!(
            "`target` must be one of: git-info-exclude, root-gitignore (got `{}`)",
            raw
        )
    })
}

pub(super) fn parse_changed_since(raw: &str) -> Result<OffsetDateTime> {
    OffsetDateTime::parse(raw.trim(), &Rfc3339)
        .map(|value| value.to_offset(UtcOffset::UTC))
        .map_err(|_| {
            anyhow::anyhow!(
                "`changed_since` must be RFC3339 timestamp with timezone (got `{}`)",
                raw
            )
        })
}

pub(super) fn parse_changed_since_commit(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!("`changed_since_commit` must be non-empty");
    }
    Ok(trimmed.to_string())
}

pub(super) fn format_changed_since(value: OffsetDateTime) -> Result<String> {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(Into::into)
}
