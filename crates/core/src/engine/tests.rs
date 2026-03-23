use super::Engine;
use super::compatibility;
use super::context;
use super::schema;
use crate::graph::CURRENT_GRAPH_FINGERPRINT_VERSION;
use crate::model::IndexingOptions;
use crate::model::MigrationMode;
use crate::model::SearchHit;
use rusqlite::{Connection, OptionalExtension, params};
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use time::{Duration, OffsetDateTime};

fn hit(path: &str, preview: &str, score: f32) -> SearchHit {
    SearchHit {
        path: path.to_string(),
        preview: preview.to_string(),
        score,
        size_bytes: 0,
        language: "rust".to_string(),
    }
}

#[test]
fn context_selection_skips_oversized_hit_and_keeps_fitting_later_hits() {
    use std::collections::HashMap;

    let hits = vec![
        hit("a.rs", &"X".repeat(200), 1.0),
        hit("b.rs", "small-one", 0.9),
        hit("c.rs", "small-two", 0.8),
    ];

    let selected = context::context_from_hits(&hits, &HashMap::new(), None, 40, 40);
    assert_eq!(selected.files.len(), 2);
    assert_eq!(selected.files[0].path, "b.rs");
    assert_eq!(selected.files[1].path, "c.rs");
    assert!(selected.truncated);
}

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

fn file_graph_counts(
    engine: &Engine,
    path: &str,
) -> anyhow::Result<(Option<i64>, Option<i64>, Option<i64>)> {
    let conn = engine.open_db()?;
    let counts = conn.query_row(
        "SELECT graph_symbol_count, graph_ref_count, graph_module_dep_count FROM files WHERE path = ?1",
        [path],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;
    Ok(counts)
}

fn file_graph_fingerprint(
    engine: &Engine,
    path: &str,
) -> anyhow::Result<(Option<String>, Option<i64>)> {
    let conn = engine.open_db()?;
    let fingerprint = conn.query_row(
        "SELECT graph_content_hash, graph_fingerprint_version FROM files WHERE path = ?1",
        [path],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(fingerprint)
}

fn actual_graph_counts(engine: &Engine, path: &str) -> anyhow::Result<(i64, i64, i64)> {
    let conn = engine.open_db()?;
    let symbol_count = conn.query_row(
        "SELECT COUNT(1) FROM symbols WHERE path = ?1",
        [path],
        |row| row.get(0),
    )?;
    let ref_count = conn.query_row("SELECT COUNT(1) FROM refs WHERE path = ?1", [path], |row| {
        row.get(0)
    })?;
    let dep_count = conn.query_row(
        "SELECT COUNT(1) FROM module_deps WHERE path = ?1",
        [path],
        |row| row.get(0),
    )?;
    Ok((symbol_count, ref_count, dep_count))
}

fn null_graph_counts(engine: &Engine, path: &str) -> anyhow::Result<()> {
    let conn = engine.open_db()?;
    conn.execute(
        "UPDATE files
         SET graph_symbol_count = NULL, graph_ref_count = NULL, graph_module_dep_count = NULL
         WHERE path = ?1",
        [path],
    )?;
    Ok(())
}

fn null_graph_fingerprint(engine: &Engine, path: &str) -> anyhow::Result<()> {
    let conn = engine.open_db()?;
    conn.execute(
        "UPDATE files
         SET graph_content_hash = NULL, graph_fingerprint_version = NULL
         WHERE path = ?1",
        [path],
    )?;
    Ok(())
}

fn delete_graph_rows(engine: &Engine, table: &str, path: &str) -> anyhow::Result<()> {
    let conn = engine.open_db()?;
    conn.execute(&format!("DELETE FROM {table} WHERE path = ?1"), [path])?;
    Ok(())
}

fn corrupt_first_symbol_name(engine: &Engine, path: &str, replacement: &str) -> anyhow::Result<()> {
    let conn = engine.open_db()?;
    conn.execute(
        "UPDATE symbols
         SET name = ?2
         WHERE id = (SELECT MIN(id) FROM symbols WHERE path = ?1)",
        params![path, replacement],
    )?;
    Ok(())
}

fn corrupt_first_module_dep(engine: &Engine, path: &str, replacement: &str) -> anyhow::Result<()> {
    let conn = engine.open_db()?;
    conn.execute(
        "UPDATE module_deps
         SET dep = ?2
         WHERE id = (SELECT MIN(id) FROM module_deps WHERE path = ?1)",
        params![path, replacement],
    )?;
    Ok(())
}

fn corrupt_ref_language(engine: &Engine, path: &str, replacement: &str) -> anyhow::Result<()> {
    let conn = engine.open_db()?;
    conn.execute(
        "UPDATE refs
         SET language = ?2
         WHERE id = (SELECT MIN(id) FROM refs WHERE path = ?1)",
        params![path, replacement],
    )?;
    Ok(())
}

#[test]
fn future_schema_version_hard_fails_without_meta_writes() -> anyhow::Result<()> {
    let root = temp_dir("rmu-future-schema");
    fs::create_dir_all(&root)?;
    let db_path = root.join(".rmu/index.db");
    let parent = db_path.parent().expect("db parent should exist");
    fs::create_dir_all(parent)?;

    let conn = Connection::open(&db_path)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)",
        params!["schema_version", "999"],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)",
        params!["marker", "immutable"],
    )?;
    drop(conn);

    let err = Engine::new(root.clone(), Some(db_path.clone()))
        .expect_err("future schema version must hard-fail");
    assert!(err.to_string().contains("newer than binary supported"));

    let verify = Connection::open(&db_path)?;
    let schema_version: Option<String> = verify
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let marker: Option<String> = verify
        .query_row("SELECT value FROM meta WHERE key = 'marker'", [], |row| {
            row.get(0)
        })
        .optional()?;
    let project_root: Option<String> = verify
        .query_row(
            "SELECT value FROM meta WHERE key = 'project_root'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    assert_eq!(schema_version.as_deref(), Some("999"));
    assert_eq!(marker.as_deref(), Some("immutable"));
    assert!(
        project_root.is_none(),
        "hard-fail must not write access metadata"
    );

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn migration_mode_off_rejects_missing_database() {
    let root = temp_dir("rmu-migration-mode-off-missing");
    let db_path = root.join(".rmu/index.db");
    let err = Engine::new_with_migration_mode(root.clone(), Some(db_path), MigrationMode::Off)
        .expect_err("migration_mode=off must reject missing database");
    assert!(
        err.to_string()
            .contains("migration_mode=off requires pre-existing initialized database")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn migration_mode_off_accepts_initialized_database() -> anyhow::Result<()> {
    let root = temp_dir("rmu-migration-mode-off-existing");
    fs::create_dir_all(&root)?;
    let db_path = root.join(".rmu/index.db");
    fs::create_dir_all(db_path.parent().expect("db parent must exist"))?;

    let mut conn = Connection::open(&db_path)?;
    conn.execute_batch(schema::INIT_DB_SCHEMA_SQL)?;
    schema::apply_schema_migrations(&mut conn, &db_path, false)?;
    compatibility::reconcile_schema_and_index_meta(&conn)?;
    drop(conn);

    let engine =
        Engine::new_with_migration_mode(root.clone(), Some(db_path.clone()), MigrationMode::Off)?;
    let status = engine.index_status()?;
    assert_eq!(status.files, 0);

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn changed_since_backfills_null_graph_counts_even_when_cutoff_would_skip() -> anyhow::Result<()> {
    let root = temp_dir("rmu-graph-count-backfill");
    fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "src/main.rs",
        "use crate::shared::helper;\n\npub struct Widget;\n\nfn entry() {\n    helper();\n    Widget::build();\n}\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    let first = engine.index_path()?;
    assert_eq!(first.indexed, 1);

    null_graph_counts(&engine, "src/main.rs")?;
    let repaired = engine.index_path_with_options(&IndexingOptions {
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..IndexingOptions::default()
    })?;

    assert_eq!(repaired.indexed, 1);
    assert_eq!(repaired.changed, 1);
    assert_eq!(repaired.skipped_before_changed_since, 0);

    let stored = file_graph_counts(&engine, "src/main.rs")?;
    let actual = actual_graph_counts(&engine, "src/main.rs")?;
    assert_eq!(stored.0, Some(actual.0));
    assert_eq!(stored.1, Some(actual.1));
    assert_eq!(stored.2, Some(actual.2));

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn changed_since_backfills_null_graph_hash_and_version_even_when_cutoff_would_skip()
-> anyhow::Result<()> {
    let root = temp_dir("rmu-graph-fingerprint-backfill");
    fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "src/main.rs",
        "use crate::shared::helper;\n\npub struct Widget;\n\nfn entry() {\n    helper();\n    Widget::build();\n}\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;
    let before = file_graph_fingerprint(&engine, "src/main.rs")?;
    assert_eq!(before.1, Some(CURRENT_GRAPH_FINGERPRINT_VERSION));

    null_graph_fingerprint(&engine, "src/main.rs")?;
    let repaired = engine.index_path_with_options(&IndexingOptions {
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..IndexingOptions::default()
    })?;
    assert_eq!(repaired.indexed, 1);
    assert_eq!(repaired.changed, 1);
    assert_eq!(repaired.skipped_before_changed_since, 0);

    let after = file_graph_fingerprint(&engine, "src/main.rs")?;
    assert_eq!(after, before);

    let verify_skip = engine.index_path_with_options(&IndexingOptions {
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..IndexingOptions::default()
    })?;
    assert_eq!(verify_skip.indexed, 0);
    assert_eq!(verify_skip.skipped_before_changed_since, 1);

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn changed_since_repairs_deleted_graph_rows_on_unchanged_file() -> anyhow::Result<()> {
    let root = temp_dir("rmu-graph-row-repair");
    fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "src/main.rs",
        "use crate::shared::helper;\n\npub struct Widget;\n\nfn entry() {\n    helper();\n    Widget::build();\n}\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let before = file_graph_counts(&engine, "src/main.rs")?;
    assert!(
        before.1.unwrap_or_default() > 0,
        "fixture must produce at least one ref row"
    );

    delete_graph_rows(&engine, "refs", "src/main.rs")?;

    let repaired = engine.index_path_with_options(&IndexingOptions {
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..IndexingOptions::default()
    })?;
    assert_eq!(repaired.indexed, 1);
    assert_eq!(repaired.changed, 1);
    assert_eq!(repaired.skipped_before_changed_since, 0);

    let after = file_graph_counts(&engine, "src/main.rs")?;
    let actual = actual_graph_counts(&engine, "src/main.rs")?;
    assert_eq!(after.0, Some(actual.0));
    assert_eq!(after.1, Some(actual.1));
    assert_eq!(after.2, Some(actual.2));
    assert_eq!(after.1, before.1);

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn changed_since_repairs_same_count_symbol_corruption_on_unchanged_file() -> anyhow::Result<()> {
    let root = temp_dir("rmu-graph-symbol-corruption");
    fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "src/main.rs",
        "use crate::shared::helper;\n\npub struct Widget;\n\nfn entry() {\n    helper();\n    Widget::build();\n}\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;
    let before = file_graph_fingerprint(&engine, "src/main.rs")?;

    corrupt_first_symbol_name(&engine, "src/main.rs", "CorruptedSymbol")?;

    let repaired = engine.index_path_with_options(&IndexingOptions {
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..IndexingOptions::default()
    })?;
    assert_eq!(repaired.indexed, 1);
    assert_eq!(repaired.changed, 1);
    assert_eq!(repaired.skipped_before_changed_since, 0);
    assert_eq!(file_graph_fingerprint(&engine, "src/main.rs")?, before);

    let verify_skip = engine.index_path_with_options(&IndexingOptions {
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..IndexingOptions::default()
    })?;
    assert_eq!(verify_skip.indexed, 0);
    assert_eq!(verify_skip.skipped_before_changed_since, 1);

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn changed_since_repairs_same_count_module_dep_corruption_on_unchanged_file() -> anyhow::Result<()>
{
    let root = temp_dir("rmu-graph-dep-corruption");
    fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "src/main.rs",
        "use crate::shared::helper;\n\npub struct Widget;\n\nfn entry() {\n    helper();\n    Widget::build();\n}\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;
    let before = file_graph_fingerprint(&engine, "src/main.rs")?;

    corrupt_first_module_dep(&engine, "src/main.rs", "crate::shared::corrupted")?;

    let repaired = engine.index_path_with_options(&IndexingOptions {
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..IndexingOptions::default()
    })?;
    assert_eq!(repaired.indexed, 1);
    assert_eq!(repaired.changed, 1);
    assert_eq!(repaired.skipped_before_changed_since, 0);
    assert_eq!(file_graph_fingerprint(&engine, "src/main.rs")?, before);

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn changed_since_repairs_same_count_graph_language_corruption_on_unchanged_file()
-> anyhow::Result<()> {
    let root = temp_dir("rmu-graph-language-corruption");
    fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "src/main.rs",
        "use crate::shared::helper;\n\npub struct Widget;\n\nfn entry() {\n    helper();\n    Widget::build();\n}\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;
    let before = file_graph_fingerprint(&engine, "src/main.rs")?;

    corrupt_ref_language(&engine, "src/main.rs", "python")?;

    let repaired = engine.index_path_with_options(&IndexingOptions {
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..IndexingOptions::default()
    })?;
    assert_eq!(repaired.indexed, 1);
    assert_eq!(repaired.changed, 1);
    assert_eq!(repaired.skipped_before_changed_since, 0);
    assert_eq!(file_graph_fingerprint(&engine, "src/main.rs")?, before);

    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn changed_since_does_not_false_positive_on_legitimate_zero_graph_output() -> anyhow::Result<()> {
    let root = temp_dir("rmu-zero-graph-output");
    fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "notes.txt",
        "plain text without any language graph\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    let first = engine.index_path_with_options(&IndexingOptions {
        profile: Some(crate::model::IndexProfile::DocsHeavy),
        ..IndexingOptions::default()
    })?;
    assert_eq!(first.indexed, 1);
    assert_eq!(
        file_graph_counts(&engine, "notes.txt")?,
        (Some(0), Some(0), Some(0))
    );

    let second = engine.index_path_with_options(&IndexingOptions {
        profile: Some(crate::model::IndexProfile::DocsHeavy),
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..IndexingOptions::default()
    })?;
    assert_eq!(second.indexed, 0);
    assert_eq!(second.skipped_before_changed_since, 1);
    assert_eq!(
        file_graph_counts(&engine, "notes.txt")?,
        (Some(0), Some(0), Some(0))
    );

    let _ = fs::remove_dir_all(root);
    Ok(())
}
