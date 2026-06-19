use anyhow::Result;
use rmu_core::Engine;
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
    if requested_auto_index(args) && existing_index_db_exists(state) {
        forwarded.insert("auto_index".to_string(), json!(false));
    }
    let mut result = super::rule_violations::rule_violations(&Value::Object(forwarded), state)?;
    keep_only_facade_findings(&mut result);
    Ok(result)
}

fn requested_auto_index(args: &Value) -> bool {
    args.get("auto_index")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn existing_index_db_exists(state: &ServerState) -> bool {
    if let Some(path) = &state.db_path {
        return path.exists();
    }
    Engine::new_read_only(state.project_path.clone(), None)
        .map(|engine| engine.db_path.exists())
        .unwrap_or(false)
}

fn keep_only_facade_findings(result: &mut Value) {
    let hit_count = {
        let Some(hits) = result
            .pointer_mut("/structuredContent/hits")
            .and_then(Value::as_array_mut)
        else {
            return;
        };
        hits.retain(hit_has_violation);
        hits.len()
    };

    if let Some(text) = result
        .get_mut("content")
        .and_then(Value::as_array_mut)
        .and_then(|content| content.first_mut())
        .and_then(Value::as_object_mut)
        .and_then(|item| item.get_mut("text"))
    {
        *text = Value::String(format!("ok: hits={hit_count}"));
    }
}

fn hit_has_violation(hit: &Value) -> bool {
    hit.get("violation_count")
        .and_then(Value::as_u64)
        .is_some_and(|count| count > 0)
        || hit
            .get("violations")
            .and_then(Value::as_array)
            .is_some_and(|violations| !violations.is_empty())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::keep_only_facade_findings;

    #[test]
    fn facade_filter_removes_zero_violation_hits() {
        for (mut result, expected_hits) in [
            (
                json!({
                    "content": [{"type": "text", "text": "ok: hits=2"}],
                    "structuredContent": {"hits": [
                        {"path": "src/noise.rs", "violation_count": 0},
                        {"path": "src/finding.rs", "violation_count": 1}
                    ]},
                    "isError": false
                }),
                json!([{"path": "src/finding.rs", "violation_count": 1}]),
            ),
            (
                json!({
                    "content": [{"type": "text", "text": "ok: hits=2"}],
                    "structuredContent": {"hits": [
                        {"path": "src/noise.rs", "violations": []},
                        {"path": "src/finding.rs", "violations": [{"rule_id": "x"}]}
                    ]},
                    "isError": false
                }),
                json!([{"path": "src/finding.rs", "violations": [{"rule_id": "x"}]}]),
            ),
        ] {
            keep_only_facade_findings(&mut result);
            assert_eq!(result["structuredContent"]["hits"], expected_hits);
            assert_eq!(result["content"][0]["text"], json!("ok: hits=1"));
        }
    }
}
