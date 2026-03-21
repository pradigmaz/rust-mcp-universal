use std::error::Error;
use std::fs;

use rmu_core::{Engine, IndexProfile, IndexingOptions, QualityStatus};
use rusqlite::{Connection, OptionalExtension};

use crate::common::{cleanup_project, temp_project_dir};

#[test]
fn mixed_scope_meta_backfills_only_in_scope_quality_rows() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-quality-scope-meta");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("docs"))?;
    fs::create_dir_all(project_dir.join(".codex-planning"))?;
    fs::write(
        project_dir.join("src/main.ts"),
        "export function mainScopeQuality() { return 'ok'; }\n",
    )?;
    fs::write(project_dir.join("docs/design.md"), "docs only\n")?;
    fs::write(
        project_dir.join(".codex-planning/task_plan.md"),
        "# planning only\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path_with_options(&IndexingOptions {
        profile: Some(IndexProfile::Mixed),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let conn = Connection::open(project_dir.join(".rmu/index.db"))?;
    assert_eq!(meta_value(&conn, "index.scope.version")?.as_deref(), Some("1"));
    assert_eq!(meta_value(&conn, "index.scope.profile")?.as_deref(), Some("mixed"));
    assert_eq!(
        meta_value(&conn, "index.scope.include_paths_json")?.as_deref(),
        Some("[]")
    );
    assert_eq!(
        meta_value(&conn, "index.scope.exclude_paths_json")?.as_deref(),
        Some("[]")
    );
    delete_quality_for_path(&conn, "src/main.ts")?;

    assert_eq!(
        engine.workspace_brief_with_policy(false)?.quality_summary.status,
        QualityStatus::Stale
    );
    engine.refresh_quality_if_needed()?;

    let brief = engine.workspace_brief_with_policy(false)?;
    assert_eq!(brief.quality_summary.status, QualityStatus::Ready);
    assert!(quality_path_exists(&conn, "src/main.ts")?);
    assert!(!quality_path_exists(&conn, "docs/design.md")?);
    assert!(!quality_path_exists(&conn, ".codex-planning/task_plan.md")?);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn legacy_index_without_scope_meta_backfills_missing_quality_rows() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-quality-legacy-backfill");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.ts"),
        "export function legacyQualityBackfill() { return 1; }\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path_with_options(&IndexingOptions {
        profile: Some(IndexProfile::Mixed),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let conn = Connection::open(project_dir.join(".rmu/index.db"))?;
    delete_scope_meta(&conn)?;
    delete_quality_for_path(&conn, "src/main.ts")?;

    assert_eq!(
        engine.workspace_brief_with_policy(false)?.quality_summary.status,
        QualityStatus::Stale
    );
    engine.refresh_quality_if_needed()?;

    let brief = engine.workspace_brief_with_policy(false)?;
    assert_eq!(brief.quality_summary.status, QualityStatus::Ready);
    assert!(quality_path_exists(&conn, "src/main.ts")?);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn legacy_missing_files_rows_do_not_hold_quality_status_stale() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-quality-legacy-missing-file");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("docs"))?;
    fs::write(
        project_dir.join("src/main.ts"),
        "export function stillIndexed() { return true; }\n",
    )?;
    fs::write(project_dir.join("docs/stale.md"), "stale docs row\n")?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let conn = Connection::open(project_dir.join(".rmu/index.db"))?;
    delete_scope_meta(&conn)?;
    fs::remove_file(project_dir.join("docs/stale.md"))?;

    assert_eq!(
        engine.workspace_brief_with_policy(false)?.quality_summary.status,
        QualityStatus::Stale
    );
    engine.refresh_quality_if_needed()?;

    let brief = engine.workspace_brief_with_policy(false)?;
    assert_eq!(brief.quality_summary.status, QualityStatus::Ready);
    assert_eq!(file_row_exists(&conn, "docs/stale.md")?, true);
    assert!(!quality_path_exists(&conn, "docs/stale.md")?);

    cleanup_project(&project_dir);
    Ok(())
}

fn meta_value(conn: &Connection, key: &str) -> Result<Option<String>, Box<dyn Error>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| row.get(0))
        .optional()?)
}

fn delete_scope_meta(conn: &Connection) -> Result<(), Box<dyn Error>> {
    conn.execute("DELETE FROM meta WHERE key LIKE 'index.scope.%'", [])?;
    Ok(())
}

fn delete_quality_for_path(conn: &Connection, path: &str) -> Result<(), Box<dyn Error>> {
    conn.execute("DELETE FROM file_rule_violations WHERE path = ?1", [path])?;
    conn.execute("DELETE FROM file_quality_metrics WHERE path = ?1", [path])?;
    conn.execute("DELETE FROM file_quality WHERE path = ?1", [path])?;
    Ok(())
}

fn quality_path_exists(conn: &Connection, path: &str) -> Result<bool, Box<dyn Error>> {
    Ok(conn.query_row("SELECT COUNT(1) FROM file_quality WHERE path = ?1", [path], |row| {
        row.get::<_, i64>(0)
    })? > 0)
}

fn file_row_exists(conn: &Connection, path: &str) -> Result<bool, Box<dyn Error>> {
    Ok(conn.query_row("SELECT COUNT(1) FROM files WHERE path = ?1", [path], |row| {
        row.get::<_, i64>(0)
    })? > 0)
}
