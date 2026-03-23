use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::OptionalExtension;
use time::{Duration, OffsetDateTime};

use super::Engine;
use crate::model::{
    PrivacyMode, QualityMode, QueryOptions, RuleViolationsOptions, SemanticFailMode,
};

fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

fn write_project_file(root: &Path, relative: &str, contents: &str) -> anyhow::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn repeated_lines(prefix: &str, count: usize) -> String {
    (0..count)
        .map(|idx| format!("{prefix}_{idx}\n"))
        .collect::<String>()
}

#[test]
fn indexing_persists_quality_snapshot_and_workspace_summary() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-summary");
    fs::create_dir_all(&root)?;
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
    assert_eq!(brief.quality_summary.ruleset_id, "quality-core-v2");
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

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn quality_policy_overrides_default_thresholds() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-policy");
    fs::create_dir_all(&root)?;
    write_project_file(&root, "src/lib.rs", &repeated_lines("line", 301))?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"thresholds":{"max_non_empty_lines_default":400}}"#,
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.rule_violations(&RuleViolationsOptions::default())?;
    assert!(
        result.hits.iter().all(|hit| hit
            .violations
            .iter()
            .all(|violation| { violation.rule_id != "max_non_empty_lines_default" })),
        "policy override should suppress the default non-empty-line violation"
    );

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn rule_violations_expose_metrics_and_locations() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-metrics-locations");
    fs::create_dir_all(&root)?;
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

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn quality_rules_cover_default_test_config_and_import_thresholds() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-thresholds");
    fs::create_dir_all(&root)?;
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

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn oversize_files_are_quality_only_and_not_searchable() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-oversize");
    fs::create_dir_all(&root)?;
    write_project_file(&root, "src/lib.rs", "pub fn searchable_probe() {}\n")?;
    let oversize = format!(
        "oversize_unique_marker\n{}",
        "X".repeat((crate::utils::INDEX_FILE_LIMIT as usize) + 2048)
    );
    write_project_file(&root, "src/big.rs", &oversize)?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let conn = engine.open_db()?;
    let indexed_big: Option<String> = conn
        .query_row(
            "SELECT path FROM files WHERE path = 'src/big.rs'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    assert!(indexed_big.is_none());

    let quality_big: Option<(String, Option<i64>, String)> = conn
        .query_row(
            "SELECT path, total_lines, quality_mode FROM file_quality WHERE path = 'src/big.rs'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let quality_big = quality_big.expect("oversize quality snapshot should exist");
    assert!(quality_big.1.is_none());
    assert_eq!(quality_big.2, "quality-only-oversize");

    let violations = engine.rule_violations(&RuleViolationsOptions::default())?;
    let big_hit = violations
        .hits
        .iter()
        .find(|hit| hit.path == "src/big.rs")
        .expect("oversize path should be reported");
    assert_eq!(big_hit.quality_mode, QualityMode::QualityOnlyOversize);

    let search_hits = engine.search(&QueryOptions {
        query: "oversize_unique_marker".to_string(),
        limit: 5,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?;
    assert!(
        search_hits.iter().all(|hit| hit.path != "src/big.rs"),
        "oversize files must not leak into retrieval surfaces"
    );

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn changed_since_backfills_missing_quality_rows_even_when_cutoff_would_skip() -> anyhow::Result<()>
{
    let root = temp_dir("rmu-quality-backfill");
    fs::create_dir_all(&root)?;
    write_project_file(&root, "src/lib.rs", "pub fn quality_backfill() {}\n")?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let conn = engine.open_db()?;
    conn.execute(
        "DELETE FROM file_rule_violations WHERE path = 'src/lib.rs'",
        [],
    )?;
    conn.execute("DELETE FROM file_quality WHERE path = 'src/lib.rs'", [])?;

    let repaired = engine.index_path_with_options(&crate::model::IndexingOptions {
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..crate::model::IndexingOptions::default()
    })?;
    assert_eq!(repaired.indexed, 0);
    assert_eq!(repaired.changed, 0);
    assert_eq!(repaired.unchanged, 1);
    assert_eq!(repaired.skipped_before_changed_since, 0);

    let restored: Option<String> = engine
        .open_db()?
        .query_row(
            "SELECT path FROM file_quality WHERE path = 'src/lib.rs'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    assert_eq!(restored.as_deref(), Some("src/lib.rs"));

    let _ = fs::remove_dir_all(root);
    Ok(())
}
