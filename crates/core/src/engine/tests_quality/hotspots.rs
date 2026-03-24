use super::{Engine, RuleViolationsOptions, temp_dir, write_project_file};
use crate::model::{
    QualityHotspotAggregation, QualityHotspotsOptions, QualitySource, RuleViolationsSortBy,
};

#[test]
fn rule_violations_expose_hotspot_locations_and_sources() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-hotspots");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":2,"thresholds":{"max_function_lines":3,"max_nesting_depth":2,"max_parameters_per_function":3,"max_export_count_per_file":2,"max_class_member_count":2,"max_todo_count_per_file":1}}"#,
    )?;
    write_project_file(
        &root,
        "src/app.ts",
        "export function noisy(a: number, b: number, c: number, d: number) {\n  if (a) {\n    if (b) {\n      if (c) {\n        return d;\n      }\n    }\n  }\n  return 0;\n}\n\nexport class Widget {\n  first() {}\n  second() {}\n  third() {}\n}\n\nexport const one = 1;\nexport const two = 2;\n// TODO: first\n// FIXME: second\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.rule_violations(&RuleViolationsOptions {
        metric_ids: vec![
            "max_function_lines".to_string(),
            "max_nesting_depth".to_string(),
            "max_parameters_per_function".to_string(),
            "max_class_member_count".to_string(),
            "max_todo_count_per_file".to_string(),
        ],
        sort_metric_id: Some("max_function_lines".to_string()),
        sort_by: RuleViolationsSortBy::MetricValue,
        ..RuleViolationsOptions::default()
    })?;

    let hit = result
        .hits
        .iter()
        .find(|hit| hit.path == "src/app.ts")
        .expect("typescript hotspot file should be present");

    let function_metric = hit
        .metrics
        .iter()
        .find(|metric| metric.metric_id == "max_function_lines")
        .expect("function hotspot metric should be present");
    assert_eq!(function_metric.source, Some(QualitySource::Ast));
    assert_eq!(
        function_metric
            .location
            .as_ref()
            .map(|location| location.start_line),
        Some(1)
    );

    let todo_metric = hit
        .metrics
        .iter()
        .find(|metric| metric.metric_id == "max_todo_count_per_file")
        .expect("todo hotspot metric should be present");
    assert_eq!(todo_metric.source, Some(QualitySource::Heuristic));
    assert!(todo_metric.location.is_none());

    let nesting_violation = hit
        .violations
        .iter()
        .find(|violation| violation.rule_id == "max_nesting_depth")
        .expect("nesting hotspot violation should be present");
    assert_eq!(nesting_violation.source, Some(QualitySource::ParserLight));
    assert!(nesting_violation.location.is_some());

    let export_violation = hit
        .violations
        .iter()
        .find(|violation| violation.rule_id == "max_export_count_per_file")
        .expect("export hotspot violation should be present");
    assert!(export_violation.location.is_none());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn quality_hotspots_file_mode_reuses_file_risk_scores() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-hotspots-file");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "src/lib.rs",
        "pub fn noisy() {\n  let _a = 1;\n  let _b = 2;\n  let _c = 3;\n  let _d = 4;\n  let _e = 5;\n  let _f = 6;\n  let _g = 7;\n  let _h = 8;\n  let _i = 9;\n  let _j = 10;\n}\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let violations = engine.rule_violations(&RuleViolationsOptions::default())?;
    let hotspots = engine.quality_hotspots(&QualityHotspotsOptions::default())?;

    let violation_hit = violations
        .hits
        .iter()
        .find(|hit| hit.path == "src/lib.rs")
        .expect("file hit should exist");
    let hotspot_bucket = hotspots
        .buckets
        .iter()
        .find(|bucket| bucket.bucket_id == "src/lib.rs")
        .expect("file hotspot bucket should exist");

    assert_eq!(hotspots.summary.aggregation, QualityHotspotAggregation::File);
    assert_eq!(
        hotspot_bucket.risk_score.map(|risk| risk.score),
        violation_hit.risk_score.map(|risk| risk.score)
    );
    assert_eq!(
        hotspot_bucket.hotspot_score,
        violation_hit.risk_score.map(|risk| risk.score).unwrap_or_default()
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn quality_hotspots_directory_and_module_modes_group_deterministically() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-hotspots-aggregate");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{
            "version":2,
            "structural":{
                "zones":[
                    {"id":"ui","paths":["src/ui/**"]},
                    {"id":"core","paths":["src/core/**"]}
                ],
                "allowed_directions":[{"from":"ui","to":"core"}],
                "unmatched_behavior":"allow"
            }
        }"#,
    )?;
    write_project_file(&root, "src/ui/page.ts", "export function page() {\n  return 1;\n}\n")?;
    write_project_file(
        &root,
        "src/core/logic.ts",
        "export function noisy() {\n  let value = \"this line is intentionally very very very very very very very very very very very very very very very very very very very long\";\n  return value;\n}\n",
    )?;
    write_project_file(&root, "src/misc/root.ts", "export const misc = 1;\n")?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let directory = engine.quality_hotspots(&QualityHotspotsOptions {
        aggregation: QualityHotspotAggregation::Directory,
        ..QualityHotspotsOptions::default()
    })?;
    let module = engine.quality_hotspots(&QualityHotspotsOptions {
        aggregation: QualityHotspotAggregation::Module,
        ..QualityHotspotsOptions::default()
    })?;

    assert!(directory.buckets.iter().any(|bucket| bucket.bucket_id == "src/core"));
    assert!(directory.buckets.iter().any(|bucket| bucket.bucket_id == "src/ui"));
    assert!(module.buckets.iter().any(|bucket| bucket.bucket_id == "core"));
    assert!(module.buckets.iter().any(|bucket| bucket.bucket_id == "ui"));
    assert!(module
        .buckets
        .iter()
        .any(|bucket| bucket.bucket_id == "unmatched"));

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
