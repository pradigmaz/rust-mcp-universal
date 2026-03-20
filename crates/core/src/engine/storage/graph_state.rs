use std::collections::HashMap;

use anyhow::Result;

use crate::graph::{
    GraphEdgeFingerprintBuilder, GraphFingerprintBuilder, empty_graph_content_hash,
    empty_graph_edge_content_hash,
};

#[derive(Debug, Clone)]
pub(super) struct ActualGraphState {
    pub(super) symbol_count: i64,
    pub(super) ref_count: i64,
    pub(super) module_dep_count: i64,
    pub(super) content_hash: String,
}

impl Default for ActualGraphState {
    fn default() -> Self {
        Self {
            symbol_count: 0,
            ref_count: 0,
            module_dep_count: 0,
            content_hash: empty_graph_content_hash(),
        }
    }
}

#[derive(Debug, Default)]
struct ActualGraphStateBuilder {
    symbol_count: i64,
    ref_count: i64,
    module_dep_count: i64,
    fingerprint: GraphFingerprintBuilder,
}

#[derive(Debug, Clone)]
pub(super) struct ActualGraphEdgeState {
    pub(super) outgoing_count: i64,
    pub(super) incoming_count: i64,
    pub(super) content_hash: String,
}

impl Default for ActualGraphEdgeState {
    fn default() -> Self {
        Self {
            outgoing_count: 0,
            incoming_count: 0,
            content_hash: empty_graph_edge_content_hash(),
        }
    }
}

#[derive(Debug, Default)]
struct ActualGraphEdgeStateBuilder {
    outgoing_count: i64,
    incoming_count: i64,
    fingerprint: GraphEdgeFingerprintBuilder,
}

pub(super) fn load_actual_graph_state(
    tx: &rusqlite::Transaction<'_>,
) -> Result<HashMap<String, ActualGraphState>> {
    let mut by_path = HashMap::<String, ActualGraphStateBuilder>::new();

    let mut symbols = tx.prepare(
        "SELECT path, name, kind, language, line, column
         FROM symbols
         ORDER BY path ASC, name ASC, kind ASC, line ASC, column ASC, language ASC",
    )?;
    let symbol_rows = symbols.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, Option<i64>>(4)?,
            row.get::<_, Option<i64>>(5)?,
        ))
    })?;
    for row in symbol_rows {
        let (path, name, kind, language, line, column) = row?;
        let state = by_path.entry(path).or_default();
        state.symbol_count += 1;
        state
            .fingerprint
            .add_symbol(&name, &kind, line, column, &language);
    }

    let mut deps = tx.prepare(
        "SELECT path, dep, language
         FROM module_deps
         ORDER BY path ASC, dep ASC, language ASC",
    )?;
    let dep_rows = deps.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in dep_rows {
        let (path, dep, language) = row?;
        let state = by_path.entry(path).or_default();
        state.module_dep_count += 1;
        state.fingerprint.add_dep(&dep, &language);
    }

    let mut refs = tx.prepare(
        "SELECT path, symbol, language, line, column
         FROM refs
         ORDER BY path ASC, symbol ASC, line ASC, column ASC, language ASC",
    )?;
    let ref_rows = refs.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, Option<i64>>(3)?,
            row.get::<_, Option<i64>>(4)?,
        ))
    })?;
    for row in ref_rows {
        let (path, symbol, language, line, column) = row?;
        let state = by_path.entry(path).or_default();
        state.ref_count += 1;
        state.fingerprint.add_ref(&symbol, line, column, &language);
    }

    let mut out = HashMap::with_capacity(by_path.len());
    for (path, state) in by_path {
        out.insert(
            path,
            ActualGraphState {
                symbol_count: state.symbol_count,
                ref_count: state.ref_count,
                module_dep_count: state.module_dep_count,
                content_hash: state.fingerprint.finish(),
            },
        );
    }
    Ok(out)
}

pub(super) fn load_actual_graph_edge_state(
    tx: &rusqlite::Transaction<'_>,
) -> Result<HashMap<String, ActualGraphEdgeState>> {
    let mut by_path = HashMap::<String, ActualGraphEdgeStateBuilder>::new();
    let mut stmt = tx.prepare(
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
            row.get::<_, f64>(4)? as f32,
        ))
    })?;
    for row in rows {
        let (src_path, dst_path, edge_kind, raw_count, weight) = row?;
        let outgoing = by_path.entry(src_path.clone()).or_default();
        outgoing.outgoing_count += 1;
        outgoing
            .fingerprint
            .add_outgoing(&src_path, &dst_path, &edge_kind, raw_count, weight);

        let incoming = by_path.entry(dst_path.clone()).or_default();
        incoming.incoming_count += 1;
        incoming
            .fingerprint
            .add_incoming(&src_path, &dst_path, &edge_kind, raw_count, weight);
    }

    let mut out = HashMap::with_capacity(by_path.len());
    for (path, state) in by_path {
        out.insert(
            path,
            ActualGraphEdgeState {
                outgoing_count: state.outgoing_count,
                incoming_count: state.incoming_count,
                content_hash: state.fingerprint.finish(),
            },
        );
    }
    Ok(out)
}
