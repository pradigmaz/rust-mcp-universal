use anyhow::Result;
use rmu_core::{IgnoreInstallTarget, IndexProfile, MigrationMode, PrivacyMode};
use serde_json::Value;
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Rfc3339};

use crate::rpc_tools::errors::invalid_params_error;

pub(super) fn parse_optional_migration_mode(
    args: &Value,
    tool_name: &str,
) -> Result<Option<MigrationMode>> {
    let Some(raw) = args.get("migration_mode") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `migration_mode` must be string"
        )));
    };
    let parsed = MigrationMode::parse(raw_string).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `migration_mode` must be one of: auto, off"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_privacy_mode(
    args: &Value,
    tool_name: &str,
) -> Result<Option<PrivacyMode>> {
    let Some(raw) = args.get("privacy_mode") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `privacy_mode` must be string"
        )));
    };
    let parsed = PrivacyMode::parse(raw_string).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `privacy_mode` must be one of: off, mask, hash"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_index_profile(
    args: &Value,
    tool_name: &str,
) -> Result<Option<IndexProfile>> {
    let Some(raw) = args.get("profile") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `profile` must be string"
        )));
    };
    let parsed = IndexProfile::parse(raw_string).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `profile` must be one of: rust-monorepo, mixed, docs-heavy"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_ignore_install_target(
    args: &Value,
    tool_name: &str,
) -> Result<Option<IgnoreInstallTarget>> {
    let Some(raw) = args.get("target") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `target` must be string"
        )));
    };
    let parsed = IgnoreInstallTarget::parse(raw_string).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `target` must be one of: git-info-exclude, root-gitignore"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_changed_since(
    args: &Value,
    tool_name: &str,
) -> Result<Option<OffsetDateTime>> {
    let Some(raw) = args.get("changed_since") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `changed_since` must be string"
        )));
    };
    let parsed = OffsetDateTime::parse(raw_string.trim(), &Rfc3339)
        .map(|value| value.to_offset(UtcOffset::UTC))
        .map_err(|_| {
            invalid_params_error(format!(
                "{tool_name} `changed_since` must be RFC3339 timestamp with timezone"
            ))
        })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_changed_since_commit(
    args: &Value,
    tool_name: &str,
) -> Result<Option<String>> {
    let Some(raw) = args.get("changed_since_commit") else {
        return Ok(None);
    };
    let Some(raw_string) = raw.as_str() else {
        return Err(invalid_params_error(format!(
            "{tool_name} `changed_since_commit` must be string"
        )));
    };
    let trimmed = raw_string.trim();
    if trimmed.is_empty() {
        return Err(invalid_params_error(format!(
            "{tool_name} `changed_since_commit` must be non-empty"
        )));
    }
    Ok(Some(trimmed.to_string()))
}

pub(super) fn format_changed_since(value: OffsetDateTime) -> Result<String> {
    value
        .to_offset(UtcOffset::UTC)
        .format(&Rfc3339)
        .map_err(Into::into)
}
