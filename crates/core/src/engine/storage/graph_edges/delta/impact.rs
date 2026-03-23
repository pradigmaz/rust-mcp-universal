use std::collections::{BTreeMap, HashMap, HashSet};

use crate::text_utils::symbol_tail;

use super::super::{
    REF_EXACT_EDGE_WEIGHT, REF_TAIL_UNIQUE_EDGE_WEIGHT, SHARED_DEP_EDGE_WEIGHT,
    add_materialized_edge,
};
use super::seed::GraphRefreshSeed;

pub(super) fn determine_impacted_paths(
    dirty_paths: &HashSet<String>,
    pre_refresh: &HashMap<String, GraphRefreshSeed>,
    current_refresh: &HashMap<String, GraphRefreshSeed>,
    symbol_destinations: &HashMap<String, Vec<String>>,
    ref_rows: &[(String, String)],
    dep_groups: &HashMap<String, Vec<String>>,
) -> HashSet<String> {
    let mut impacted_paths = dirty_paths.clone();
    let mut candidate_symbol_names = HashSet::<String>::new();
    let mut candidate_dep_names = HashSet::<String>::new();

    for path in dirty_paths {
        if let Some(seed) = pre_refresh.get(path) {
            impacted_paths.extend(seed.neighbor_paths.iter().cloned());
            candidate_symbol_names.extend(seed.symbol_names.iter().cloned());
            candidate_dep_names.extend(seed.dep_names.iter().cloned());
        }
        if let Some(seed) = current_refresh.get(path) {
            candidate_symbol_names.extend(seed.symbol_names.iter().cloned());
            candidate_dep_names.extend(seed.dep_names.iter().cloned());
        }
    }

    for symbol_name in &candidate_symbol_names {
        if let Some(dest_paths) = symbol_destinations.get(symbol_name) {
            impacted_paths.extend(dest_paths.iter().cloned());
        }
    }

    for (src_path, symbol) in ref_rows {
        if dirty_paths.contains(src_path) {
            for_each_resolved_destination(symbol, symbol_destinations, |dst_path| {
                if dst_path != src_path {
                    impacted_paths.insert(dst_path.to_string());
                }
            });
        }
        if ref_symbol_matches_candidates(symbol, &candidate_symbol_names) {
            impacted_paths.insert(src_path.clone());
        }
    }

    for dep_name in candidate_dep_names {
        if let Some(paths) = dep_groups.get(&dep_name) {
            impacted_paths.extend(paths.iter().cloned());
        }
    }

    impacted_paths
}

pub(super) fn materialize_impacted_edges(
    impacted_paths: &HashSet<String>,
    symbol_destinations: &HashMap<String, Vec<String>>,
    ref_rows: &[(String, String)],
    dep_groups: &HashMap<String, Vec<String>>,
) -> BTreeMap<(String, String, String), (i64, f32)> {
    let mut edges = BTreeMap::<(String, String, String), (i64, f32)>::new();

    for (src_path, symbol) in ref_rows {
        let src_impacted = impacted_paths.contains(src_path);
        for_each_resolved_destination(symbol, symbol_destinations, |dst_path| {
            if dst_path == src_path {
                return;
            }
            if !src_impacted && !impacted_paths.contains(dst_path) {
                return;
            }
            let edge_kind = if symbol_destinations.contains_key(symbol) {
                "ref_exact"
            } else {
                "ref_tail_unique"
            };
            let edge_weight = if edge_kind == "ref_exact" {
                REF_EXACT_EDGE_WEIGHT
            } else {
                REF_TAIL_UNIQUE_EDGE_WEIGHT
            };
            add_materialized_edge(&mut edges, src_path, dst_path, edge_kind, edge_weight);
        });
    }

    for paths in dep_groups.values() {
        if !paths.iter().any(|path| impacted_paths.contains(path)) {
            continue;
        }
        for src_idx in 0..paths.len() {
            for dst_idx in 0..paths.len() {
                if src_idx == dst_idx {
                    continue;
                }
                let src_path = &paths[src_idx];
                let dst_path = &paths[dst_idx];
                if !impacted_paths.contains(src_path) && !impacted_paths.contains(dst_path) {
                    continue;
                }
                add_materialized_edge(
                    &mut edges,
                    src_path,
                    dst_path,
                    "shared_dep",
                    SHARED_DEP_EDGE_WEIGHT,
                );
            }
        }
    }

    edges
}

fn ref_symbol_matches_candidates(
    ref_symbol: &str,
    candidate_symbol_names: &HashSet<String>,
) -> bool {
    candidate_symbol_names.contains(ref_symbol) || {
        let tail = symbol_tail(ref_symbol);
        tail != ref_symbol && candidate_symbol_names.contains(tail)
    }
}

fn for_each_resolved_destination(
    symbol: &str,
    symbol_destinations: &HashMap<String, Vec<String>>,
    mut visit: impl FnMut(&str),
) {
    if let Some(dest_paths) = symbol_destinations.get(symbol) {
        for dst_path in dest_paths {
            visit(dst_path);
        }
        return;
    }

    let tail = symbol_tail(symbol);
    if tail == symbol {
        return;
    }
    let Some(dest_paths) = symbol_destinations.get(tail) else {
        return;
    };
    if dest_paths.len() != 1 {
        return;
    }
    visit(&dest_paths[0]);
}
