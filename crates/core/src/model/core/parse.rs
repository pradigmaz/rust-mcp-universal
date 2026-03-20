use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use super::{
    ContextMode, IndexProfile, MigrationMode, PrivacyMode, RolloutPhase, SemanticFailMode,
};

pub(super) fn semantic_fail_mode(value: &str) -> Option<SemanticFailMode> {
    match value.trim() {
        "fail_open" => Some(SemanticFailMode::FailOpen),
        "fail_closed" => Some(SemanticFailMode::FailClosed),
        _ => None,
    }
}

pub(super) fn privacy_mode(value: &str) -> Option<PrivacyMode> {
    match value.trim() {
        "off" => Some(PrivacyMode::Off),
        "mask" => Some(PrivacyMode::Mask),
        "hash" => Some(PrivacyMode::Hash),
        _ => None,
    }
}

pub(super) fn context_mode(value: &str) -> Option<ContextMode> {
    match value.trim() {
        "code" => Some(ContextMode::Code),
        "design" => Some(ContextMode::Design),
        "bugfix" => Some(ContextMode::Bugfix),
        _ => None,
    }
}

pub(super) fn rollout_phase(value: &str) -> Option<RolloutPhase> {
    match value.trim() {
        "shadow" => Some(RolloutPhase::Shadow),
        "canary_5" => Some(RolloutPhase::Canary5),
        "canary_25" => Some(RolloutPhase::Canary25),
        "full_100" => Some(RolloutPhase::Full100),
        _ => None,
    }
}

pub(super) fn migration_mode(value: &str) -> Option<MigrationMode> {
    match value.trim() {
        "auto" => Some(MigrationMode::Auto),
        "off" => Some(MigrationMode::Off),
        _ => None,
    }
}

pub(super) fn index_profile(value: &str) -> Option<IndexProfile> {
    match value.trim() {
        "rust-monorepo" => Some(IndexProfile::RustMonorepo),
        "mixed" => Some(IndexProfile::Mixed),
        "docs-heavy" => Some(IndexProfile::DocsHeavy),
        _ => None,
    }
}

pub(super) fn changed_since(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value.trim(), &Rfc3339)
        .ok()
        .map(|value| value.to_offset(UtcOffset::UTC))
}
