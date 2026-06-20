use std::collections::{HashMap, HashSet};

use anyhow::Result;
use rusqlite::Connection;

use super::call_path_graph::{
    CallPathState, edge_cost, load_outgoing_graph_edges, pop_best_call_path_state,
    resolve_edge_evidence,
};
use crate::model::{CallPathEndpoint, CallPathExplain, CallPathResult, CallPathStep};

pub(super) fn find_call_path(
    conn: &Connection,
    from_endpoint: CallPathEndpoint,
    to_endpoint: CallPathEndpoint,
    max_hops: usize,
) -> Result<CallPathResult> {
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

        push_next_call_path_states(
            conn,
            current,
            max_hops,
            &mut frontier,
            &mut best_costs,
            &mut considered_edges,
        )?;
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

fn push_next_call_path_states(
    conn: &Connection,
    current: CallPathState,
    max_hops: usize,
    frontier: &mut Vec<CallPathState>,
    best_costs: &mut HashMap<(String, usize), f32>,
    considered_edges: &mut usize,
) -> Result<()> {
    for edge in load_outgoing_graph_edges(conn, &current.node)? {
        *considered_edges = considered_edges.saturating_add(1);
        let next_hops = current.hops.saturating_add(1);
        if next_hops > max_hops {
            continue;
        }

        let next_cost = current.cost + edge_cost(&edge);
        let best_key = (edge.dst_path.clone(), next_hops);
        if best_costs
            .get(&best_key)
            .is_some_and(|best_cost| *best_cost <= next_cost)
        {
            continue;
        }
        best_costs.insert(best_key, next_cost);

        let (evidence, line, column) = resolve_edge_evidence(conn, &edge)?;
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

    Ok(())
}
