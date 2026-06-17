use super::{Engine, RuleViolationsOptions, temp_dir, write_project_file};
use crate::model::RuleViolationsSortBy;

#[test]
fn rule_violations_expose_api_surface_contract() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-api-surface");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":4,"thresholds":{"max_public_api_exports_per_file":2,"max_public_reexports_per_file":1,"max_public_api_hub_score":20,"max_fan_in_per_file":1}}"#,
    )?;
    write_project_file(
        &root,
        "src/index.ts",
        "export { A, B } from './a';\nexport * from './b';\nexport function run() {}\nimport { user } from './consumer';\n",
    )?;
    write_project_file(
        &root,
        "src/consumer.ts",
        "import { run } from './index';\nexport const user = run;\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.rule_violations(&RuleViolationsOptions {
        sort_metric_id: Some("public_api_export_count".to_string()),
        sort_by: RuleViolationsSortBy::MetricValue,
        ..RuleViolationsOptions::default()
    })?;
    let hit = result
        .hits
        .iter()
        .find(|hit| hit.path == "src/index.ts")
        .expect("api surface file should be present");

    assert!(hit.metrics.iter().any(|metric| {
        metric.metric_id == "public_api_export_count" && metric.metric_value == 4
    }));
    assert!(hit.metrics.iter().any(|metric| {
        metric.metric_id == "public_api_reexport_count" && metric.metric_value == 3
    }));
    assert!(
        hit.violations
            .iter()
            .any(|violation| { violation.rule_id == "wide_public_api_surface" })
    );
    assert!(
        hit.violations
            .iter()
            .any(|violation| { violation.rule_id == "public_reexport_hub" })
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
