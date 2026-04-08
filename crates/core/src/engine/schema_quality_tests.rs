use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use super::{OPEN_DB_PRAGMAS_SQL, apply_schema_migrations};

fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

fn open_db(db_path: &PathBuf) -> anyhow::Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(OPEN_DB_PRAGMAS_SQL)?;
    Ok(conn)
}

#[test]
fn init_schema_contains_quality_tables() -> anyhow::Result<()> {
    let root = temp_dir("rmu-schema-quality-init");
    fs::create_dir_all(&root)?;
    let db_path = root.join("index.db");
    let conn = open_db(&db_path)?;
    conn.execute_batch(super::INIT_DB_SCHEMA_SQL)?;

    let tables = table_names(&conn)?;
    assert!(tables.iter().any(|name| name == "file_quality"));
    assert!(tables.iter().any(|name| name == "file_quality_metrics"));
    assert!(tables.iter().any(|name| name == "file_rule_violations"));

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn migration_runner_adds_quality_tables() -> anyhow::Result<()> {
    let root = temp_dir("rmu-schema-quality-migration");
    fs::create_dir_all(&root)?;
    let db_path = root.join("index.db");
    let mut conn = open_db(&db_path)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS files (
            path TEXT PRIMARY KEY,
            sha256 TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            language TEXT NOT NULL,
            sample TEXT NOT NULL,
            indexed_at_utc TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS schema_migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at_utc TEXT NOT NULL
        );
        "#,
    )?;

    apply_schema_migrations(&mut conn, &db_path, true)?;

    let tables = table_names(&conn)?;
    assert!(tables.iter().any(|name| name == "file_quality"));
    assert!(tables.iter().any(|name| name == "file_rule_violations"));

    let quality_columns = table_columns(&conn, "file_quality")?;
    assert!(
        quality_columns
            .iter()
            .any(|name| name == "quality_ruleset_version")
    );
    assert!(
        quality_columns
            .iter()
            .any(|name| name == "quality_violation_hash")
    );
    assert!(
        quality_columns
            .iter()
            .any(|name| name == "quality_suppressed_violation_hash")
    );
    let violation_columns = table_columns(&conn, "file_rule_violations")?;
    let metric_columns = table_columns(&conn, "file_quality_metrics")?;
    assert!(metric_columns.iter().any(|name| name == "source"));
    assert!(metric_columns.iter().any(|name| name == "start_line"));
    assert!(violation_columns.iter().any(|name| name == "start_line"));
    assert!(violation_columns.iter().any(|name| name == "end_column"));
    assert!(violation_columns.iter().any(|name| name == "source"));
    assert!(violation_columns.iter().any(|name| name == "severity"));
    assert!(violation_columns.iter().any(|name| name == "category"));
    assert!(
        violation_columns
            .iter()
            .any(|name| name == "finding_family")
    );
    assert!(violation_columns.iter().any(|name| name == "confidence"));
    assert!(
        violation_columns
            .iter()
            .any(|name| name == "manual_review_required")
    );
    assert!(violation_columns.iter().any(|name| name == "noise_reason"));
    assert!(
        violation_columns
            .iter()
            .any(|name| name == "recommended_followups_json")
    );

    let _ = fs::remove_dir_all(root);
    Ok(())
}

fn table_names(conn: &Connection) -> anyhow::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type='table'")?;
    Ok(stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

fn table_columns(conn: &Connection, table: &str) -> anyhow::Result<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    Ok(stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}
