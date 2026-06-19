use anyhow::Result;
use rusqlite::{Connection, params};

use super::common::load_string_set;
use crate::model::CallPathStep;
use crate::text_utils::{i64_to_option_usize, symbol_tail};

#[derive(Debug, Clone)]
pub(super) struct GraphEdgeRow {
    pub(super) src_path: String,
    pub(super) dst_path: String,
    pub(super) edge_kind: String,
    pub(super) raw_count: usize,
    pub(super) weight: f32,
}

#[derive(Debug, Clone)]
pub(super) struct CallPathState {
    pub(super) node: String,
    pub(super) cost: f32,
    pub(super) total_weight: f32,
    pub(super) hops: usize,
    pub(super) path: Vec<String>,
    pub(super) steps: Vec<CallPathStep>,
}

pub(super) fn pop_best_call_path_state(frontier: &mut Vec<CallPathState>) -> Option<CallPathState> {
    if frontier.is_empty() {
        return None;
    }

    let mut best_idx = 0_usize;
    for idx in 1..frontier.len() {
        let left = &frontier[idx];
        let right = &frontier[best_idx];
        let ordering = left
            .cost
            .total_cmp(&right.cost)
            .then_with(|| left.hops.cmp(&right.hops))
            .then_with(|| right.total_weight.total_cmp(&left.total_weight))
            .then_with(|| left.node.cmp(&right.node));
        if ordering.is_lt() {
            best_idx = idx;
        }
    }

    Some(frontier.swap_remove(best_idx))
}

pub(super) fn load_outgoing_graph_edges(
    conn: &Connection,
    src_path: &str,
) -> Result<Vec<GraphEdgeRow>> {
    let mut stmt = conn.prepare(
        "SELECT src_path, dst_path, edge_kind, raw_count, weight
         FROM file_graph_edges
         WHERE src_path = ?1
         ORDER BY weight DESC, raw_count DESC, edge_kind ASC, dst_path ASC",
    )?;
    let rows = stmt.query_map([src_path], |row| {
        Ok(GraphEdgeRow {
            src_path: row.get(0)?,
            dst_path: row.get(1)?,
            edge_kind: row.get(2)?,
            raw_count: row.get::<_, i64>(3)?.try_into().unwrap_or(usize::MAX),
            weight: row.get::<_, f64>(4)? as f32,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

pub(super) fn edge_cost(edge: &GraphEdgeRow) -> f32 {
    let kind_penalty = match edge.edge_kind.as_str() {
        "ref_exact" => 0.0,
        "ref_tail_unique" => 0.35,
        "shared_dep" => 1.25,
        _ => 0.75,
    };
    (1.0 / edge.weight.max(0.05)) + kind_penalty
}

pub(super) fn resolve_edge_evidence(
    conn: &Connection,
    edge: &GraphEdgeRow,
) -> Result<(String, Option<usize>, Option<usize>)> {
    match edge.edge_kind.as_str() {
        "ref_exact" => resolve_ref_exact_evidence(conn, edge),
        "ref_tail_unique" => resolve_ref_tail_unique_evidence(conn, edge),
        "shared_dep" => resolve_shared_dep_evidence(conn, edge),
        _ => Ok((edge.edge_kind.clone(), None, None)),
    }
}

fn resolve_ref_exact_evidence(
    conn: &Connection,
    edge: &GraphEdgeRow,
) -> Result<(String, Option<usize>, Option<usize>)> {
    let mut stmt = conn.prepare(
        "SELECT r.symbol, r.line, r.column
         FROM refs r
         INNER JOIN symbols s
           ON s.path = ?2
          AND s.name = r.symbol
         WHERE r.path = ?1
         ORDER BY COALESCE(r.line, 2147483647) ASC,
                  COALESCE(r.column, 2147483647) ASC,
                  r.symbol ASC
         LIMIT 1",
    )?;
    let result = stmt.query_row(params![&edge.src_path, &edge.dst_path], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<i64>>(1)?.and_then(i64_to_option_usize),
            row.get::<_, Option<i64>>(2)?.and_then(i64_to_option_usize),
        ))
    });

    match result {
        Ok(value) => Ok(value),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok((edge.edge_kind.clone(), None, None)),
        Err(err) => Err(err.into()),
    }
}

fn resolve_ref_tail_unique_evidence(
    conn: &Connection,
    edge: &GraphEdgeRow,
) -> Result<(String, Option<usize>, Option<usize>)> {
    let dst_symbols = load_string_set(
        conn,
        "SELECT name FROM symbols WHERE path = ?1",
        &edge.dst_path,
        "call path destination symbols",
    )?;
    if dst_symbols.is_empty() {
        return Ok((edge.edge_kind.clone(), None, None));
    }

    let mut stmt = conn.prepare(
        "SELECT symbol, line, column
         FROM refs
         WHERE path = ?1
         ORDER BY COALESCE(line, 2147483647) ASC,
                  COALESCE(column, 2147483647) ASC,
                  symbol ASC",
    )?;
    let rows = stmt.query_map([&edge.src_path], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<i64>>(1)?.and_then(i64_to_option_usize),
            row.get::<_, Option<i64>>(2)?.and_then(i64_to_option_usize),
        ))
    })?;
    for row in rows {
        let (symbol, line, column) = row?;
        if dst_symbols.contains(symbol_tail(&symbol)) {
            return Ok((symbol, line, column));
        }
    }

    Ok((edge.edge_kind.clone(), None, None))
}

fn resolve_shared_dep_evidence(
    conn: &Connection,
    edge: &GraphEdgeRow,
) -> Result<(String, Option<usize>, Option<usize>)> {
    let mut stmt = conn.prepare(
        "SELECT a.dep
         FROM module_deps a
         INNER JOIN module_deps b
           ON b.dep = a.dep
          AND b.path = ?2
         WHERE a.path = ?1
         ORDER BY a.dep ASC
         LIMIT 1",
    )?;
    let result = stmt.query_row(params![&edge.src_path, &edge.dst_path], |row| row.get(0));
    match result {
        Ok(dep) => Ok((dep, None, None)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok((edge.edge_kind.clone(), None, None)),
        Err(err) => Err(err.into()),
    }
}
