use anyhow::Result;
use serde_json::Value;

use rmu_core::{
    AgentIntentMode, BootstrapProfile, ContextMode, MigrationMode, PrivacyMode, RolloutPhase,
    SemanticFailMode,
};

use crate::rpc_tools::errors::invalid_params_error;
use crate::rpc_tools::parsing::parse_optional_non_empty_string;

pub(super) fn parse_optional_semantic_fail_mode(
    args: &Value,
    tool_name: &str,
    field_name: &str,
) -> Result<Option<SemanticFailMode>> {
    let value = parse_optional_non_empty_string(args, tool_name, field_name)?;
    let Some(raw) = value else {
        return Ok(None);
    };
    let parsed = SemanticFailMode::parse(&raw).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `{field_name}` must be one of: fail_open, fail_closed"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_privacy_mode(
    args: &Value,
    tool_name: &str,
    field_name: &str,
) -> Result<Option<PrivacyMode>> {
    let value = parse_optional_non_empty_string(args, tool_name, field_name)?;
    let Some(raw) = value else {
        return Ok(None);
    };
    let parsed = PrivacyMode::parse(&raw).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `{field_name}` must be one of: off, mask, hash; use `off` for unsanitized output (not `none` or `repo-only`)"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_context_mode(
    args: &Value,
    tool_name: &str,
    field_name: &str,
) -> Result<Option<ContextMode>> {
    let value = parse_optional_non_empty_string(args, tool_name, field_name)?;
    let Some(raw) = value else {
        return Ok(None);
    };
    let parsed = ContextMode::parse(&raw).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `{field_name}` must be one of: code, design, bugfix"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_agent_intent_mode(
    args: &Value,
    tool_name: &str,
    field_name: &str,
) -> Result<Option<AgentIntentMode>> {
    let value = parse_optional_non_empty_string(args, tool_name, field_name)?;
    let Some(raw) = value else {
        return Ok(None);
    };
    let parsed = AgentIntentMode::parse(&raw).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `{field_name}` must be one of: entrypoint_map, test_map, review_prep, api_contract_map, runtime_surface, refactor_surface"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_bootstrap_profile(
    args: &Value,
    tool_name: &str,
    field_name: &str,
) -> Result<Option<BootstrapProfile>> {
    let value = parse_optional_non_empty_string(args, tool_name, field_name)?;
    let Some(raw) = value else {
        return Ok(None);
    };
    let parsed = BootstrapProfile::parse(&raw).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `{field_name}` must be one of: fast, investigation_summary, report, full"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_rollout_phase(
    args: &Value,
    tool_name: &str,
    field_name: &str,
) -> Result<Option<RolloutPhase>> {
    let value = parse_optional_non_empty_string(args, tool_name, field_name)?;
    let Some(raw) = value else {
        return Ok(None);
    };
    let parsed = RolloutPhase::parse(&raw).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `{field_name}` must be one of: shadow, canary_5, canary_25, full_100"
        ))
    })?;
    Ok(Some(parsed))
}

pub(super) fn parse_optional_migration_mode(
    args: &Value,
    tool_name: &str,
    field_name: &str,
) -> Result<Option<MigrationMode>> {
    let value = parse_optional_non_empty_string(args, tool_name, field_name)?;
    let Some(raw) = value else {
        return Ok(None);
    };
    let parsed = MigrationMode::parse(&raw).ok_or_else(|| {
        invalid_params_error(format!(
            "{tool_name} `{field_name}` must be one of: auto, off"
        ))
    })?;
    Ok(Some(parsed))
}
