use std::collections::{BTreeMap, HashMap, HashSet};

use anyhow::Result;
use rusqlite::params;

use crate::graph::{
    CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION, GraphEdgeFingerprintBuilder,
    empty_graph_edge_content_hash,
};

#[derive(Debug, Clone)]
struct GraphEdgeState {
    outgoing_count: i64,
    incoming_count: i64,
    content_hash: String,
}

#[derive(Debug, Default)]
struct GraphEdgeStateAccumulator {
    outgoing_count: i64,
    incoming_count: i64,
    fingerprint: GraphEdgeFingerprintBuilder,
}

impl GraphEdgeStateAccumulator {
    fn add_outgoing(
        &mut self,
        src_path: &str,
        dst_path: &str,
        edge_kind: &str,
        raw_count: i64,
        weight: f32,
    ) {
        self.outgoing_count += 1;
        self.fingerprint
            .add_outgoing(src_path, dst_path, edge_kind, raw_count, weight);
    }

    fn add_incoming(
        &mut self,
        src_path: &str,
        dst_path: &str,
        edge_kind: &str,
        raw_count: i64,
        weight: f32,
    ) {
        self.incoming_count += 1;
        self.fingerprint
            .add_incoming(src_path, dst_path, edge_kind, raw_count, weight);
    }

    fn finish(self) -> GraphEdgeState {
        GraphEdgeState {
            outgoing_count: self.outgoing_count,
            incoming_count: self.incoming_count,
            content_hash: self.fingerprint.finish(),
        }
    }
}

pub(super) fn refresh_graph_edge_metadata_for_paths(
    tx: &rusqlite::Transaction<'_>,
    impacted_paths: &HashSet<String>,
    rebuilt_edges: &BTreeMap<(String, String, String), (i64, f32)>,
) -> Result<()> {
    let empty_hash = empty_graph_edge_content_hash();
    tx.execute(
        "UPDATE files
         SET graph_edge_out_count = 0,
             graph_edge_in_count = 0,
             graph_edge_hash = ?1,
             graph_edge_fingerprint_version = ?2
         WHERE path IN (SELECT path FROM temp.graph_edge_impacted_paths)",
        params![&empty_hash, CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION],
    )?;

    let states = build_impacted_graph_edge_states(impacted_paths, rebuilt_edges);
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

pub(super) fn populate_impacted_paths_temp_table(
    tx: &rusqlite::Transaction<'_>,
    impacted_paths: &HashSet<String>,
) -> Result<()> {
    tx.execute_batch(
        r#"
        DROP TABLE IF EXISTS temp.graph_edge_impacted_paths;
        CREATE TEMP TABLE graph_edge_impacted_paths (
            path TEXT PRIMARY KEY
        );
        "#,
    )?;
    for path in impacted_paths {
        tx.execute(
            "INSERT INTO graph_edge_impacted_paths(path) VALUES (?1)",
            [path],
        )?;
    }
    Ok(())
}

fn build_impacted_graph_edge_states(
    impacted_paths: &HashSet<String>,
    rebuilt_edges: &BTreeMap<(String, String, String), (i64, f32)>,
) -> HashMap<String, GraphEdgeState> {
    let mut by_path = HashMap::<String, GraphEdgeStateAccumulator>::new();
    for ((src_path, dst_path, edge_kind), (raw_count, weight)) in rebuilt_edges {
        if impacted_paths.contains(src_path) {
            by_path
                .entry(src_path.clone())
                .or_default()
                .add_outgoing(src_path, dst_path, edge_kind, *raw_count, *weight);
        }
        if impacted_paths.contains(dst_path) {
            by_path
                .entry(dst_path.clone())
                .or_default()
                .add_incoming(src_path, dst_path, edge_kind, *raw_count, *weight);
        }
    }

    by_path
        .into_iter()
        .map(|(path, accumulator)| (path, accumulator.finish()))
        .collect()
}
