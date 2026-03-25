use rmu_core::InvestigationCaseLabels;
use serde_json::Value;

#[derive(Default)]
pub(super) struct ClusterMetricSnapshot {
    pub(super) top_variant_match: Option<bool>,
    pub(super) semantic_state_present: Option<bool>,
    pub(super) semantic_state_matches_expectation: Option<bool>,
    pub(super) semantic_fail_open_visible: Option<bool>,
    pub(super) low_signal_semantic_false_penalty: Option<bool>,
}

pub(super) fn evaluate_cluster_metrics(
    labels: &InvestigationCaseLabels,
    variants: &[Value],
) -> ClusterMetricSnapshot {
    let Some(top_variant) = variants.first() else {
        return ClusterMetricSnapshot {
            top_variant_match: labels
                .expected_top_variant_entry_path
                .as_ref()
                .map(|_| false),
            semantic_state_present: semantic_state_required(labels).then_some(false),
            semantic_state_matches_expectation: labels
                .expected_semantic_state
                .as_ref()
                .map(|_| false),
            semantic_fail_open_visible: labels.semantic_fail_open_case.then_some(false),
            low_signal_semantic_false_penalty: labels.low_signal_semantic_case.then_some(true),
        };
    };
    let semantic_state = top_variant["semantic_state"].as_str();
    let gaps = top_variant["gaps"].as_array().cloned().unwrap_or_default();
    let has_fail_open_gap = gaps
        .iter()
        .filter_map(Value::as_str)
        .any(|gap| gap == "semantic_unavailable_fail_open");
    ClusterMetricSnapshot {
        top_variant_match: labels
            .expected_top_variant_entry_path
            .as_ref()
            .map(|path| top_variant["entry_anchor"]["path"].as_str() == Some(path.as_str())),
        semantic_state_present: semantic_state_required(labels).then_some(semantic_state.is_some()),
        semantic_state_matches_expectation: labels
            .expected_semantic_state
            .as_ref()
            .map(|state| semantic_state == Some(state.as_str())),
        semantic_fail_open_visible: labels
            .semantic_fail_open_case
            .then_some(semantic_state == Some("unavailable_fail_open") && has_fail_open_gap),
        low_signal_semantic_false_penalty: labels
            .low_signal_semantic_case
            .then_some(semantic_state != Some("disabled_low_signal") || has_fail_open_gap),
    }
}

fn semantic_state_required(labels: &InvestigationCaseLabels) -> bool {
    labels.expected_semantic_state.is_some()
        || labels.low_signal_semantic_case
        || labels.semantic_fail_open_case
}
