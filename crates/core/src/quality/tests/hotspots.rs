use crate::model::QualitySource;
use crate::quality::{
    IndexedQualityMetrics, build_indexed_quality_facts, default_quality_policy, evaluate_quality,
};

#[test]
fn javascript_hotspots_emit_locations_and_sources() {
    let facts = build_indexed_quality_facts(
        "src/app.ts",
        "typescript",
        128,
        Some(1),
        "export function alpha(a: number, b: number, c: number) {\n  if (a) {\n    if (b) {\n      return c;\n    }\n  }\n  return 0;\n}\nexport class Widget {\n  first() {}\n  second() {}\n}\n",
    );
    let evaluation = evaluate_quality(
        &facts,
        &IndexedQualityMetrics::default(),
        &default_quality_policy(),
    );

    let function_metric = evaluation
        .snapshot
        .metrics
        .iter()
        .find(|metric| metric.metric_id == "max_function_lines")
        .expect("typescript function hotspot metric should exist");
    assert_eq!(function_metric.source, Some(QualitySource::Ast));
    assert_eq!(
        function_metric
            .location
            .as_ref()
            .map(|location| location.start_line),
        Some(1)
    );

    let nesting_metric = evaluation
        .snapshot
        .metrics
        .iter()
        .find(|metric| metric.metric_id == "max_nesting_depth")
        .expect("typescript nesting hotspot metric should exist");
    assert_eq!(nesting_metric.source, Some(QualitySource::ParserLight));
}

#[test]
fn python_and_rust_hotspots_use_parser_light() {
    let python = build_indexed_quality_facts(
        "src/tasks.py",
        "python",
        64,
        Some(1),
        "class Worker:\n    def alpha(self, a, b, c):\n        if a:\n            if b:\n                return c\n        return 0\n",
    );
    let rust = build_indexed_quality_facts(
        "src/lib.rs",
        "rust",
        64,
        Some(1),
        "pub fn alpha(a: i32, b: i32, c: i32, d: i32) {\n    if a > 0 {\n        if b > 0 {\n            if c > 0 {\n                println!(\"{}\", d);\n            }\n        }\n    }\n}\n",
    );
    let policy = default_quality_policy();

    let python_eval = evaluate_quality(&python, &IndexedQualityMetrics::default(), &policy);
    let rust_eval = evaluate_quality(&rust, &IndexedQualityMetrics::default(), &policy);

    let python_metric = python_eval
        .snapshot
        .metrics
        .iter()
        .find(|metric| metric.metric_id == "max_parameters_per_function")
        .expect("python params hotspot metric should exist");
    assert_eq!(python_metric.source, Some(QualitySource::ParserLight));
    assert!(python_metric.location.is_some());

    let rust_metric = rust_eval
        .snapshot
        .metrics
        .iter()
        .find(|metric| metric.metric_id == "max_export_count_per_file")
        .expect("rust export hotspot metric should exist");
    assert_eq!(rust_metric.source, Some(QualitySource::ParserLight));
    assert!(
        rust_eval
            .snapshot
            .metrics
            .iter()
            .all(|metric| metric.metric_id != "max_class_member_count")
    );
}
