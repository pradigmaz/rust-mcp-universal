use std::error::Error;
use std::fs;

use rmu_core::{Engine, IndexingOptions, QualityStatus, RuleViolationsOptions};
use rusqlite::{Connection, OptionalExtension};

use crate::common::{cleanup_project, temp_project_dir};

#[test]
fn scoped_encoded_paths_can_be_repaired_by_quality_refresh() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-quality-encoded-path");
    fs::create_dir_all(project_dir.join("src/[id]"))?;
    fs::write(
        project_dir.join("src/[id]/page.tsx"),
        "export function encodedPathQuality() { return 1; }\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path_with_options(&IndexingOptions {
        include_paths: vec!["src".to_string()],
        ..IndexingOptions::default()
    })?;

    let conn = Connection::open(project_dir.join(".rmu/index.db"))?;
    let stored_path: String = conn.query_row(
        "SELECT path FROM files WHERE path LIKE 'src/%/page.tsx'",
        [],
        |row| row.get(0),
    )?;
    assert_ne!(stored_path, "src/[id]/page.tsx");
    delete_quality_for_path(&conn, &stored_path)?;

    assert_eq!(
        engine
            .workspace_brief_with_policy(false)?
            .quality_summary
            .status,
        QualityStatus::Stale
    );
    engine.refresh_quality_if_needed()?;

    assert_eq!(
        engine
            .workspace_brief_with_policy(false)?
            .quality_summary
            .status,
        QualityStatus::Ready
    );
    assert!(quality_path_exists(&conn, &stored_path)?);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn missing_quality_tables_return_unavailable_instead_of_failing() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-quality-missing-table");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.ts"),
        "export function missingQualityTable() { return true; }\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let conn = Connection::open(project_dir.join(".rmu/index.db"))?;
    conn.execute("DROP TABLE file_quality_metrics", [])?;

    let brief = engine.workspace_brief_with_policy(false)?;
    assert_eq!(brief.quality_summary.status, QualityStatus::Unavailable);

    let violations = engine.rule_violations(&RuleViolationsOptions::default())?;
    assert_eq!(violations.summary.status, QualityStatus::Unavailable);
    assert!(violations.hits.is_empty());

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn malformed_quality_tables_degrade_instead_of_crashing() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-quality-malformed-table");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.ts"),
        "export function malformedQualityTable() { return true; }\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let conn = Connection::open(project_dir.join(".rmu/index.db"))?;
    conn.execute(
        "ALTER TABLE file_quality_metrics RENAME TO file_quality_metrics_backup",
        [],
    )?;
    conn.execute(
        "CREATE TABLE file_quality_metrics(path TEXT PRIMARY KEY)",
        [],
    )?;

    assert_eq!(
        engine
            .workspace_brief_with_policy(false)?
            .quality_summary
            .status,
        QualityStatus::Degraded
    );
    assert_eq!(
        engine
            .rule_violations(&RuleViolationsOptions::default())?
            .summary
            .status,
        QualityStatus::Degraded
    );
    assert!(engine.refresh_quality_if_needed().is_ok());
    assert_eq!(
        meta_value(&conn, "quality.status")?.as_deref(),
        Some(QualityStatus::Unavailable.as_str())
    );

    cleanup_project(&project_dir);
    Ok(())
}

fn delete_quality_for_path(conn: &Connection, path: &str) -> Result<(), Box<dyn Error>> {
    conn.execute("DELETE FROM file_rule_violations WHERE path = ?1", [path])?;
    conn.execute("DELETE FROM file_quality_metrics WHERE path = ?1", [path])?;
    conn.execute("DELETE FROM file_quality WHERE path = ?1", [path])?;
    Ok(())
}

fn quality_path_exists(conn: &Connection, path: &str) -> Result<bool, Box<dyn Error>> {
    Ok(conn.query_row(
        "SELECT COUNT(1) FROM file_quality WHERE path = ?1",
        [path],
        |row| row.get::<_, i64>(0),
    )? > 0)
}

fn meta_value(conn: &Connection, key: &str) -> Result<Option<String>, Box<dyn Error>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?)
}
