use super::{Engine, RuleViolationsOptions, repeated_lines, temp_dir, write_project_file};
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
        r#"{"version":3,"thresholds":{"max_function_lines":3,"max_nesting_depth":2,"max_parameters_per_function":3,"max_export_count_per_file":2,"max_class_member_count":2,"max_todo_count_per_file":1}}"#,
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

    assert_eq!(
        hotspots.summary.aggregation,
        QualityHotspotAggregation::File
    );
    assert_eq!(
        hotspot_bucket.risk_score.map(|risk| risk.score),
        violation_hit.risk_score.map(|risk| risk.score)
    );
    assert_eq!(
        hotspot_bucket.hotspot_score,
        violation_hit
            .risk_score
            .map(|risk| risk.score)
            .unwrap_or_default()
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn rule_violations_and_hotspots_expose_complexity_contract() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-complexity-contract");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_cyclomatic_complexity":2,"max_cognitive_complexity":2}}"#,
    )?;
    write_project_file(
        &root,
        "src/lib.rs",
        "pub fn noisy(flag: bool, level: i32) -> i32 {\n    if flag {\n        if level > 0 {\n            return 1;\n        }\n    }\n    if level < 0 {\n        return -1;\n    }\n    return 0;\n}\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let violations = engine.rule_violations(&RuleViolationsOptions {
        metric_ids: vec![
            "max_cyclomatic_complexity".to_string(),
            "max_cognitive_complexity".to_string(),
            "max_branch_count".to_string(),
            "max_early_return_count".to_string(),
        ],
        sort_metric_id: Some("max_cognitive_complexity".to_string()),
        sort_by: RuleViolationsSortBy::MetricValue,
        ..RuleViolationsOptions::default()
    })?;
    let hotspots = engine.quality_hotspots(&QualityHotspotsOptions::default())?;

    let hit = violations
        .hits
        .iter()
        .find(|hit| hit.path == "src/lib.rs")
        .expect("complexity file hit should exist");
    let bucket = hotspots
        .buckets
        .iter()
        .find(|bucket| bucket.bucket_id == "src/lib.rs")
        .expect("complexity hotspot bucket should exist");
    let metric_ids = hit
        .metrics
        .iter()
        .map(|metric| metric.metric_id.as_str())
        .collect::<Vec<_>>();
    let violation_ids = hit
        .violations
        .iter()
        .map(|violation| violation.rule_id.as_str())
        .collect::<Vec<_>>();
    let risk_score = hit.risk_score.expect("risk_score should exist");
    let bucket_risk = bucket.risk_score.expect("bucket risk score should exist");

    assert!(metric_ids.contains(&"max_cyclomatic_complexity"));
    assert!(metric_ids.contains(&"max_cognitive_complexity"));
    assert!(metric_ids.contains(&"max_branch_count"));
    assert!(metric_ids.contains(&"max_early_return_count"));
    assert!(violation_ids.contains(&"max_cyclomatic_complexity"));
    assert!(violation_ids.contains(&"max_cognitive_complexity"));
    assert!(!violation_ids.contains(&"max_branch_count"));
    assert!(!violation_ids.contains(&"max_early_return_count"));
    assert_eq!(
        hit.metrics
            .iter()
            .find(|metric| metric.metric_id == "max_cyclomatic_complexity")
            .and_then(|metric| metric.source),
        Some(QualitySource::ParserLight)
    );
    assert!(risk_score.components.complexity > 0.0);
    assert_eq!(
        bucket_risk.components.complexity,
        risk_score.components.complexity
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
            "version":3,
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
    write_project_file(
        &root,
        "src/ui/page.ts",
        "export function page() {\n  return 1;\n}\n",
    )?;
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

    assert!(
        directory
            .buckets
            .iter()
            .any(|bucket| bucket.bucket_id == "src/core")
    );
    assert!(
        directory
            .buckets
            .iter()
            .any(|bucket| bucket.bucket_id == "src/ui")
    );
    assert!(
        module
            .buckets
            .iter()
            .any(|bucket| bucket.bucket_id == "core")
    );
    assert!(module.buckets.iter().any(|bucket| bucket.bucket_id == "ui"));
    assert!(
        module
            .buckets
            .iter()
            .any(|bucket| bucket.bucket_id == "unmatched")
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn complexity_can_outrank_long_linear_files_in_hotspots() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-hotspots-complexity-ranking");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{
            "version":3,
            "thresholds":{
                "max_non_empty_lines_default":50,
                "max_cyclomatic_complexity":2,
                "max_cognitive_complexity":2
            }
        }"#,
    )?;
    write_project_file(
        &root,
        "src/branchy.rs",
        "pub fn branchy(flag: bool, level: i32) -> i32 {\n    if flag {\n        if level > 10 {\n            return 1;\n        }\n    }\n    if level < 0 {\n        return -1;\n    }\n    return 0;\n}\n",
    )?;
    write_project_file(
        &root,
        "src/linear.rs",
        &repeated_lines("let value = 1;", 80),
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let hotspots = engine.quality_hotspots(&QualityHotspotsOptions::default())?;
    let top_bucket = hotspots
        .buckets
        .first()
        .expect("at least one hotspot bucket should exist");

    assert_eq!(top_bucket.bucket_id, "src/branchy.rs");

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn rule_violations_and_hotspots_expose_duplication_contract() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-contract");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":500}}"#,
    )?;
    write_project_file(
        &root,
        "src/alpha.rs",
        "pub fn repeated(input: i32) -> i32 {\n    let mut total = input;\n    total += 1;\n    total += 2;\n    total += 3;\n    total += 4;\n    total += 5;\n    total += 6;\n    total += 7;\n    total += 8;\n    total += 9;\n    total += 10;\n    total += 11;\n    total += 12;\n    total += 13;\n    total += 14;\n    total += 15;\n    total += 16;\n    if total > 10 {\n        total -= 2;\n    }\n    if total % 2 == 0 {\n        total += 3;\n    }\n    if total > 40 {\n        total -= 5;\n    }\n    if total > 80 {\n        total -= 7;\n    }\n    if total % 3 == 0 {\n        total += 9;\n    }\n    if total > 120 {\n        total -= 11;\n    }\n    total\n}\n",
    )?;
    write_project_file(
        &root,
        "src/beta.rs",
        "pub fn repeated(input: i32) -> i32 {\n    let mut total = input;\n    total += 1;\n    total += 2;\n    total += 3;\n    total += 4;\n    total += 5;\n    total += 6;\n    total += 7;\n    total += 8;\n    total += 9;\n    total += 10;\n    total += 11;\n    total += 12;\n    total += 13;\n    total += 14;\n    total += 15;\n    total += 16;\n    if total > 10 {\n        total -= 2;\n    }\n    if total % 2 == 0 {\n        total += 3;\n    }\n    if total > 40 {\n        total -= 5;\n    }\n    if total > 80 {\n        total -= 7;\n    }\n    if total % 3 == 0 {\n        total += 9;\n    }\n    if total > 120 {\n        total -= 11;\n    }\n    total\n}\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let violations = engine.rule_violations(&RuleViolationsOptions {
        metric_ids: vec![
            "duplicate_block_count".to_string(),
            "duplicate_density_bps".to_string(),
            "duplicate_peer_count".to_string(),
            "max_duplicate_block_tokens".to_string(),
        ],
        sort_metric_id: Some("duplicate_density_bps".to_string()),
        sort_by: RuleViolationsSortBy::MetricValue,
        ..RuleViolationsOptions::default()
    })?;
    let hotspots = engine.quality_hotspots(&QualityHotspotsOptions::default())?;

    let hit = violations
        .hits
        .iter()
        .find(|hit| hit.path == "src/alpha.rs")
        .expect("duplication file hit should exist");
    let bucket = hotspots
        .buckets
        .iter()
        .find(|bucket| bucket.bucket_id == "src/alpha.rs")
        .expect("duplication hotspot bucket should exist");

    assert!(
        hit.metrics
            .iter()
            .any(|metric| metric.metric_id == "duplicate_block_count" && metric.metric_value > 0)
    );
    assert!(
        hit.violations
            .iter()
            .any(|violation| violation.rule_id == "max_duplicate_block_count")
    );
    assert_eq!(
        hit.metrics
            .iter()
            .find(|metric| metric.metric_id == "duplicate_density_bps")
            .and_then(|metric| metric.source),
        Some(QualitySource::Duplication)
    );
    assert!(
        hit.risk_score
            .expect("duplication risk score should exist")
            .components
            .duplication
            > 0.0
    );
    assert!(
        bucket
            .risk_score
            .expect("duplication bucket risk should exist")
            .components
            .duplication
            > 0.0
    );
    assert!(
        root.join(".rmu/quality/duplication.clone_classes.json")
            .exists()
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn same_file_duplication_stays_out_of_file_level_quality_signal() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-same-file");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":500}}"#,
    )?;
    write_project_file(
        &root,
        "src/lib.rs",
        "pub fn alpha(input: i32) -> i32 {\n    let mut total = input;\n    total += 1;\n    total += 2;\n    total += 3;\n    total += 4;\n    total += 5;\n    total += 6;\n    total += 7;\n    total += 8;\n    if total > 10 {\n        total -= 2;\n    }\n    if total % 2 == 0 {\n        total += 3;\n    }\n    if total > 40 {\n        total -= 5;\n    }\n    total\n}\n\npub fn beta(value: i32) -> i32 {\n    let mut amount = value;\n    amount += 1;\n    amount += 2;\n    amount += 3;\n    amount += 4;\n    amount += 5;\n    amount += 6;\n    amount += 7;\n    amount += 8;\n    if amount > 10 {\n        amount -= 2;\n    }\n    if amount % 2 == 0 {\n        amount += 3;\n    }\n    if amount > 40 {\n        amount -= 5;\n    }\n    amount\n}\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let violations = engine.rule_violations(&RuleViolationsOptions {
        metric_ids: vec![
            "duplicate_block_count".to_string(),
            "duplicate_density_bps".to_string(),
        ],
        sort_metric_id: Some("duplicate_density_bps".to_string()),
        sort_by: RuleViolationsSortBy::MetricValue,
        ..RuleViolationsOptions::default()
    })?;

    let hit = violations
        .hits
        .iter()
        .find(|hit| hit.path == "src/lib.rs")
        .expect("same-file duplication test hit should exist");
    assert_eq!(
        hit.metrics
            .iter()
            .find(|metric| metric.metric_id == "duplicate_density_bps")
            .map(|metric| metric.metric_value),
        Some(0)
    );
    assert!(
        hit.violations
            .iter()
            .all(|violation| violation.rule_id != "max_duplicate_density_bps")
    );
    assert_eq!(
        hit.risk_score
            .expect("same-file duplication risk score should exist")
            .components
            .duplication,
        0.0
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
