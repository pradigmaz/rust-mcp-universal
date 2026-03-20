use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use rusqlite::params;

use super::graph_state::load_actual_graph_edge_state;
use crate::graph::{CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION, empty_graph_edge_content_hash};
use crate::text_utils::symbol_tail;

const REF_EXACT_EDGE_WEIGHT: f32 = 1.0;
const REF_TAIL_UNIQUE_EDGE_WEIGHT: f32 = 0.72;
const SHARED_DEP_EDGE_WEIGHT: f32 = 0.35;

pub(in crate::engine) fn rebuild_file_graph_edges(tx: &rusqlite::Transaction<'_>) -> Result<()> {
    let symbol_destinations = load_symbol_destinations(tx)?;
    let ref_rows = load_ref_rows(tx)?;
    let dep_groups = load_dep_groups(tx)?;
    let mut edges = BTreeMap::<(String, String, String), (i64, f32)>::new();

    for (src_path, symbol) in ref_rows {
        if let Some(dest_paths) = symbol_destinations.get(&symbol) {
            for dst_path in dest_paths {
                if dst_path == &src_path {
                    continue;
                }
                add_materialized_edge(
                    &mut edges,
                    &src_path,
                    dst_path,
                    "ref_exact",
                    REF_EXACT_EDGE_WEIGHT,
                );
            }
            continue;
        }

        let tail = symbol_tail(&symbol);
        if tail == symbol {
            continue;
        }
        let Some(dest_paths) = symbol_destinations.get(tail) else {
            continue;
        };
        if dest_paths.len() != 1 {
            continue;
        }
        let dst_path = &dest_paths[0];
        if dst_path == &src_path {
            continue;
        }
        add_materialized_edge(
            &mut edges,
            &src_path,
            dst_path,
            "ref_tail_unique",
            REF_TAIL_UNIQUE_EDGE_WEIGHT,
        );
    }

    for paths in dep_groups.into_values() {
        if paths.len() < 2 {
            continue;
        }
        for src_idx in 0..paths.len() {
            for dst_idx in 0..paths.len() {
                if src_idx == dst_idx {
                    continue;
                }
                add_materialized_edge(
                    &mut edges,
                    &paths[src_idx],
                    &paths[dst_idx],
                    "shared_dep",
                    SHARED_DEP_EDGE_WEIGHT,
                );
            }
        }
    }

    tx.execute_batch(
        r#"
        DROP TABLE IF EXISTS temp.file_graph_edges_rebuild;
        CREATE TEMP TABLE file_graph_edges_rebuild (
            src_path TEXT NOT NULL,
            dst_path TEXT NOT NULL,
            edge_kind TEXT NOT NULL,
            raw_count INTEGER NOT NULL,
            weight REAL NOT NULL,
            PRIMARY KEY(src_path, dst_path, edge_kind)
        );
        "#,
    )?;
    for ((src_path, dst_path, edge_kind), (raw_count, weight)) in edges {
        tx.execute(
            "INSERT INTO file_graph_edges_rebuild(src_path, dst_path, edge_kind, raw_count, weight)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![src_path, dst_path, edge_kind, raw_count, weight],
        )?;
    }
    tx.execute("DELETE FROM file_graph_edges", [])?;
    tx.execute(
        "INSERT INTO file_graph_edges(src_path, dst_path, edge_kind, raw_count, weight)
         SELECT src_path, dst_path, edge_kind, raw_count, weight
         FROM file_graph_edges_rebuild",
        [],
    )?;
    tx.execute("DROP TABLE file_graph_edges_rebuild", [])?;
    refresh_graph_edge_metadata(tx)?;
    Ok(())
}

fn refresh_graph_edge_metadata(tx: &rusqlite::Transaction<'_>) -> Result<()> {
    let empty_hash = empty_graph_edge_content_hash();
    tx.execute(
        "UPDATE files
         SET graph_edge_out_count = 0,
             graph_edge_in_count = 0,
             graph_edge_hash = ?1,
             graph_edge_fingerprint_version = ?2",
        params![&empty_hash, CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION],
    )?;

    let states = load_actual_graph_edge_state(tx)?;
    for (path, state) in states {
        tx.execute(
            "UPDATE files
             SET graph_edge_out_count = ?2,
                 graph_edge_in_count = ?3,
                 graph_edge_hash = ?4,
                 graph_edge_fingerprint_version = ?5
             WHERE path = ?1",
            params![
                path,
                state.outgoing_count,
                state.incoming_count,
                state.content_hash,
                CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION
            ],
        )?;
    }
    Ok(())
}

fn load_symbol_destinations(
    tx: &rusqlite::Transaction<'_>,
) -> Result<HashMap<String, Vec<String>>> {
    let mut by_symbol = BTreeMap::<String, BTreeMap<String, ()>>::new();
    let mut stmt = tx.prepare("SELECT name, path FROM symbols ORDER BY name ASC, path ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (name, path) = row?;
        by_symbol.entry(name).or_default().insert(path, ());
    }

    Ok(by_symbol
        .into_iter()
        .map(|(name, paths)| (name, paths.into_keys().collect()))
        .collect())
}

fn load_ref_rows(tx: &rusqlite::Transaction<'_>) -> Result<Vec<(String, String)>> {
    let mut stmt = tx.prepare("SELECT path, symbol FROM refs ORDER BY path ASC, symbol ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn load_dep_groups(tx: &rusqlite::Transaction<'_>) -> Result<HashMap<String, Vec<String>>> {
    let mut by_dep = BTreeMap::<String, BTreeMap<String, ()>>::new();
    let mut stmt = tx.prepare("SELECT dep, path FROM module_deps ORDER BY dep ASC, path ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (dep, path) = row?;
        by_dep.entry(dep).or_default().insert(path, ());
    }

    Ok(by_dep
        .into_iter()
        .map(|(dep, paths)| (dep, paths.into_keys().collect()))
        .collect())
}

fn add_materialized_edge(
    edges: &mut BTreeMap<(String, String, String), (i64, f32)>,
    src_path: &str,
    dst_path: &str,
    edge_kind: &str,
    base_weight: f32,
) {
    let entry = edges
        .entry((
            src_path.to_string(),
            dst_path.to_string(),
            edge_kind.to_string(),
        ))
        .or_insert((0, 0.0));
    entry.0 += 1;
    entry.1 += base_weight;
}

#[cfg(test)]
mod tests {
    use super::rebuild_file_graph_edges;
    use crate::graph::{CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION, empty_graph_edge_content_hash};
    use rusqlite::{Connection, params};

    type EdgeRow = (String, String, String, i64, f64);

    fn setup_graph_edge_schema(conn: &Connection) -> anyhow::Result<()> {
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

    fn insert_file(conn: &Connection, path: &str) -> anyhow::Result<()> {
        conn.execute(
            "INSERT INTO files(path, graph_edge_out_count, graph_edge_in_count, graph_edge_hash, graph_edge_fingerprint_version)
             VALUES (?1, -1, -1, 'stale', -1)",
            [path],
        )?;
        Ok(())
    }

    fn fetch_edges(conn: &Connection) -> anyhow::Result<Vec<EdgeRow>> {
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

    fn assert_edge_weight(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected weight {expected}, got {actual}"
        );
    }

    #[test]
    fn rebuild_file_graph_edges_materializes_ref_exact_edges() -> anyhow::Result<()> {
        let mut conn = Connection::open_in_memory()?;
        setup_graph_edge_schema(&conn)?;
        insert_file(&conn, "src/a.rs")?;
        insert_file(&conn, "src/b.rs")?;
        conn.execute(
            "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
            params!["src/b.rs", "Helper"],
        )?;
        conn.execute(
            "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
            params!["src/a.rs", "Helper"],
        )?;

        let tx = conn.transaction()?;
        rebuild_file_graph_edges(&tx)?;
        tx.commit()?;

        assert_eq!(
            fetch_edges(&conn)?,
            vec![(
                "src/a.rs".to_string(),
                "src/b.rs".to_string(),
                "ref_exact".to_string(),
                1,
                1.0
            )]
        );
        Ok(())
    }

    #[test]
    fn rebuild_file_graph_edges_materializes_ref_tail_unique_edges() -> anyhow::Result<()> {
        let mut conn = Connection::open_in_memory()?;
        setup_graph_edge_schema(&conn)?;
        insert_file(&conn, "src/a.rs")?;
        insert_file(&conn, "src/b.rs")?;
        conn.execute(
            "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
            params!["src/b.rs", "Helper"],
        )?;
        conn.execute(
            "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
            params!["src/a.rs", "crate::nested::Helper"],
        )?;

        let tx = conn.transaction()?;
        rebuild_file_graph_edges(&tx)?;
        tx.commit()?;

        let edges = fetch_edges(&conn)?;
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].0, "src/a.rs");
        assert_eq!(edges[0].1, "src/b.rs");
        assert_eq!(edges[0].2, "ref_tail_unique");
        assert_eq!(edges[0].3, 1);
        assert_edge_weight(edges[0].4, 0.72);
        Ok(())
    }

    #[test]
    fn rebuild_file_graph_edges_materializes_shared_dep_edges() -> anyhow::Result<()> {
        let mut conn = Connection::open_in_memory()?;
        setup_graph_edge_schema(&conn)?;
        insert_file(&conn, "src/a.rs")?;
        insert_file(&conn, "src/b.rs")?;
        conn.execute(
            "INSERT INTO module_deps(path, dep) VALUES (?1, ?2)",
            params!["src/a.rs", "serde"],
        )?;
        conn.execute(
            "INSERT INTO module_deps(path, dep) VALUES (?1, ?2)",
            params!["src/b.rs", "serde"],
        )?;

        let tx = conn.transaction()?;
        rebuild_file_graph_edges(&tx)?;
        tx.commit()?;

        let edges = fetch_edges(&conn)?;
        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].0, "src/a.rs");
        assert_eq!(edges[0].1, "src/b.rs");
        assert_eq!(edges[0].2, "shared_dep");
        assert_eq!(edges[0].3, 1);
        assert_edge_weight(edges[0].4, 0.35);
        assert_eq!(edges[1].0, "src/b.rs");
        assert_eq!(edges[1].1, "src/a.rs");
        assert_eq!(edges[1].2, "shared_dep");
        assert_eq!(edges[1].3, 1);
        assert_edge_weight(edges[1].4, 0.35);
        Ok(())
    }

    #[test]
    fn rebuild_file_graph_edges_skips_self_edges() -> anyhow::Result<()> {
        let mut conn = Connection::open_in_memory()?;
        setup_graph_edge_schema(&conn)?;
        insert_file(&conn, "src/a.rs")?;
        conn.execute(
            "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
            params!["src/a.rs", "Helper"],
        )?;
        conn.execute(
            "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
            params!["src/a.rs", "Helper"],
        )?;

        let tx = conn.transaction()?;
        rebuild_file_graph_edges(&tx)?;
        tx.commit()?;

        assert!(fetch_edges(&conn)?.is_empty());
        Ok(())
    }

    #[test]
    fn rebuild_file_graph_edges_resets_metadata_before_refresh() -> anyhow::Result<()> {
        let mut conn = Connection::open_in_memory()?;
        setup_graph_edge_schema(&conn)?;
        insert_file(&conn, "src/a.rs")?;
        insert_file(&conn, "src/b.rs")?;
        insert_file(&conn, "src/c.rs")?;
        conn.execute(
            "INSERT INTO file_graph_edges(src_path, dst_path, edge_kind, raw_count, weight)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["old/src.rs", "old/dst.rs", "stale", 99_i64, 42.0_f64],
        )?;
        conn.execute(
            "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
            params!["src/b.rs", "Helper"],
        )?;
        conn.execute(
            "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
            params!["src/a.rs", "Helper"],
        )?;

        let tx = conn.transaction()?;
        rebuild_file_graph_edges(&tx)?;
        tx.commit()?;

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

        assert_eq!(rows.len(), 3);
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
}
