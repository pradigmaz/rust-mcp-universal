use anyhow::{Result, anyhow};
use serde_json::Value;

use rmu_core::{
    Engine, FindingFamily, MigrationMode, PrivacyMode, SignalMemoryDecision,
    SignalMemoryMarkRequest, SignalMemoryOptions, sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::errors::{invalid_params_error, tool_domain_error};
use crate::rpc_tools::parsing::{
    parse_optional_non_empty_string, parse_optional_usize_with_min, reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{parse_optional_migration_mode, parse_optional_privacy_mode};

pub(super) fn signal_memory(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "signal_memory",
        &[
            "limit",
            "finding_family",
            "decision",
            "privacy_mode",
            "migration_mode",
        ],
    )?;
    let limit = parse_optional_usize_with_min(args, "signal_memory", "limit", 1, 20)?;
    let finding_family = parse_optional_non_empty_string(args, "signal_memory", "finding_family")?
        .map(|value| parse_finding_family(&value))
        .transpose()?;
    let decision = parse_optional_non_empty_string(args, "signal_memory", "decision")?
        .map(|value| parse_decision(&value))
        .transpose()?;
    let privacy_mode = parse_optional_privacy_mode(args, "signal_memory", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let migration_mode = parse_optional_migration_mode(args, "signal_memory", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);
    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )
    .map_err(|err| tool_domain_error(err.to_string()))?;
    let result = engine
        .signal_memory(&SignalMemoryOptions {
            limit,
            finding_family,
            decision,
        })
        .map_err(|err| tool_domain_error(err.to_string()))?;
    let mut payload = serde_json::to_value(result)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

pub(super) fn mark_signal_memory(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "mark_signal_memory",
        &[
            "signal_key",
            "finding_family",
            "scope",
            "decision",
            "reason",
            "source",
            "privacy_mode",
            "migration_mode",
        ],
    )?;
    let signal_key = parse_optional_non_empty_string(args, "mark_signal_memory", "signal_key")?
        .ok_or_else(|| invalid_params_error("mark_signal_memory `signal_key` is required"))?;
    let finding_family =
        parse_optional_non_empty_string(args, "mark_signal_memory", "finding_family")?
            .ok_or_else(|| invalid_params_error("mark_signal_memory `finding_family` is required"))
            .and_then(|value| parse_finding_family(&value))?;
    let scope = parse_optional_non_empty_string(args, "mark_signal_memory", "scope")?;
    let decision = parse_optional_non_empty_string(args, "mark_signal_memory", "decision")?
        .ok_or_else(|| invalid_params_error("mark_signal_memory `decision` is required"))
        .and_then(|value| parse_decision(&value))?;
    let reason = parse_optional_non_empty_string(args, "mark_signal_memory", "reason")?
        .ok_or_else(|| invalid_params_error("mark_signal_memory `reason` is required"))?;
    let source = parse_optional_non_empty_string(args, "mark_signal_memory", "source")?
        .unwrap_or_else(|| "manual".to_string());
    let privacy_mode = parse_optional_privacy_mode(args, "mark_signal_memory", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let migration_mode =
        parse_optional_migration_mode(args, "mark_signal_memory", "migration_mode")?
            .unwrap_or(MigrationMode::Auto);
    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )
    .map_err(|err| tool_domain_error(err.to_string()))?;
    let result = engine
        .mark_signal_memory(&SignalMemoryMarkRequest {
            signal_key,
            finding_family,
            scope,
            decision,
            reason,
            source,
        })
        .map_err(|err| tool_domain_error(err.to_string()))?;
    let mut payload = serde_json::to_value(result)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

fn parse_finding_family(raw: &str) -> Result<FindingFamily> {
    FindingFamily::parse(raw)
        .ok_or_else(|| anyhow!("unsupported finding_family `{raw}`"))
        .map_err(|err| invalid_params_error(&err.to_string()))
}

fn parse_decision(raw: &str) -> Result<SignalMemoryDecision> {
    SignalMemoryDecision::parse(raw)
        .ok_or_else(|| anyhow!("unsupported decision `{raw}`"))
        .map_err(|err| invalid_params_error(&err.to_string()))
}
