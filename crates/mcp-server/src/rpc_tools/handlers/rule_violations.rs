use anyhow::Result;
use serde_json::{Map, Value};

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
            "details",
        ],
    )?;
    let limit = parse_optional_usize_with_min(args, "rule_violations", "limit", 1, 3)?;
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
                    "rule_violations `sort_by` must be one of: violation_count, size_bytes, non_empty_lines, metric_value; use `path_prefix` to filter paths because `sort_by=path` is not supported",
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
    let details =
        crate::rpc_tools::parsing::parse_optional_bool(args, "rule_violations", "details")?
            .unwrap_or(false);

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
    if !details {
        compact_rule_violations_payload(&mut payload);
    }
    sanitize_value_for_privacy(privacy_mode, &mut payload);
    tool_result(payload)
}

fn compact_rule_violations_payload(payload: &mut Value) {
    let Some(hits) = payload.get_mut("hits").and_then(Value::as_array_mut) else {
        return;
    };
    for hit in hits {
        let Some(object) = hit.as_object_mut() else {
            continue;
        };
        let violation_count = object
            .get("violations")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let metric_count = object
            .get("metrics")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let top_rule_ids = object
            .get("violations")
            .and_then(Value::as_array)
            .map(|violations| {
                violations
                    .iter()
                    .filter_map(|item| item.get("rule_id").and_then(Value::as_str))
                    .take(5)
                    .map(Value::from)
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        object.insert("violation_count".to_string(), Value::from(violation_count));
        object.insert("metric_count".to_string(), Value::from(metric_count));
        object.insert("top_rule_ids".to_string(), Value::Array(top_rule_ids));
        compact_risk_score(object);
        object.remove("violations");
        object.remove("metrics");
        object.remove("signal_key");
    }
}

fn compact_risk_score(object: &mut Map<String, Value>) {
    let Some(score) = object
        .get("risk_score")
        .and_then(|value| value.get("score"))
        .cloned()
    else {
        return;
    };
    object.insert(
        "risk_score".to_string(),
        serde_json::json!({ "score": score }),
    );
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::compact_rule_violations_payload;

    #[test]
    fn compact_rule_violations_removes_heavy_fields() {
        let mut payload = json!({
            "hits": [{
                "path": "src/lib.rs",
                "risk_score": {"score": 12.0, "components": {"size": 1}, "weights": {"size": 1}},
                "metrics": [{"id": "a"}, {"id": "b"}],
                "violations": [
                    {"rule_id": "long_file"},
                    {"rule_id": "many_imports"}
                ],
                "signal_key": "abc"
            }]
        });

        compact_rule_violations_payload(&mut payload);
        let hit = &payload["hits"][0];

        assert_eq!(hit["violation_count"], 2);
        assert_eq!(hit["metric_count"], 2);
        assert_eq!(hit["top_rule_ids"], json!(["long_file", "many_imports"]));
        assert_eq!(hit["risk_score"], json!({"score": 12.0}));
        assert!(hit.get("violations").is_none());
        assert!(hit.get("metrics").is_none());
        assert!(hit.get("signal_key").is_none());
    }
}
