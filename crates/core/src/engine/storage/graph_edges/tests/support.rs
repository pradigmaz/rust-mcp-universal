use std::collections::{HashMap, HashSet};

use crate::graph::{CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION, empty_graph_edge_content_hash};
use rusqlite::{Connection, params};

use super::super::{capture_graph_refresh_seed, rebuild_file_graph_edges, refresh_file_graph_edges};

pub(super) type EdgeRow = (String, String, String, i64, f64);
pub(super) type MetadataRow = (String, i64, i64, String, i64);

pub(super) fn setup_graph_edge_schema(conn: &Connection) -> anyhow::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE files (
            path TEXT PRIMARY KEY,
            graph_edge_out_count INTEGER,
            graph_edge_in_count INTEGER,
            graph_edge_hash TEXT,
            graph_edge_fingerprint_version INTEGER
        );
        CREATE TABLE symbols (
            path TEXT NOT NULL,
            name TEXT NOT NULL
        );
        CREATE TABLE refs (
            path TEXT NOT NULL,
            symbol TEXT NOT NULL
        );
        CREATE TABLE module_deps (
            path TEXT NOT NULL,
            dep TEXT NOT NULL
        );
        CREATE TABLE file_graph_edges (
            src_path TEXT NOT NULL,
            dst_path TEXT NOT NULL,
            edge_kind TEXT NOT NULL,
            raw_count INTEGER NOT NULL,
            weight REAL NOT NULL
        );
        "#,
    )?;
    Ok(())
}

pub(super) fn insert_file(conn: &Connection, path: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO files(path, graph_edge_out_count, graph_edge_in_count, graph_edge_hash, graph_edge_fingerprint_version)
         VALUES (?1, -1, -1, 'stale', -1)",
        [path],
    )?;
    Ok(())
}

pub(super) fn fetch_edges(conn: &Connection) -> anyhow::Result<Vec<EdgeRow>> {
    let mut stmt = conn.prepare(
        "SELECT src_path, dst_path, edge_kind, raw_count, weight
         FROM file_graph_edges
         ORDER BY src_path ASC, dst_path ASC, edge_kind ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub(super) fn fetch_metadata(conn: &Connection) -> anyhow::Result<Vec<MetadataRow>> {
    let mut stmt = conn.prepare(
        "SELECT path, graph_edge_out_count, graph_edge_in_count, graph_edge_hash, graph_edge_fingerprint_version
         FROM files
         ORDER BY path ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i64>(4)?,
        ))
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub(super) fn seed_delta_equivalence_fixture(conn: &Connection) -> anyhow::Result<()> {
    for path in [
        "src/caller.rs",
        "src/dirty.rs",
        "src/owner.rs",
        "src/old_dst.rs",
        "src/new_dst.rs",
        "src/dep_old.rs",
        "src/dep_new.rs",
    ] {
        insert_file(conn, path)?;
    }

    conn.execute(
        "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
        params!["src/owner.rs", "Helper"],
    )?;
    conn.execute(
        "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
        params!["src/old_dst.rs", "OldTarget"],
    )?;
    conn.execute(
        "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
        params!["src/caller.rs", "crate::nested::Helper"],
    )?;
    conn.execute(
        "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
        params!["src/dirty.rs", "OldTarget"],
    )?;
    conn.execute(
        "INSERT INTO module_deps(path, dep) VALUES (?1, ?2)",
        params!["src/dirty.rs", "serde"],
    )?;
    conn.execute(
        "INSERT INTO module_deps(path, dep) VALUES (?1, ?2)",
        params!["src/dep_old.rs", "serde"],
    )?;
    Ok(())
}

pub(super) fn mutate_delta_equivalence_fixture(conn: &Connection) -> anyhow::Result<()> {
    conn.execute("DELETE FROM refs WHERE path = ?1", ["src/dirty.rs"])?;
    conn.execute(
        "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
        params!["src/dirty.rs", "NewTarget"],
    )?;
    conn.execute(
        "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
        params!["src/dirty.rs", "Helper"],
    )?;
    conn.execute(
        "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
        params!["src/new_dst.rs", "NewTarget"],
    )?;
    conn.execute("DELETE FROM module_deps WHERE path = ?1", ["src/dirty.rs"])?;
    conn.execute(
        "INSERT INTO module_deps(path, dep) VALUES (?1, ?2)",
        params!["src/dirty.rs", "chrono"],
    )?;
    conn.execute(
        "INSERT INTO module_deps(path, dep) VALUES (?1, ?2)",
        params!["src/dep_new.rs", "chrono"],
    )?;
    Ok(())
}

pub(super) fn prepare_dirty_delta_fixture(
    delta_conn: &mut Connection,
    full_conn: &mut Connection,
) -> anyhow::Result<(HashSet<String>, HashMap<String, super::super::GraphRefreshSeed>)> {
    setup_graph_edge_schema(delta_conn)?;
    setup_graph_edge_schema(full_conn)?;
    seed_delta_equivalence_fixture(delta_conn)?;
    seed_delta_equivalence_fixture(full_conn)?;

    let dirty_seed = {
        let tx = delta_conn.transaction()?;
        rebuild_file_graph_edges(&tx)?;
        let seed = capture_graph_refresh_seed(&tx, "src/dirty.rs")?;
        tx.commit()?;
        seed
    };
    {
        let tx = full_conn.transaction()?;
        rebuild_file_graph_edges(&tx)?;
        tx.commit()?;
    }

    mutate_delta_equivalence_fixture(delta_conn)?;
    mutate_delta_equivalence_fixture(full_conn)?;

    let mut dirty_paths = HashSet::new();
    dirty_paths.insert("src/dirty.rs".to_string());
    let mut pre_refresh = HashMap::new();
    pre_refresh.insert("src/dirty.rs".to_string(), dirty_seed);
    Ok((dirty_paths, pre_refresh))
}

pub(super) fn run_delta_refresh(
    conn: &mut Connection,
    dirty_paths: &HashSet<String>,
    pre_refresh: &HashMap<String, super::super::GraphRefreshSeed>,
) -> anyhow::Result<()> {
    let tx = conn.transaction()?;
    refresh_file_graph_edges(&tx, dirty_paths, pre_refresh)?;
    tx.commit()?;
    Ok(())
}

pub(super) fn run_full_rebuild(conn: &mut Connection) -> anyhow::Result<()> {
    let tx = conn.transaction()?;
    rebuild_file_graph_edges(&tx)?;
    tx.commit()?;
    Ok(())
}

pub(super) fn assert_edge_weight(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-6,
        "expected weight {expected}, got {actual}"
    );
}

pub(super) fn assert_reset_metadata(
    conn: &Connection,
    expected_len: usize,
) -> anyhow::Result<()> {
    let empty_hash = empty_graph_edge_content_hash();
    let mut stmt = conn.prepare(
        "SELECT path, graph_edge_out_count, graph_edge_in_count, graph_edge_hash, graph_edge_fingerprint_version
         FROM files
         ORDER BY path ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    assert_eq!(rows.len(), expected_len);
    assert_eq!(rows[0].0, "src/a.rs");
    assert_eq!(rows[0].1, 1);
    assert_eq!(rows[0].2, 0);
    assert_eq!(rows[0].4, CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION);
    assert_ne!(rows[0].3, empty_hash);

    assert_eq!(rows[1].0, "src/b.rs");
    assert_eq!(rows[1].1, 0);
    assert_eq!(rows[1].2, 1);
    assert_eq!(rows[1].4, CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION);
    assert_ne!(rows[1].3, empty_hash);

    assert_eq!(rows[2].0, "src/c.rs");
    assert_eq!(rows[2].1, 0);
    assert_eq!(rows[2].2, 0);
    assert_eq!(rows[2].3, empty_hash);
    assert_eq!(rows[2].4, CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION);
    Ok(())
}
