use anyhow::Result;
use serde_json::Value;

use rmu_core::{
    Engine, MigrationMode, PrivacyMode, RuleViolationsOptions, RuleViolationsSortBy,
    sanitize_value_for_privacy,
};

use crate::ServerState;
use crate::rpc_tools::errors::{invalid_params_error, tool_domain_error};
use crate::rpc_tools::parsing::{
    parse_optional_non_empty_string, parse_optional_string_list, parse_optional_usize_with_min,
    reject_unknown_fields,
};
use crate::rpc_tools::result::tool_result;

use super::{ensure_query_index_ready, parse_optional_migration_mode, parse_optional_privacy_mode};

pub(super) fn rule_violations(args: &Value, state: &mut ServerState) -> Result<Value> {
    reject_unknown_fields(
        args,
        "rule_violations",
        &[
            "limit",
            "path_prefix",
            "language",
            "rule_ids",
            "metric_ids",
            "sort_metric_id",
            "sort_by",
            "auto_index",
            "privacy_mode",
            "migration_mode",
        ],
    )?;
    let limit = parse_optional_usize_with_min(args, "rule_violations", "limit", 1, 20)?;
    let path_prefix = parse_optional_non_empty_string(args, "rule_violations", "path_prefix")?
        .map(|value| value.replace('\\', "/"));
    let language = parse_optional_non_empty_string(args, "rule_violations", "language")?;
    let rule_ids =
        parse_optional_string_list(args, "rule_violations", "rule_ids")?.unwrap_or_default();
    let metric_ids =
        parse_optional_string_list(args, "rule_violations", "metric_ids")?.unwrap_or_default();
    let sort_metric_id =
        parse_optional_non_empty_string(args, "rule_violations", "sort_metric_id")?;
    let sort_by = parse_optional_non_empty_string(args, "rule_violations", "sort_by")?
        .map(|raw| {
            RuleViolationsSortBy::parse(&raw).ok_or_else(|| {
                invalid_params_error(
                    "rule_violations `sort_by` must be one of: violation_count, size_bytes, non_empty_lines, metric_value",
                )
            })
        })
        .transpose()?
        .unwrap_or(RuleViolationsSortBy::ViolationCount);
    if matches!(sort_by, RuleViolationsSortBy::MetricValue)
        && sort_metric_id.is_none()
        && metric_ids.is_empty()
    {
        return Err(invalid_params_error(
            "rule_violations `metric_value` sorting requires `sort_metric_id` or at least one `metric_ids` entry",
        ));
    }
    let auto_index =
        crate::rpc_tools::parsing::parse_optional_bool(args, "rule_violations", "auto_index")?
            .unwrap_or(false);
    let privacy_mode = parse_optional_privacy_mode(args, "rule_violations", "privacy_mode")?
        .unwrap_or(PrivacyMode::Off);
    let migration_mode = parse_optional_migration_mode(args, "rule_violations", "migration_mode")?
        .unwrap_or(MigrationMode::Auto);

    let engine = Engine::new_with_migration_mode(
        state.project_path.clone(),
        state.db_path.clone(),
        migration_mode,
    )
    .map_err(|err| tool_domain_error(err.to_string()))?;

    if auto_index {
        ensure_query_index_ready(&engine, true)
            .map_err(|err| tool_domain_error(err.to_string()))?;
        engine
            .refresh_quality_if_needed()
            .map_err(|err| tool_domain_error(err.to_string()))?;
    } else if !engine.db_path.exists() {
        return Err(tool_domain_error(
            "index is empty; run an indexing flow or enable automatic indexing before requesting rule violations",
        ));
    }

    let result = engine
        .rule_violations(&RuleViolationsOptions {
            limit,
            path_prefix,
            language,
            rule_ids,
            metric_ids,
            sort_metric_id,
            sort_by,
        })
        .map_err(|err| tool_domain_error(err.to_string()))?;
    let mut payload = serde_json::to_value(result)?;
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}
