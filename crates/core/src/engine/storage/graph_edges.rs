use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
#[cfg(test)]
use rusqlite::params;

#[cfg(test)]
use super::graph_state::load_actual_graph_edge_state;
#[cfg(test)]
use crate::graph::{CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION, empty_graph_edge_content_hash};
#[cfg(test)]
use crate::text_utils::symbol_tail;

#[path = "graph_edges/delta.rs"]
mod delta;

const REF_EXACT_EDGE_WEIGHT: f32 = 1.0;
const REF_TAIL_UNIQUE_EDGE_WEIGHT: f32 = 0.72;
const SHARED_DEP_EDGE_WEIGHT: f32 = 0.35;

pub(in crate::engine) use delta::{
    GraphRefreshSeed, capture_graph_refresh_seed, refresh_file_graph_edges,
};

#[cfg(test)]
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

#[cfg(test)]
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
#[path = "graph_edges/tests.rs"]
mod tests;
