use std::collections::{HashMap, HashSet};

use anyhow::Result;
use rusqlite::params;

use super::{load_dep_groups, load_ref_rows, load_symbol_destinations};

#[path = "delta/impact.rs"]
mod impact;
#[path = "delta/metadata.rs"]
mod metadata;
#[path = "delta/seed.rs"]
mod seed;

use impact::{determine_impacted_paths, materialize_impacted_edges};
use metadata::{populate_impacted_paths_temp_table, refresh_graph_edge_metadata_for_paths};
pub(in crate::engine) use seed::{GraphRefreshSeed, capture_graph_refresh_seed};

pub(in crate::engine) fn refresh_file_graph_edges(
    tx: &rusqlite::Transaction<'_>,
    dirty_paths: &HashSet<String>,
    pre_refresh: &HashMap<String, GraphRefreshSeed>,
) -> Result<()> {
    if dirty_paths.is_empty() {
        return Ok(());
    }

    let current_refresh = load_graph_refresh_seeds(tx, dirty_paths)?;
    let symbol_destinations = load_symbol_destinations(tx)?;
    let ref_rows = load_ref_rows(tx)?;
    let dep_groups = load_dep_groups(tx)?;
    let impacted_paths = determine_impacted_paths(
        dirty_paths,
        pre_refresh,
        &current_refresh,
        &symbol_destinations,
        &ref_rows,
        &dep_groups,
    );
    if impacted_paths.is_empty() {
        return Ok(());
    }

    populate_impacted_paths_temp_table(tx, &impacted_paths)?;
    tx.execute(
        "DELETE FROM file_graph_edges
         WHERE src_path IN (SELECT path FROM temp.graph_edge_impacted_paths)
            OR dst_path IN (SELECT path FROM temp.graph_edge_impacted_paths)",
        [],
    )?;

    let rebuilt_edges = materialize_impacted_edges(
        &impacted_paths,
        &symbol_destinations,
        &ref_rows,
        &dep_groups,
    );
    tx.execute_batch(
        r#"
        DROP TABLE IF EXISTS temp.file_graph_edges_delta_rebuild;
        CREATE TEMP TABLE file_graph_edges_delta_rebuild (
            src_path TEXT NOT NULL,
            dst_path TEXT NOT NULL,
            edge_kind TEXT NOT NULL,
            raw_count INTEGER NOT NULL,
            weight REAL NOT NULL,
            PRIMARY KEY(src_path, dst_path, edge_kind)
        );
        "#,
    )?;
    for ((src_path, dst_path, edge_kind), (raw_count, weight)) in &rebuilt_edges {
        tx.execute(
            "INSERT INTO file_graph_edges_delta_rebuild(src_path, dst_path, edge_kind, raw_count, weight)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![src_path, dst_path, edge_kind, raw_count, weight],
        )?;
    }
    tx.execute(
        "INSERT INTO file_graph_edges(src_path, dst_path, edge_kind, raw_count, weight)
         SELECT src_path, dst_path, edge_kind, raw_count, weight
         FROM file_graph_edges_delta_rebuild",
        [],
    )?;
    tx.execute("DROP TABLE temp.file_graph_edges_delta_rebuild", [])?;
    refresh_graph_edge_metadata_for_paths(tx, &impacted_paths, &rebuilt_edges)?;
    tx.execute("DROP TABLE temp.graph_edge_impacted_paths", [])?;
    Ok(())
}

fn load_graph_refresh_seeds(
    tx: &rusqlite::Transaction<'_>,
    paths: &HashSet<String>,
) -> Result<HashMap<String, GraphRefreshSeed>> {
    let mut out = HashMap::with_capacity(paths.len());
    for path in paths {
        out.insert(path.clone(), capture_graph_refresh_seed(tx, path)?);
    }
    Ok(out)
}
