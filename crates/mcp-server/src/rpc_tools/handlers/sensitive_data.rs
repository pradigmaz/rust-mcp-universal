use anyhow::Result;
use serde_json::Value;

use rmu_core::{
    Engine, MigrationMode, PrivacyMode, SensitiveDataOptions, sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::errors::tool_domain_error;
use crate::rpc_tools::parsing::{
    parse_optional_bool, parse_optional_non_empty_string, parse_optional_usize_with_min,
    reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{parse_optional_migration_mode, parse_optional_privacy_mode};

pub(super) fn sensitive_data(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "sensitive_data",
        &[
            "limit",
            "path_prefix",
            "include_low_confidence",
            "privacy_mode",
            "migration_mode",
        ],
    )?;
    let limit = parse_optional_usize_with_min(args, "sensitive_data", "limit", 1, 20)?;
    let path_prefix = parse_optional_non_empty_string(args, "sensitive_data", "path_prefix")?
        .map(|value| value.replace('\\', "/"));
    let include_low_confidence =
        parse_optional_bool(args, "sensitive_data", "include_low_confidence")?.unwrap_or(false);
    let privacy_mode = parse_optional_privacy_mode(args, "sensitive_data", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let migration_mode = parse_optional_migration_mode(args, "sensitive_data", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);

    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )
    .map_err(|err| tool_domain_error(err.to_string()))?;

    let result = engine
        .sensitive_data(&SensitiveDataOptions {
            path_prefix,
            limit,
            include_low_confidence,
        })
        .map_err(|err| tool_domain_error(err.to_string()))?;
    let mut payload = serde_json::to_value(result)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
