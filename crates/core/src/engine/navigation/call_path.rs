use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use rusqlite::{Connection, params};

use super::super::Engine;
use super::common::{file_exists, load_string_set, require_non_empty};
use crate::model::{CallPathEndpoint, CallPathExplain, CallPathResult, CallPathStep, SymbolMatch};
use crate::text_utils::{i64_to_option_usize, symbol_tail};
use crate::utils::normalize_path;

#[derive(Debug, Clone)]
struct GraphEdgeRow {
    src_path: String,
    dst_path: String,
    edge_kind: String,
    raw_count: usize,
    weight: f32,
}

#[derive(Debug, Clone)]
struct CallPathState {
    node: String,
    cost: f32,
    total_weight: f32,
    hops: usize,
    path: Vec<String>,
    steps: Vec<CallPathStep>,
}

impl Engine {
    pub fn call_path(&self, from: &str, to: &str, max_hops: usize) -> Result<CallPathResult> {
        if max_hops == 0 {
            bail!("`max_hops` must be >= 1");
        }

        let conn = self.open_db()?;
        let from_endpoint = self.resolve_call_path_endpoint(&conn, from)?;
        let to_endpoint = self.resolve_call_path_endpoint(&conn, to)?;

        if from_endpoint.resolved_path == to_endpoint.resolved_path {
            let resolved_path = from_endpoint.resolved_path.clone();
            return Ok(CallPathResult {
                from: from_endpoint,
                to: to_endpoint,
                found: true,
                path: vec![resolved_path],
                steps: Vec::new(),
                hops: 0,
                total_weight: 0.0,
                explain: CallPathExplain {
                    algorithm: "bounded_weighted_dijkstra".to_string(),
                    max_hops,
                    visited_nodes: 1,
                    considered_edges: 0,
                },
            });
        }

        let start_path = from_endpoint.resolved_path.clone();
        let target_path = to_endpoint.resolved_path.clone();
        let mut frontier = vec![CallPathState {
            node: start_path.clone(),
            cost: 0.0,
            total_weight: 0.0,
            hops: 0,
            path: vec![start_path.clone()],
            steps: Vec::new(),
        }];
        let mut best_costs = HashMap::<(String, usize), f32>::new();
        best_costs.insert((start_path.clone(), 0), 0.0);
        let mut visited_nodes = HashSet::<String>::new();
        let mut considered_edges = 0_usize;

        while let Some(current) = pop_best_call_path_state(&mut frontier) {
            visited_nodes.insert(current.node.clone());
            if current.node == target_path {
                return Ok(CallPathResult {
                    from: from_endpoint,
                    to: to_endpoint,
                    found: true,
                    path: current.path,
                    steps: current.steps,
                    hops: current.hops,
                    total_weight: current.total_weight,
                    explain: CallPathExplain {
                        algorithm: "bounded_weighted_dijkstra".to_string(),
                        max_hops,
                        visited_nodes: visited_nodes.len(),
                        considered_edges,
                    },
                });
            }

            if current.hops >= max_hops {
                continue;
            }

            for edge in load_outgoing_graph_edges(&conn, &current.node)? {
                considered_edges = considered_edges.saturating_add(1);
                let next_hops = current.hops.saturating_add(1);
                let next_cost = current.cost + edge_cost(&edge);
                let best_key = (edge.dst_path.clone(), next_hops);
                if best_costs
                    .get(&best_key)
                    .is_some_and(|best_cost| *best_cost <= next_cost)
                {
                    continue;
                }
                best_costs.insert(best_key, next_cost);

                let (evidence, line, column) = resolve_edge_evidence(&conn, &edge)?;
                let mut next_path = current.path.clone();
                next_path.push(edge.dst_path.clone());
                let mut next_steps = current.steps.clone();
                next_steps.push(CallPathStep {
                    from_path: edge.src_path.clone(),
                    to_path: edge.dst_path.clone(),
                    edge_kind: edge.edge_kind.clone(),
                    raw_count: edge.raw_count,
                    weight: edge.weight,
                    evidence,
                    line,
                    column,
                });

                frontier.push(CallPathState {
                    node: edge.dst_path.clone(),
                    cost: next_cost,
                    total_weight: current.total_weight + edge.weight,
                    hops: next_hops,
                    path: next_path,
                    steps: next_steps,
                });
            }
        }

        Ok(CallPathResult {
            from: from_endpoint,
            to: to_endpoint,
            found: false,
            path: Vec::new(),
            steps: Vec::new(),
            hops: 0,
            total_weight: 0.0,
            explain: CallPathExplain {
                algorithm: "bounded_weighted_dijkstra".to_string(),
                max_hops,
                visited_nodes: visited_nodes.len().max(1),
                considered_edges,
            },
        })
    }

    pub(super) fn normalize_lookup_path(&self, path: &str) -> Result<String> {
        let raw = require_non_empty(path, "path")?;
        let input_path = Path::new(raw);
        if !input_path.is_absolute() {
            return Ok(normalize_path(input_path));
        }

        if let Ok(relative) = input_path.strip_prefix(&self.project_root) {
            return Ok(normalize_path(relative));
        }

        #[cfg(windows)]
        {
            if let (Ok(canonical_input), Ok(canonical_root)) =
                (input_path.canonicalize(), self.project_root.canonicalize())
            {
                if let Ok(relative) = canonical_input.strip_prefix(&canonical_root) {
                    return Ok(normalize_path(relative));
                }
            }
        }

        Err(anyhow!("path `{raw}` is outside project root"))
    }

    fn resolve_call_path_endpoint(&self, conn: &Connection, raw: &str) -> Result<CallPathEndpoint> {
        let input = require_non_empty(raw, "endpoint")?.to_string();
        if let Ok(path) = self.normalize_lookup_path(&input) {
            if file_exists(conn, &path)? {
                return Ok(CallPathEndpoint {
                    input,
                    resolved_path: path,
                    kind: "path".to_string(),
                    symbol: None,
                    line: None,
                    column: None,
                });
            }
        }

        let mut stmt = conn.prepare(
            "SELECT path, name, line, column
             FROM symbols
             WHERE name = ?1
             ORDER BY path ASC,
                      COALESCE(line, 2147483647) ASC,
                      COALESCE(column, 2147483647) ASC",
        )?;
        let rows = stmt
            .query_map([&input], |row| {
                Ok(SymbolMatch {
                    path: row.get(0)?,
                    name: row.get(1)?,
                    kind: "symbol".to_string(),
                    language: String::new(),
                    line: row.get::<_, Option<i64>>(2)?.and_then(i64_to_option_usize),
                    column: row.get::<_, Option<i64>>(3)?.and_then(i64_to_option_usize),
                    exact: true,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut unique_paths = rows.iter().map(|row| row.path.clone()).collect::<Vec<_>>();
        unique_paths.sort();
        unique_paths.dedup();

        match rows.first() {
            Some(first) if unique_paths.len() == 1 => Ok(CallPathEndpoint {
                input,
                resolved_path: first.path.clone(),
                kind: "symbol".to_string(),
                symbol: Some(first.name.clone()),
                line: first.line,
                column: first.column,
            }),
            Some(_) => bail!(
                "symbol endpoint `{}` is ambiguous across {} files; use a path instead",
                raw.trim(),
                unique_paths.len()
            ),
            None => bail!(
                "unable to resolve endpoint `{}` as indexed path or exact symbol",
                raw.trim()
            ),
        }
    }
}

fn pop_best_call_path_state(frontier: &mut Vec<CallPathState>) -> Option<CallPathState> {
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

fn load_outgoing_graph_edges(conn: &Connection, src_path: &str) -> Result<Vec<GraphEdgeRow>> {
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

fn edge_cost(edge: &GraphEdgeRow) -> f32 {
    let kind_penalty = match edge.edge_kind.as_str() {
        "ref_exact" => 0.0,
        "ref_tail_unique" => 0.35,
        "shared_dep" => 1.25,
        _ => 0.75,
    };
    (1.0 / edge.weight.max(0.05)) + kind_penalty
}

fn resolve_edge_evidence(
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
