use anyhow::Result;
use serde_json::{Map, Value, json};

use crate::ServerState;
use crate::rpc_tools::parsing::reject_unknown_fields;

const FACADE_FIELDS: &[&str] = &[
    "limit",
    "path_prefix",
    "language",
    "auto_index",
    "privacy_mode",
    "migration_mode",
    "details",
];

pub(crate) fn api_surface(args: &Value, state: &mut ServerState) -> Result<Value> {
    quality_facade(
        args,
        state,
        "api_surface",
        &[
            "wide_public_api_surface",
            "public_reexport_hub",
            "public_api_hub",
            "unstable_public_hub",
            "public_api_restricted_export_count_metric",
            "public_api_type_count_metric",
            "public_api_function_count_metric",
        ],
        &[
            "public_api_export_count",
            "public_api_reexport_count",
            "public_api_hub_score",
            "public_api_restricted_export_count",
            "public_api_type_count",
            "public_api_function_count",
        ],
        "public_api_export_count",
    )
}

pub(crate) fn complexity_report(args: &Value, state: &mut ServerState) -> Result<Value> {
    quality_facade(
        args,
        state,
        "complexity_report",
        &[
            "max_cyclomatic_complexity",
            "max_cognitive_complexity",
            "max_branch_count",
            "max_early_return_count",
        ],
        &[
            "max_cyclomatic_complexity",
            "max_cognitive_complexity",
            "max_branch_count",
            "max_early_return_count",
        ],
        "max_cognitive_complexity",
    )
}

pub(crate) fn dead_code_report(args: &Value, state: &mut ServerState) -> Result<Value> {
    quality_facade(
        args,
        state,
        "dead_code_report",
        &["dead_code_unused_export_candidate"],
        &["dead_code_exported_symbol_count"],
        "dead_code_exported_symbol_count",
    )
}

pub(crate) fn mark_signal_memory(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::signal_memory::mark_signal_memory(args, state)
}

pub(crate) fn quality_hotspots(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::quality_hotspots::quality_hotspots(args, state)
}

pub(crate) fn quality_snapshot(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::quality_snapshot::quality_snapshot(args, state)
}

pub(crate) fn rule_violations(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::rule_violations::rule_violations(args, state)
}

pub(crate) fn sensitive_data(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::sensitive_data::sensitive_data(args, state)
}

pub(crate) fn signal_memory(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::signal_memory::signal_memory(args, state)
}

fn quality_facade(
    args: &Value,
    state: &mut ServerState,
    tool_name: &str,
    rule_ids: &[&str],
    metric_ids: &[&str],
    sort_metric_id: &str,
) -> Result<Value> {
    reject_unknown_fields(args, tool_name, FACADE_FIELDS)?;
    let mut forwarded = args.as_object().cloned().unwrap_or_else(Map::new);
    forwarded.insert("rule_ids".to_string(), json!(rule_ids));
    forwarded.insert("metric_ids".to_string(), json!(metric_ids));
    forwarded.insert("sort_by".to_string(), json!("metric_value"));
    forwarded.insert("sort_metric_id".to_string(), json!(sort_metric_id));
    super::rule_violations::rule_violations(&Value::Object(forwarded), state)
}
