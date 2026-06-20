use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use rusqlite::Connection;

use super::super::Engine;
use super::call_path_graph::{
    CallPathState, edge_cost, load_outgoing_graph_edges, pop_best_call_path_state,
    resolve_edge_evidence,
};
use super::common::file_exists;
use super::validation::require_non_empty;
use crate::model::{CallPathEndpoint, CallPathExplain, CallPathResult, CallPathStep, SymbolMatch};
use crate::text_utils::i64_to_option_usize;
use crate::utils::normalize_path;

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

    pub(crate) fn normalize_lookup_path(&self, path: &str) -> Result<String> {
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
