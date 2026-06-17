use crate::model::QualitySource;
use crate::quality::{
    GitRiskFacts, IndexedQualityMetrics, StructuralFacts, build_indexed_quality_facts,
    default_quality_policy, evaluate_quality,
};

#[test]
fn api_surface_analyzer_counts_rust_public_reexports_and_restricted_exports() {
    let facts = build_indexed_quality_facts(
        "src/lib.rs",
        "rust",
        128,
        Some(1),
        "pub use crate::domain::Thing;\npub(crate) fn internal() {}\npub struct PublicType;\npub async fn public_fn() {}\n",
    );

    assert_eq!(facts.api_surface.public_export_count, 3);
    assert_eq!(facts.api_surface.restricted_export_count, 1);
    assert_eq!(facts.api_surface.public_reexport_count, 1);
    assert_eq!(facts.api_surface.public_type_count, 1);
    assert_eq!(facts.api_surface.public_function_count, 1);
    assert_eq!(
        facts
            .api_surface
            .primary_location
            .as_ref()
            .map(|location| location.start_line),
        Some(1)
    );
}

#[test]
fn api_surface_analyzer_counts_javascript_exports_and_reexports() {
    let facts = build_indexed_quality_facts(
        "src/index.ts",
        "typescript",
        128,
        Some(1),
        "export { A, B } from './a';\nexport * from './b';\nexport default function run() {}\nexport interface Contract {}\n",
    );

    assert_eq!(facts.api_surface.public_export_count, 5);
    assert_eq!(facts.api_surface.public_reexport_count, 3);
    assert_eq!(facts.api_surface.public_type_count, 1);
    assert_eq!(facts.api_surface.public_function_count, 1);
}

#[test]
fn api_surface_analyzer_only_counts_explicit_python_all() {
    let implicit = build_indexed_quality_facts(
        "pkg/service.py",
        "python",
        64,
        Some(1),
        "def public_name():\n    pass\n",
    );
    let explicit = build_indexed_quality_facts(
        "pkg/__init__.py",
        "python",
        64,
        Some(1),
        "__all__ = ['first', 'second']\n",
    );

    assert_eq!(implicit.api_surface.public_export_count, 0);
    assert_eq!(explicit.api_surface.public_export_count, 2);
}

#[test]
fn api_surface_rules_emit_metrics_and_violations() {
    let mut facts = build_indexed_quality_facts(
        "src/index.ts",
        "typescript",
        128,
        Some(1),
        "export { A, B, C } from './a';\nexport * from './b';\nexport function run() {}\n",
    );
    facts.structural = StructuralFacts {
        fan_in_count: Some(30),
        ..StructuralFacts::default()
    };
    facts.git_risk = GitRiskFacts {
        recent_churn_lines: 2_000,
        ..GitRiskFacts::default()
    };
    let mut policy = default_quality_policy();
    policy.thresholds.max_public_api_exports_per_file = 2;
    policy.thresholds.max_public_reexports_per_file = 2;
    policy.thresholds.max_public_api_hub_score = 100;
    policy.git_risk.max_recent_churn_lines_per_file = 100;

    let evaluation = evaluate_quality(&facts, &IndexedQualityMetrics::default(), &policy);

    assert!(evaluation.snapshot.metrics.iter().any(|metric| {
        metric.metric_id == "public_api_export_count"
            && metric.metric_value == 5
            && metric.source == Some(QualitySource::ParserLight)
    }));
    assert!(evaluation.snapshot.metrics.iter().any(|metric| {
        metric.metric_id == "public_api_hub_score" && metric.metric_value == 150
    }));
    for rule_id in [
        "wide_public_api_surface",
        "public_reexport_hub",
        "public_api_hub",
        "unstable_public_hub",
    ] {
        assert!(
            evaluation
                .snapshot
                .violations
                .iter()
                .any(|violation| violation.rule_id == rule_id),
            "{rule_id} should be emitted"
        );
    }
}
