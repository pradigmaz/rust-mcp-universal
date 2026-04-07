use crate::quality::{
    IndexedQualityMetrics, LayeringFacts, StructuralFacts, build_indexed_quality_facts,
    default_quality_policy, evaluate_quality,
};

#[test]
fn structural_rules_emit_metrics_and_violations() {
    let mut facts = build_indexed_quality_facts(
        "src/ui/view.ts",
        "typescript",
        128,
        Some(1),
        "export function view() { return 1; }\n",
    );
    facts.structural = StructuralFacts {
        fan_in_count: Some(25),
        fan_out_count: Some(24),
        cycle_member: true,
        orphan_module: false,
    };
    facts.layering = LayeringFacts {
        zone_id: Some("ui".to_string()),
        forbidden_edge_count: 2,
        primary_message: Some("zone `ui` depends on forbidden zone `data`".to_string()),
        ..LayeringFacts::default()
    };

    let evaluation = evaluate_quality(
        &facts,
        &IndexedQualityMetrics::default(),
        &default_quality_policy(),
    );

    assert!(
        evaluation
            .snapshot
            .metrics
            .iter()
            .any(|metric| metric.metric_id == "fan_in_count" && metric.metric_value == 25)
    );
    assert!(
        evaluation
            .snapshot
            .metrics
            .iter()
            .any(|metric| metric.metric_id == "fan_out_count" && metric.metric_value == 24)
    );
    assert!(
        evaluation
            .snapshot
            .violations
            .iter()
            .any(|violation| violation.rule_id == "max_fan_in_per_file")
    );
    assert!(
        evaluation
            .snapshot
            .violations
            .iter()
            .any(|violation| violation.rule_id == "max_fan_out_per_file")
    );
    assert!(
        evaluation
            .snapshot
            .violations
            .iter()
            .any(|violation| violation.rule_id == "hub_module")
    );
    assert!(
        evaluation
            .snapshot
            .violations
            .iter()
            .any(|violation| violation.rule_id == "module_cycle_member")
    );
    assert!(
        evaluation
            .snapshot
            .violations
            .iter()
            .any(|violation| violation.rule_id == "cross_layer_dependency")
    );
    assert!(
        evaluation
            .snapshot
            .metrics
            .iter()
            .any(|metric| metric.metric_id == "layering_forbidden_edge_count"
                && metric.metric_value == 2)
    );
}

#[test]
fn orphan_module_violation_is_policy_derived_and_boolean() {
    let mut facts = build_indexed_quality_facts(
        "src/domain/isolated.ts",
        "typescript",
        64,
        Some(1),
        "export const lonely = 1;\n",
    );
    facts.structural = StructuralFacts {
        orphan_module: true,
        ..StructuralFacts::default()
    };

    let evaluation = evaluate_quality(
        &facts,
        &IndexedQualityMetrics::default(),
        &default_quality_policy(),
    );

    let violation = evaluation
        .snapshot
        .violations
        .iter()
        .find(|violation| violation.rule_id == "orphan_module")
        .expect("orphan module violation should be present");
    assert_eq!(violation.actual_value, 1);
    assert_eq!(violation.threshold_value, 0);
}
