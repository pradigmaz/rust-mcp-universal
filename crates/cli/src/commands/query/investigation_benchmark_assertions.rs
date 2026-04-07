use crate::commands::query::investigation_benchmark_eval::run_tool;
use rmu_core::{
    Engine, InvestigationAssertion, InvestigationBenchmarkCase, InvestigationBenchmarkTool,
    sanitize_value_for_privacy,
};
use serde_json::Value;

pub(super) fn evaluate_assertion(payload: &Value, assertion: &InvestigationAssertion) -> bool {
    match assertion.kind.as_str() {
        "body_anchor_present" => {
            payload["items"]
                .as_array()
                .is_some_and(|items| items.iter().any(|item| item.get("anchor").is_some()))
                || payload["variants"].as_array().is_some_and(|variants| {
                    variants
                        .iter()
                        .any(|variant| !variant["body_anchor"].is_null())
                })
        }
        "route_kind_present" | "contains_route_kind" => collect_route_strings(payload, "kind")
            .iter()
            .any(|value| *value == assertion.value.as_deref().unwrap_or_default()),
        "relation_kind_present" => collect_route_strings(payload, "relation_kind")
            .iter()
            .any(|value| *value == assertion.value.as_deref().unwrap_or_default()),
        "source_span_present" => route_segments(payload)
            .into_iter()
            .any(|segment| !segment["source_span"].is_null()),
        "constraint_kind_present" | "contains_constraint_kind" => {
            collect_constraint_strings(payload, "kind")
                .iter()
                .any(|value| *value == assertion.value.as_deref().unwrap_or_default())
        }
        "contains_language" => collect_languages(payload)
            .iter()
            .any(|value| *value == assertion.value.as_deref().unwrap_or_default()),
        "contains_gap" => collect_gaps(payload)
            .iter()
            .any(|value| *value == assertion.value.as_deref().unwrap_or_default()),
        "min_variant_count" => payload["variants"]
            .as_array()
            .is_some_and(|variants| variants.len() >= parse_usize_assertion(assertion)),
        "min_body_items" => payload["items"]
            .as_array()
            .is_some_and(|items| items.len() >= parse_usize_assertion(assertion)),
        "min_divergence_axes" => payload["divergence_axes"]
            .as_array()
            .is_some_and(|axes| axes.len() >= parse_usize_assertion(assertion)),
        "strong_constraint_present" => {
            collect_constraint_strings(payload, "strength").contains(&"strong")
        }
        "divergence_axis_present" => payload["divergence_axes"].as_array().is_some_and(|axes| {
            axes.iter()
                .any(|axis| axis["axis"].as_str() == assertion.value.as_deref())
        }),
        "divergence_severity_present" | "expected_severity" => payload["divergence_signals"]
            .as_array()
            .is_some_and(|signals| {
                signals
                    .iter()
                    .any(|signal| signal["severity"].as_str() == assertion.value.as_deref())
            }),
        "manual_review_required" => payload["manual_review_required"]
            .as_bool()
            .is_some_and(|value| value == parse_bool_assertion(assertion)),
        "chain_role_present" | "contains_chain_role" => payload["chain"]
            .as_array()
            .is_some_and(|chain| chain.iter().any(|link| link["role"].as_str() == assertion.value.as_deref())),
        "recommended_target_role" => payload["actionability"]["recommended_target_role"]
            .as_str()
            == assertion.value.as_deref(),
        "recommended_target_path" => payload["actionability"]["recommended_target_path"]
            .as_str()
            == assertion.value.as_deref(),
        _ => false,
    }
}

pub(super) fn concept_cluster_rank_consistent(
    engine: &Engine,
    case: &InvestigationBenchmarkCase,
    limit: usize,
    payload: &Value,
) -> Option<bool> {
    if !matches!(case.tool, InvestigationBenchmarkTool::ConceptCluster) {
        return None;
    }
    let rerun_payload = run_tool(engine, case.tool, &case.seed, case.seed_kind, limit).ok()?;
    Some(top_variant_paths(payload) == top_variant_paths(&rerun_payload))
}

pub(super) fn count_privacy_failures(
    engine: &Engine,
    privacy_mode: rmu_core::PrivacyMode,
    payload: &Value,
) -> usize {
    if matches!(privacy_mode, rmu_core::PrivacyMode::Off) {
        return 0;
    }
    let mut sanitized = payload.clone();
    sanitize_value_for_privacy(privacy_mode, &mut sanitized);
    let serialized = serde_json::to_string(&sanitized).unwrap_or_default();
    usize::from(serialized.contains(&engine.project_root.display().to_string()))
}

fn collect_route_strings<'a>(payload: &'a Value, key: &str) -> Vec<&'a str> {
    route_segments(payload)
        .into_iter()
        .filter_map(|entry| entry[key].as_str())
        .collect()
}

fn route_segments(payload: &Value) -> Vec<&Value> {
    if payload.get("best_route").is_some() {
        payload["best_route"]["segments"]
            .as_array()
            .into_iter()
            .flatten()
            .chain(
                payload["alternate_routes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .flat_map(|route| route["segments"].as_array().into_iter().flatten()),
            )
            .collect()
    } else {
        payload["variants"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|item| item["route"].as_array())
            .flatten()
            .collect()
    }
}

fn collect_constraint_strings<'a>(payload: &'a Value, key: &str) -> Vec<&'a str> {
    payload["items"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(
            payload["variants"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|variant| variant["constraints"].as_array())
                .flatten(),
        )
        .filter_map(|entry| entry[key].as_str())
        .collect()
}

fn collect_languages(payload: &Value) -> Vec<&str> {
    payload["items"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|item| item["anchor"]["language"].as_str())
        .chain(
            payload["variants"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|variant| variant["entry_anchor"]["language"].as_str()),
        )
        .collect()
}

fn collect_gaps(payload: &Value) -> Vec<&str> {
    payload["gaps"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .chain(
            payload["unresolved_gaps"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|gap| gap["reason"].as_str()),
        )
        .chain(
            payload["variants"]
                .as_array()
                .into_iter()
                .flatten()
                .flat_map(|variant| variant["gaps"].as_array())
                .flatten()
                .filter_map(Value::as_str),
        )
        .collect()
}

fn parse_usize_assertion(assertion: &InvestigationAssertion) -> usize {
    assertion
        .value
        .as_deref()
        .unwrap_or("0")
        .parse::<usize>()
        .unwrap_or(0)
}

fn parse_bool_assertion(assertion: &InvestigationAssertion) -> bool {
    matches!(
        assertion.value.as_deref().unwrap_or_default().trim(),
        "true" | "True" | "TRUE" | "1"
    )
}

fn top_variant_paths(payload: &Value) -> Vec<String> {
    payload["variants"]
        .as_array()
        .into_iter()
        .flatten()
        .take(3)
        .filter_map(|variant| variant["entry_anchor"]["path"].as_str())
        .map(ToString::to_string)
        .collect()
}
