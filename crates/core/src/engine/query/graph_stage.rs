use std::collections::{BTreeSet, HashMap};

use anyhow::Result;
use rusqlite::{Connection, params};

use crate::model::SearchHit;

use super::vector_utils::trim_excerpt;

const GRAPH_SEED_LIMIT: usize = 12;
const GRAPH_POOL_LIMIT: usize = 64;
const SECOND_HOP_FRONTIER_LIMIT: usize = 24;
const SECOND_HOP_DECAY: f32 = 0.55;
const HUB_PENALTY_MIN: f32 = 0.25;
const HUB_PENALTY_MAX: f32 = 1.0;

#[derive(Debug, Clone)]
pub(super) struct GraphPoolCandidate {
    pub(super) path: String,
    pub(super) preview: String,
    pub(super) size_bytes: i64,
    pub(super) language: String,
    pub(super) graph_score: f32,
    pub(super) seed_path: String,
    pub(super) edge_kinds: Vec<String>,
    pub(super) hops: usize,
}

#[derive(Debug, Clone)]
struct IncidentEdge {
    src_path: String,
    dst_path: String,
    edge_kind: String,
    weight: f32,
}

#[derive(Debug, Clone)]
struct GraphNodeMeta {
    preview: String,
    size_bytes: i64,
    language: String,
    out_count: i64,
    in_count: i64,
}

#[derive(Debug, Clone)]
struct FrontierStep {
    current_path: String,
    previous_path: String,
    seed_path: String,
    propagated_score: f32,
    edge_kinds: BTreeSet<String>,
}

#[derive(Debug, Clone, Default)]
struct GraphAccumulator {
    raw_score: f32,
    best_contribution: f32,
    seed_path: String,
    edge_kinds: BTreeSet<String>,
    hops: usize,
}

pub(super) fn graph_candidate_pool(
    conn: &Connection,
    seed_hits: &[SearchHit],
) -> Result<Vec<GraphPoolCandidate>> {
    if seed_hits.is_empty() {
        return Ok(Vec::new());
    }

    let top_score = seed_hits
        .first()
        .map(|hit| hit.score.max(0.001))
        .unwrap_or(1.0);
    let mut accumulators = HashMap::<String, GraphAccumulator>::new();
    let mut first_hop_frontier = HashMap::<(String, String), FrontierStep>::new();

    for seed in seed_hits.iter().take(GRAPH_SEED_LIMIT) {
        let seed_strength = (seed.score.max(0.0) / top_score).clamp(0.05, 1.0);
        for edge in incident_edges_for_path(conn, &seed.path)? {
            let (other_path, labeled_kind) = counterpart_for(&seed.path, &edge);
            if other_path == seed.path {
                continue;
            }
            let contribution = seed_strength * edge.weight.max(0.0);
            if contribution <= 0.0 {
                continue;
            }

            let mut edge_kinds = BTreeSet::new();
            edge_kinds.insert(labeled_kind.clone());
            accumulate_candidate(
                &mut accumulators,
                &other_path,
                contribution,
                &seed.path,
                &edge_kinds,
                1,
            );

            let key = (other_path.clone(), seed.path.clone());
            match first_hop_frontier.get_mut(&key) {
                Some(existing) => {
                    if contribution > existing.propagated_score {
                        existing.propagated_score = contribution;
                        existing.edge_kinds = edge_kinds.clone();
                    } else {
                        existing.edge_kinds.extend(edge_kinds.iter().cloned());
                    }
                }
                None => {
                    first_hop_frontier.insert(
                        key,
                        FrontierStep {
                            current_path: other_path,
                            previous_path: seed.path.clone(),
                            seed_path: seed.path.clone(),
                            propagated_score: contribution,
                            edge_kinds,
                        },
                    );
                }
            }
        }
    }

    let mut second_hop_frontier = first_hop_frontier.into_values().collect::<Vec<_>>();
    second_hop_frontier.sort_by(|left, right| {
        right
            .propagated_score
            .total_cmp(&left.propagated_score)
            .then_with(|| left.current_path.cmp(&right.current_path))
    });
    second_hop_frontier.truncate(SECOND_HOP_FRONTIER_LIMIT);

    for frontier in second_hop_frontier {
        for edge in incident_edges_for_path(conn, &frontier.current_path)? {
            let (other_path, labeled_kind) = counterpart_for(&frontier.current_path, &edge);
            if other_path == frontier.current_path || other_path == frontier.previous_path {
                continue;
            }
            let contribution = frontier.propagated_score * edge.weight.max(0.0) * SECOND_HOP_DECAY;
            if contribution <= 0.0 {
                continue;
            }
            let mut edge_kinds = frontier.edge_kinds.clone();
            edge_kinds.insert(labeled_kind);
            accumulate_candidate(
                &mut accumulators,
                &other_path,
                contribution,
                &frontier.seed_path,
                &edge_kinds,
                2,
            );
        }
    }

    if accumulators.is_empty() {
        return Ok(Vec::new());
    }

    let node_meta = load_node_meta(conn, &accumulators.keys().cloned().collect::<Vec<_>>())?;
    let mut candidates = Vec::new();
    for (path, accumulator) in accumulators {
        let Some(meta) = node_meta.get(&path) else {
            continue;
        };
        let degree = meta.out_count.saturating_add(meta.in_count) as f32;
        let hub_penalty = (1.0_f32 / (1.0 + degree).sqrt()).clamp(HUB_PENALTY_MIN, HUB_PENALTY_MAX);
        let graph_score = accumulator.raw_score * hub_penalty;
        if graph_score <= 0.0 {
            continue;
        }
        candidates.push(GraphPoolCandidate {
            path,
            preview: meta.preview.clone(),
            size_bytes: meta.size_bytes,
            language: meta.language.clone(),
            graph_score,
            seed_path: accumulator.seed_path,
            edge_kinds: accumulator.edge_kinds.into_iter().collect(),
            hops: accumulator.hops.max(1),
        });
    }

    candidates.sort_by(|left, right| {
        right
            .graph_score
            .total_cmp(&left.graph_score)
            .then_with(|| left.path.cmp(&right.path))
    });
    candidates.truncate(GRAPH_POOL_LIMIT);
    Ok(candidates)
}

fn accumulate_candidate(
    accumulators: &mut HashMap<String, GraphAccumulator>,
    path: &str,
    contribution: f32,
    seed_path: &str,
    edge_kinds: &BTreeSet<String>,
    hops: usize,
) {
    let entry = accumulators.entry(path.to_string()).or_default();
    entry.raw_score += contribution;
    entry.edge_kinds.extend(edge_kinds.iter().cloned());
    if entry.hops == 0 || hops < entry.hops {
        entry.hops = hops;
    }
    if contribution > entry.best_contribution || entry.seed_path.is_empty() {
        entry.best_contribution = contribution;
        entry.seed_path = seed_path.to_string();
    }
}

fn incident_edges_for_path(conn: &Connection, path: &str) -> Result<Vec<IncidentEdge>> {
    let mut stmt = conn.prepare(
        "SELECT src_path, dst_path, edge_kind, weight
         FROM file_graph_edges
         WHERE src_path = ?1 OR dst_path = ?1
         ORDER BY src_path ASC, dst_path ASC, edge_kind ASC",
    )?;
    let rows = stmt.query_map([path], |row| {
        Ok(IncidentEdge {
            src_path: row.get(0)?,
            dst_path: row.get(1)?,
            edge_kind: row.get(2)?,
            weight: row.get::<_, f64>(3)? as f32,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn counterpart_for(current_path: &str, edge: &IncidentEdge) -> (String, String) {
    if edge.src_path == current_path {
        (
            edge.dst_path.clone(),
            format!("outgoing:{}", edge.edge_kind),
        )
    } else {
        (
            edge.src_path.clone(),
            format!("incoming:{}", edge.edge_kind),
        )
    }
}

fn load_node_meta(conn: &Connection, paths: &[String]) -> Result<HashMap<String, GraphNodeMeta>> {
    let mut stmt = conn.prepare(
        "SELECT path, sample, size_bytes, language,
                COALESCE(graph_edge_out_count, 0),
                COALESCE(graph_edge_in_count, 0)
         FROM files
         WHERE path = ?1",
    )?;
    let mut out = HashMap::with_capacity(paths.len());
    for path in paths {
        let row = stmt.query_row(params![path], |row| {
            Ok(GraphNodeMeta {
                preview: trim_excerpt(&row.get::<_, String>(1)?, 260),
                size_bytes: row.get(2)?,
                language: row.get(3)?,
                out_count: row.get(4)?,
                in_count: row.get(5)?,
            })
        });
        if let Ok(meta) = row {
            out.insert(path.clone(), meta);
        }
    }
    Ok(out)
}
