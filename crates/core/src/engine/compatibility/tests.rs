use std::path::Path;

use super::{
    CURRENT_SCHEMA_VERSION, IndexCompatibilityDecision, ensure_schema_preflight,
    evaluate_index_compatibility, expected_index_meta, reconcile_schema_and_index_meta,
};
use crate::engine::schema::{INIT_DB_SCHEMA_SQL, apply_schema_migrations};
use rusqlite::{Connection, params};

fn setup_conn() -> anyhow::Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.execute_batch(INIT_DB_SCHEMA_SQL)?;
    apply_schema_migrations(&mut conn, Path::new(":memory:"), false)?;
    Ok(conn)
}

fn insert_dummy_file(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO files(path, sha256, size_bytes, language, sample, indexed_at_utc)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            "src/main.rs",
            "abc",
            1_i64,
            "rust",
            "fn main() {}",
            "2026-03-03T00:00:00Z"
        ],
    )?;
    Ok(())
}

#[test]
fn compatibility_matrix_fresh_db_is_compatible() -> anyhow::Result<()> {
    let conn = setup_conn()?;
    reconcile_schema_and_index_meta(&conn)?;
    let decision = evaluate_index_compatibility(&conn)?;
    assert_eq!(decision, IndexCompatibilityDecision::Compatible);
    Ok(())
}

#[test]
fn compatibility_matrix_legacy_index_requires_reindex() -> anyhow::Result<()> {
    let conn = setup_conn()?;
    insert_dummy_file(&conn)?;
    reconcile_schema_and_index_meta(&conn)?;
    let decision = evaluate_index_compatibility(&conn)?;
    assert!(decision.is_reindex_required());
    assert!(
        decision
            .reason()
            .is_some_and(|reason| reason.contains("index_format_version mismatch"))
    );
    Ok(())
}

#[test]
fn compatibility_matrix_matching_meta_is_compatible() -> anyhow::Result<()> {
    let conn = setup_conn()?;
    insert_dummy_file(&conn)?;
    let expected = expected_index_meta();
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["schema_version", CURRENT_SCHEMA_VERSION.to_string()],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![
            "index_format_version",
            expected.index_format_version.to_string()
        ],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["embedding_model_id", expected.embedding_model_id],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["embedding_dim", expected.embedding_dim.to_string()],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["ann_version", expected.ann_version.to_string()],
    )?;

    let decision = evaluate_index_compatibility(&conn)?;
    assert_eq!(decision, IndexCompatibilityDecision::Compatible);
    Ok(())
}

#[test]
fn compatibility_matrix_model_mismatch_requires_reindex() -> anyhow::Result<()> {
    let conn = setup_conn()?;
    insert_dummy_file(&conn)?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["schema_version", CURRENT_SCHEMA_VERSION.to_string()],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["index_format_version", "1"],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["embedding_model_id", "other-model"],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["embedding_dim", "192"],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["ann_version", "1"],
    )?;

    let decision = evaluate_index_compatibility(&conn)?;
    assert!(decision.is_reindex_required());
    assert!(
        decision
            .reason()
            .is_some_and(|reason| reason.contains("embedding_model_id mismatch"))
    );
    Ok(())
}

#[test]
fn compatibility_matrix_embedding_dim_mismatch_requires_reindex() -> anyhow::Result<()> {
    let conn = setup_conn()?;
    insert_dummy_file(&conn)?;
    let expected = expected_index_meta();
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["schema_version", CURRENT_SCHEMA_VERSION.to_string()],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![
            "index_format_version",
            expected.index_format_version.to_string()
        ],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["embedding_model_id", expected.embedding_model_id],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["embedding_dim", (expected.embedding_dim + 1).to_string()],
    )?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["ann_version", expected.ann_version.to_string()],
    )?;

    let decision = evaluate_index_compatibility(&conn)?;
    assert!(decision.is_reindex_required());
    assert!(
        decision
            .reason()
            .is_some_and(|reason| reason.contains("embedding_dim mismatch"))
    );
    Ok(())
}

#[test]
fn preflight_rejects_future_schema_version() -> anyhow::Result<()> {
    let conn = setup_conn()?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params!["schema_version", (CURRENT_SCHEMA_VERSION + 1).to_string()],
    )?;
    let err =
        ensure_schema_preflight(&conn).expect_err("future schema must be rejected with hard fail");
    assert!(err.to_string().contains("newer than binary supported"));
    Ok(())
}
