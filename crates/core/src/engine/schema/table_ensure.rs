use std::collections::HashSet;

use anyhow::Result;
use rusqlite::Connection;

use super::constants::REQUIRED_SCHEMA_TABLES;

pub(crate) fn required_schema_exists(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare("SELECT name FROM sqlite_master WHERE type = 'table'")?;
    let table_iter = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut existing = HashSet::new();
    for row in table_iter {
        existing.insert(row?);
    }
    Ok(REQUIRED_SCHEMA_TABLES
        .iter()
        .all(|required| existing.contains(*required)))
}

pub(super) fn ensure_schema_migrations_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS schema_migrations (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at_utc TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

pub(super) fn ensure_file_chunks_excerpt_column(conn: &Connection) -> Result<()> {
    let mut has_file_chunks = false;
    let mut has_excerpt = false;
    let mut stmt = conn.prepare("PRAGMA table_info(file_chunks)")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        has_file_chunks = true;
        if row?.eq_ignore_ascii_case("excerpt") {
            has_excerpt = true;
        }
    }

    if has_file_chunks && !has_excerpt {
        conn.execute(
            "ALTER TABLE file_chunks ADD COLUMN excerpt TEXT NOT NULL DEFAULT ''",
            [],
        )?;
    }
    Ok(())
}

pub(super) fn ensure_files_source_mtime_column(conn: &Connection) -> Result<()> {
    ensure_table_columns(conn, "files", &[("source_mtime_unix_ms", "INTEGER")])
}

pub(super) fn ensure_files_graph_count_columns(conn: &Connection) -> Result<()> {
    ensure_table_columns(
        conn,
        "files",
        &[
            ("graph_symbol_count", "INTEGER"),
            ("graph_ref_count", "INTEGER"),
            ("graph_module_dep_count", "INTEGER"),
        ],
    )
}

pub(super) fn ensure_files_graph_edge_columns(conn: &Connection) -> Result<()> {
    ensure_table_columns(
        conn,
        "files",
        &[
            ("graph_edge_out_count", "INTEGER"),
            ("graph_edge_in_count", "INTEGER"),
            ("graph_edge_hash", "TEXT"),
            ("graph_edge_fingerprint_version", "INTEGER"),
        ],
    )
}

pub(super) fn ensure_files_artifact_fingerprint_columns(conn: &Connection) -> Result<()> {
    ensure_table_columns(
        conn,
        "files",
        &[
            ("artifact_fingerprint_version", "INTEGER"),
            ("fts_sample_hash", "TEXT"),
            ("chunk_manifest_count", "INTEGER"),
            ("chunk_manifest_hash", "TEXT"),
            ("chunk_embedding_count", "INTEGER"),
            ("chunk_embedding_hash", "TEXT"),
            ("semantic_vector_hash", "TEXT"),
            ("ann_bucket_count", "INTEGER"),
            ("ann_bucket_hash", "TEXT"),
        ],
    )
}

pub(super) fn ensure_files_graph_fingerprint_columns(conn: &Connection) -> Result<()> {
    ensure_table_columns(
        conn,
        "files",
        &[
            ("graph_content_hash", "TEXT"),
            ("graph_fingerprint_version", "INTEGER"),
        ],
    )
}

pub(super) fn ensure_symbols_position_columns(conn: &Connection) -> Result<()> {
    ensure_table_columns(
        conn,
        "symbols",
        &[("line", "INTEGER"), ("column", "INTEGER")],
    )
}

pub(super) fn ensure_refs_position_columns(conn: &Connection) -> Result<()> {
    ensure_table_columns(conn, "refs", &[("line", "INTEGER"), ("column", "INTEGER")])
}

pub(super) fn ensure_semantic_ann_buckets_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS semantic_ann_buckets (
            path TEXT NOT NULL,
            model TEXT NOT NULL,
            bucket_family INTEGER NOT NULL,
            bucket_key TEXT NOT NULL,
            PRIMARY KEY(path, model, bucket_family)
        );
        CREATE INDEX IF NOT EXISTS idx_semantic_ann_lookup
            ON semantic_ann_buckets(model, bucket_family, bucket_key);
        "#,
    )?;
    Ok(())
}

pub(super) fn ensure_file_graph_edges_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS file_graph_edges (
            src_path TEXT NOT NULL,
            dst_path TEXT NOT NULL,
            edge_kind TEXT NOT NULL,
            raw_count INTEGER NOT NULL,
            weight REAL NOT NULL,
            PRIMARY KEY(src_path, dst_path, edge_kind)
        );
        CREATE INDEX IF NOT EXISTS idx_file_graph_edges_src ON file_graph_edges(src_path);
        CREATE INDEX IF NOT EXISTS idx_file_graph_edges_dst ON file_graph_edges(dst_path);
        "#,
    )?;
    Ok(())
}

pub(super) fn ensure_file_quality_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS file_quality (
            path TEXT PRIMARY KEY,
            language TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            total_lines INTEGER,
            non_empty_lines INTEGER,
            import_count INTEGER,
            quality_mode TEXT NOT NULL,
            source_mtime_unix_ms INTEGER,
            quality_ruleset_version INTEGER NOT NULL,
            quality_metric_count INTEGER NOT NULL DEFAULT 0,
            quality_metric_hash TEXT NOT NULL DEFAULT '',
            quality_violation_count INTEGER NOT NULL,
            quality_violation_hash TEXT NOT NULL,
            quality_indexed_at_utc TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_file_quality_language ON file_quality(language);
        CREATE INDEX IF NOT EXISTS idx_file_quality_violation_count
            ON file_quality(quality_violation_count);

        CREATE TABLE IF NOT EXISTS file_quality_metrics (
            path TEXT NOT NULL,
            metric_id TEXT NOT NULL,
            metric_value INTEGER NOT NULL,
            PRIMARY KEY(path, metric_id)
        );
        CREATE INDEX IF NOT EXISTS idx_file_quality_metrics_metric
            ON file_quality_metrics(metric_id);

        CREATE TABLE IF NOT EXISTS file_rule_violations (
            path TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            actual_value INTEGER NOT NULL,
            threshold_value INTEGER NOT NULL,
            message TEXT NOT NULL,
            start_line INTEGER,
            start_column INTEGER,
            end_line INTEGER,
            end_column INTEGER,
            PRIMARY KEY(path, rule_id)
        );
        CREATE INDEX IF NOT EXISTS idx_file_rule_violations_rule
            ON file_rule_violations(rule_id);
        "#,
    )?;
    ensure_table_columns(
        conn,
        "file_quality",
        &[
            ("quality_metric_count", "INTEGER NOT NULL DEFAULT 0"),
            ("quality_metric_hash", "TEXT NOT NULL DEFAULT ''"),
        ],
    )?;
    ensure_table_columns(
        conn,
        "file_rule_violations",
        &[
            ("start_line", "INTEGER"),
            ("start_column", "INTEGER"),
            ("end_line", "INTEGER"),
            ("end_column", "INTEGER"),
        ],
    )?;
    Ok(())
}

fn ensure_table_columns(
    conn: &Connection,
    table_name: &str,
    columns: &[(&str, &str)],
) -> Result<()> {
    let mut has_table = false;
    let mut existing_columns = HashSet::new();
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut stmt = conn.prepare(&pragma)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for row in rows {
        has_table = true;
        existing_columns.insert(row?);
    }

    if !has_table {
        return Ok(());
    }

    for (column_name, column_type) in columns {
        if existing_columns.contains(*column_name) {
            continue;
        }
        conn.execute(
            &format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {column_type}"),
            [],
        )?;
    }

    Ok(())
}
