use super::{
    Engine, HashMap, OptionalExtension, RuleViolationsOptions, repeated_lines, temp_dir,
    write_project_file,
};

#[test]
fn indexing_persists_quality_snapshot_and_workspace_summary() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-summary");
    std::fs::create_dir_all(&root)?;
    write_project_file(&root, "src/lib.rs", &repeated_lines("fn item() {}", 301))?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    let summary = engine.index_path()?;
    assert_eq!(summary.indexed, 1);

    let conn = engine.open_db()?;
    let stored: Option<(i64, i64, i64)> = conn
        .query_row(
            "SELECT size_bytes, total_lines, non_empty_lines FROM file_quality WHERE path = 'src/lib.rs'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    assert!(stored.is_some());

    let brief = engine.workspace_brief()?;
    assert_eq!(brief.quality_summary.ruleset_id, "quality-core-v13");
    assert_eq!(brief.quality_summary.status.as_str(), "ready");
    assert_eq!(brief.quality_summary.evaluated_files, 1);
    assert_eq!(brief.quality_summary.violating_files, 1);
    assert!(brief.quality_summary.total_violations >= 1);
    assert!(
        brief
            .quality_summary
            .top_rules
            .iter()
            .any(|rule| rule.rule_id == "max_non_empty_lines_default")
    );
    assert!(
        brief
            .quality_summary
            .top_metrics
            .iter()
            .any(|metric| metric.metric_id == "non_empty_lines")
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn quality_policy_overrides_default_thresholds() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-policy");
    std::fs::create_dir_all(&root)?;
    write_project_file(&root, "src/lib.rs", &repeated_lines("line", 301))?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":4,"thresholds":{"max_non_empty_lines_default":400}}"#,
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.rule_violations(&RuleViolationsOptions::default())?;
    assert!(
        result.hits.iter().all(|hit| hit
            .violations
            .iter()
            .all(|violation| violation.rule_id != "max_non_empty_lines_default")),
        "policy override should suppress the default non-empty-line violation"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn rule_violations_expose_metrics_and_locations() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-metrics-locations");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "src/lib.rs",
        "use std::fmt;\nfn short() {}\nfn wide() { let value = \"this line is intentionally very very very very very very very very very very very very very very very very very very very long and should cross the configured threshold\"; }\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.rule_violations(&RuleViolationsOptions {
        metric_ids: vec!["max_line_length".to_string()],
        sort_metric_id: Some("max_line_length".to_string()),
        sort_by: crate::model::RuleViolationsSortBy::MetricValue,
        ..RuleViolationsOptions::default()
    })?;

    let hit = result
        .hits
        .iter()
        .find(|hit| hit.path == "src/lib.rs")
        .expect("src/lib.rs should be present");
    assert!(
        hit.metrics
            .iter()
            .any(|metric| metric.metric_id == "max_line_length")
    );
    assert!(hit.risk_score.is_some(), "risk_score should be present");
    let violation = hit
        .violations
        .iter()
        .find(|violation| violation.rule_id == "max_line_length")
        .expect("max_line_length violation should be present");
    assert_eq!(
        violation
            .location
            .as_ref()
            .map(|location| location.start_line),
        Some(3)
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn quality_rules_cover_default_test_config_and_import_thresholds() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-thresholds");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "src/default.rs",
        &repeated_lines("default_line", 301),
    )?;
    write_project_file(&root, "tests/heavy.rs", &repeated_lines("test_line", 501))?;
    write_project_file(
        &root,
        "config/app.toml",
        &repeated_lines("config = true", 101),
    )?;
    let imports = (0..21)
        .map(|idx| format!("import mod_{idx}\n"))
        .collect::<String>();
    write_project_file(
        &root,
        "scripts/imports.py",
        &format!("{imports}print('ok')\n"),
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.rule_violations(&RuleViolationsOptions::default())?;
    let rules_by_path = result
        .hits
        .into_iter()
        .map(|hit| {
            (
                hit.path,
                hit.violations
                    .into_iter()
                    .map(|violation| violation.rule_id)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<HashMap<_, _>>();

    assert!(rules_by_path["src/default.rs"].contains(&"max_non_empty_lines_default".to_string()));
    assert!(rules_by_path["tests/heavy.rs"].contains(&"max_non_empty_lines_test".to_string()));
    assert!(rules_by_path["config/app.toml"].contains(&"max_non_empty_lines_config".to_string()));
    assert!(rules_by_path["scripts/imports.py"].contains(&"max_import_count".to_string()));

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn rule_violations_keep_suppressed_entries_auditable() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-suppressions");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "src/lib.rs",
        "fn noisy() { let value = \"this line is intentionally very very very very very very very very very very very very very very very very very very very long\"; }\n",
    )?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{
            "version":4,
            "test_risk":{"enabled":false},
            "suppressions":[
                {
                    "id":"legacy-line-length",
                    "rule_ids":["max_line_length"],
                    "paths":["src/**"],
                    "reason":"legacy file kept for compatibility"
                }
            ]
        }"#,
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.rule_violations(&RuleViolationsOptions::default())?;
    let hit = result
        .hits
        .iter()
        .find(|hit| hit.path == "src/lib.rs")
        .expect("src/lib.rs should be present");
    assert!(
        hit.violations
            .iter()
            .all(|violation| violation.rule_id != "max_line_length")
    );
    let suppressed = hit
        .suppressed_violations
        .iter()
        .find(|entry| entry.violation.rule_id == "max_line_length")
        .expect("suppressed max_line_length should be present");
    let risk_score = hit
        .risk_score
        .as_ref()
        .expect("risk_score should be present for suppressed-only hits");
    assert_eq!(suppressed.suppressions.len(), 1);
    assert_eq!(
        suppressed.suppressions[0].suppression_id,
        "legacy-line-length"
    );
    assert_eq!(
        suppressed.suppressions[0].reason,
        "legacy file kept for compatibility"
    );
    assert_eq!(risk_score.components.violation_count, 0.0);
    assert_eq!(risk_score.components.severity, 0.0);
    assert_eq!(result.summary.suppressed_violations, 1);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn rule_violations_summary_tracks_the_returned_slice() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-summary-returned-slice");
    std::fs::create_dir_all(&root)?;
    write_project_file(&root, "src/a.rs", &repeated_lines("line_a", 301))?;
    write_project_file(&root, "src/b.rs", &repeated_lines("line_b", 302))?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.rule_violations(&RuleViolationsOptions {
        limit: 1,
        sort_by: crate::model::RuleViolationsSortBy::NonEmptyLines,
        ..RuleViolationsOptions::default()
    })?;

    let severity_total = result
        .summary
        .severity_breakdown
        .iter()
        .map(|entry| entry.violations)
        .sum::<usize>();
    let category_total = result
        .summary
        .category_breakdown
        .iter()
        .map(|entry| entry.violations)
        .sum::<usize>();

    assert_eq!(result.summary.evaluated_files, 2);
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.summary.violating_files, 1);
    assert_eq!(severity_total, result.summary.total_violations);
    assert_eq!(category_total, result.summary.total_violations);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
