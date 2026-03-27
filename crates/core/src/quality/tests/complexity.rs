use crate::quality::{
    IndexedQualityMetrics, QualityThresholds, build_indexed_quality_facts, default_quality_policy,
    evaluate_quality,
};

#[test]
fn complexity_metrics_are_recorded_but_only_threshold_metrics_violate() {
    let facts = build_indexed_quality_facts(
        "src/lib.rs",
        "rust",
        256,
        Some(1),
        "pub fn noisy(flag: bool, level: i32) -> i32 {\n    if flag {\n        if level > 0 {\n            return 1;\n        }\n    }\n    if level < 0 {\n        return -1;\n    }\n    return 0;\n}\n",
    );
    let mut policy = default_quality_policy();
    policy.thresholds = QualityThresholds {
        max_cyclomatic_complexity: 2,
        max_cognitive_complexity: 2,
        ..policy.thresholds
    };
    let evaluation = evaluate_quality(&facts, &IndexedQualityMetrics::default(), &policy);
    let metric_ids = evaluation
        .snapshot
        .metrics
        .iter()
        .map(|entry| entry.metric_id.as_str())
        .collect::<Vec<_>>();
    let violation_ids = evaluation
        .snapshot
        .violations
        .iter()
        .map(|entry| entry.rule_id.as_str())
        .collect::<Vec<_>>();

    assert!(metric_ids.contains(&"max_cyclomatic_complexity"));
    assert!(metric_ids.contains(&"max_cognitive_complexity"));
    assert!(metric_ids.contains(&"max_branch_count"));
    assert!(metric_ids.contains(&"max_early_return_count"));
    assert!(violation_ids.contains(&"max_cyclomatic_complexity"));
    assert!(violation_ids.contains(&"max_cognitive_complexity"));
    assert!(!violation_ids.contains(&"max_branch_count"));
    assert!(!violation_ids.contains(&"max_early_return_count"));
}

#[test]
fn java_complexity_handles_crlf_and_unicode_without_panicking() {
    let facts = build_indexed_quality_facts(
        "src/Main.java",
        "java",
        256,
        Some(1),
        "package demo;\r\n\r\npublic class Main {\r\n    public int noisy(boolean flag, int level) {\r\n        String note = \"ч\";\r\n        if (flag) {\r\n            if (level > 0) {\r\n                return 1;\r\n            }\r\n        }\r\n        return 0;\r\n    }\r\n}\r\n",
    );
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
            .any(|entry| entry.metric_id == "max_cyclomatic_complexity")
    );
}

#[test]
fn javascript_complexity_handles_expression_arrow_bodies_without_panicking() {
    let facts = build_indexed_quality_facts(
        "src/app.tsx",
        "tsx",
        256,
        Some(1),
        "export const render = (flag: boolean) => flag && [1, 2].map(value => value ? value : 0);\n",
    );
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
            .any(|entry| entry.metric_id == "max_cyclomatic_complexity")
    );
}

#[test]
fn python_complexity_emits_all_metrics_for_nested_branches() {
    let facts = build_indexed_quality_facts(
        "src/tasks.py",
        "python",
        256,
        Some(1),
        "def noisy(flag, level):\n    if flag:\n        if level > 0:\n            return 1\n    if level < 0:\n        return -1\n    return 0\n",
    );
    let evaluation = evaluate_quality(
        &facts,
        &IndexedQualityMetrics::default(),
        &default_quality_policy(),
    );
    let metric_ids = evaluation
        .snapshot
        .metrics
        .iter()
        .map(|entry| entry.metric_id.as_str())
        .collect::<Vec<_>>();

    assert!(metric_ids.contains(&"max_cyclomatic_complexity"));
    assert!(metric_ids.contains(&"max_cognitive_complexity"));
    assert!(metric_ids.contains(&"max_branch_count"));
    assert!(metric_ids.contains(&"max_early_return_count"));
}

#[test]
fn javascript_complexity_emits_all_metrics_for_nested_branches() {
    let facts = build_indexed_quality_facts(
        "src/app.ts",
        "typescript",
        256,
        Some(1),
        "export function noisy(flag: boolean, level: number) {\n  if (flag) {\n    if (level > 0) {\n      return 1;\n    }\n  }\n  if (level < 0) {\n    return -1;\n  }\n  return 0;\n}\n",
    );
    let evaluation = evaluate_quality(
        &facts,
        &IndexedQualityMetrics::default(),
        &default_quality_policy(),
    );
    let metric_ids = evaluation
        .snapshot
        .metrics
        .iter()
        .map(|entry| entry.metric_id.as_str())
        .collect::<Vec<_>>();

    assert!(metric_ids.contains(&"max_cyclomatic_complexity"));
    assert!(metric_ids.contains(&"max_cognitive_complexity"));
    assert!(metric_ids.contains(&"max_branch_count"));
    assert!(metric_ids.contains(&"max_early_return_count"));
}
