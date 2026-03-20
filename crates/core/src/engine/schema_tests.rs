use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    MIGRATIONS, OPEN_DB_PRAGMAS_SQL, SchemaMigration, apply_schema_migrations,
    apply_schema_migrations_plan,
};
use rusqlite::{Connection, OptionalExtension};

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
fn migration_runner_is_idempotent_on_repeat_run() -> anyhow::Result<()> {
    let root = temp_dir("rmu-migrations-idempotent");
    fs::create_dir_all(&root)?;
    let db_path = root.join("index.db");
    let mut conn = open_db(&db_path)?;
    conn.execute_batch(super::INIT_DB_SCHEMA_SQL)?;

    apply_schema_migrations(&mut conn, &db_path, true)?;
    let first_count: i64 = conn.query_row("SELECT COUNT(1) FROM schema_migrations", [], |row| {
        row.get(0)
    })?;

    apply_schema_migrations(&mut conn, &db_path, true)?;
    let second_count: i64 =
        conn.query_row("SELECT COUNT(1) FROM schema_migrations", [], |row| {
            row.get(0)
        })?;
    assert_eq!(first_count, MIGRATIONS.len() as i64);
    assert_eq!(second_count, first_count);

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn migration_runner_applies_n_to_n_plus_one() -> anyhow::Result<()> {
    let root = temp_dir("rmu-migrations-n-to-n1");
    fs::create_dir_all(&root)?;
    let db_path = root.join("index.db");
    let mut conn = open_db(&db_path)?;
    conn.execute_batch(super::INIT_DB_SCHEMA_SQL)?;
    conn.execute_batch(
        r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at_utc TEXT NOT NULL
            );
            "#,
    )?;
    conn.execute(
            "INSERT INTO schema_migrations(id, name, applied_at_utc) VALUES (1, 'old', '2026-03-03T00:00:00Z')",
            [],
        )?;

    apply_schema_migrations(&mut conn, &db_path, true)?;
    let max_id: Option<i64> =
        conn.query_row("SELECT MAX(id) FROM schema_migrations", [], |row| {
            row.get(0)
        })?;
    assert_eq!(max_id, Some(9));

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn migration_runner_adds_position_columns_to_symbols_and_refs() -> anyhow::Result<()> {
    let root = temp_dir("rmu-migrations-positions");
    fs::create_dir_all(&root)?;
    let db_path = root.join("index.db");
    let mut conn = open_db(&db_path)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS symbols (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            language TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS refs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            path TEXT NOT NULL,
            symbol TEXT NOT NULL,
            language TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS schema_migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at_utc TEXT NOT NULL
        );
        "#,
    )?;

    apply_schema_migrations(&mut conn, &db_path, true)?;

    let symbol_columns = table_columns(&conn, "symbols")?;
    assert!(symbol_columns.iter().any(|name| name == "line"));
    assert!(symbol_columns.iter().any(|name| name == "column"));

    let ref_columns = table_columns(&conn, "refs")?;
    assert!(ref_columns.iter().any(|name| name == "line"));
    assert!(ref_columns.iter().any(|name| name == "column"));

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn migration_runner_adds_source_mtime_column_to_files() -> anyhow::Result<()> {
    let root = temp_dir("rmu-migrations-source-mtime");
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

    let file_columns = table_columns(&conn, "files")?;
    assert!(
        file_columns
            .iter()
            .any(|name| name == "source_mtime_unix_ms")
    );
    assert!(file_columns.iter().any(|name| name == "graph_symbol_count"));
    assert!(file_columns.iter().any(|name| name == "graph_ref_count"));
    assert!(
        file_columns
            .iter()
            .any(|name| name == "graph_module_dep_count")
    );
    assert!(file_columns.iter().any(|name| name == "graph_content_hash"));
    assert!(
        file_columns
            .iter()
            .any(|name| name == "graph_fingerprint_version")
    );
    assert!(
        file_columns
            .iter()
            .any(|name| name == "graph_edge_out_count")
    );
    assert!(
        file_columns
            .iter()
            .any(|name| name == "graph_edge_in_count")
    );
    assert!(file_columns.iter().any(|name| name == "graph_edge_hash"));
    assert!(
        file_columns
            .iter()
            .any(|name| name == "graph_edge_fingerprint_version")
    );

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn migration_runner_creates_premigration_backup_for_existing_db() -> anyhow::Result<()> {
    let root = temp_dir("rmu-migrations-backup");
    fs::create_dir_all(&root)?;
    let db_path = root.join("index.db");
    let mut conn = open_db(&db_path)?;
    conn.execute_batch(super::INIT_DB_SCHEMA_SQL)?;
    conn.execute("INSERT INTO files(path, sha256, size_bytes, language, sample, indexed_at_utc) VALUES ('a.rs', 'x', 1, 'rust', 'fn a(){}', '2026-03-03T00:00:00Z')", [])?;

    apply_schema_migrations(&mut conn, &db_path, true)?;
    let backup_root = db_path
        .parent()
        .expect("db parent")
        .join("migration_backups");
    let entries = fs::read_dir(&backup_root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    assert!(entries.iter().any(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".db"))
    }));

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn migration_runner_forbids_silent_downgrade() -> anyhow::Result<()> {
    let root = temp_dir("rmu-migrations-downgrade");
    fs::create_dir_all(&root)?;
    let db_path = root.join("index.db");
    let mut conn = open_db(&db_path)?;
    conn.execute_batch(super::INIT_DB_SCHEMA_SQL)?;
    conn.execute_batch(
        r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at_utc TEXT NOT NULL
            );
            "#,
    )?;
    conn.execute(
            "INSERT INTO schema_migrations(id, name, applied_at_utc) VALUES (999, 'future', '2026-03-03T00:00:00Z')",
            [],
        )?;

    let err = apply_schema_migrations(&mut conn, &db_path, true)
        .expect_err("future migration id must hard-fail");
    assert!(err.to_string().contains("silent downgrade is forbidden"));

    let _ = fs::remove_dir_all(root);
    Ok(())
}

fn test_migration_create_checkpoint(tx: &rusqlite::Transaction<'_>) -> anyhow::Result<()> {
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS checkpoint_table(value TEXT NOT NULL);
             INSERT INTO checkpoint_table(value) VALUES ('ok');",
    )?;
    Ok(())
}

fn test_migration_fail(_tx: &rusqlite::Transaction<'_>) -> anyhow::Result<()> {
    anyhow::bail!("forced interruption")
}

fn test_migration_success(tx: &rusqlite::Transaction<'_>) -> anyhow::Result<()> {
    tx.execute_batch("CREATE TABLE IF NOT EXISTS resumed_table(id INTEGER PRIMARY KEY);")?;
    Ok(())
}

fn table_columns(conn: &Connection, table: &str) -> anyhow::Result<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

#[test]
fn migration_runner_recovers_after_interrupted_migration() -> anyhow::Result<()> {
    let root = temp_dir("rmu-migrations-recover");
    fs::create_dir_all(&root)?;
    let db_path = root.join("index.db");
    let mut conn = open_db(&db_path)?;
    conn.execute_batch(super::INIT_DB_SCHEMA_SQL)?;

    let failing = [
        SchemaMigration {
            id: 10,
            name: "checkpoint",
            apply: test_migration_create_checkpoint,
        },
        SchemaMigration {
            id: 11,
            name: "forced_fail",
            apply: test_migration_fail,
        },
    ];
    let err = apply_schema_migrations_plan(&mut conn, &db_path, false, &failing)
        .expect_err("second migration should fail");
    let err_text = err.to_string();
    assert!(
        err_text.contains("forced interruption") || err_text.contains("failed during apply"),
        "unexpected migration error: {err_text}"
    );

    let applied_after_fail: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id FROM schema_migrations ORDER BY id ASC")?;
        stmt.query_map([], |row| row.get::<_, i64>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    assert_eq!(applied_after_fail, vec![10]);

    let table_from_failed_step: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='resumed_table'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    assert!(table_from_failed_step.is_none());

    let resumed = [SchemaMigration {
        id: 11,
        name: "resume_after_failure",
        apply: test_migration_success,
    }];
    apply_schema_migrations_plan(&mut conn, &db_path, false, &resumed)?;
    let max_id: Option<i64> =
        conn.query_row("SELECT MAX(id) FROM schema_migrations", [], |row| {
            row.get(0)
        })?;
    assert_eq!(max_id, Some(11));

    let _ = fs::remove_dir_all(root);
    Ok(())
}
