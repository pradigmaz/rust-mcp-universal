use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use anyhow::{Result, bail};

use crate::index_scope::IndexScope;
use crate::model::IndexingOptions;
use crate::quality::{
    CrossLayerFacts, QualityPolicy, StructuralFacts, StructuralPolicy,
    StructuralUnmatchedBehavior,
};

const ORPHAN_ENTRYPOINTS: &[&str] = &["main", "lib", "mod", "index", "__init__"];

pub(super) fn load_structural_facts(
    conn: &rusqlite::Connection,
    active_paths: &HashSet<String>,
    policy: &QualityPolicy,
) -> Result<HashMap<String, StructuralFacts>> {
    let (outgoing, incoming) = load_direct_neighbors(conn, active_paths)?;
    let mut facts = active_paths
        .iter()
        .cloned()
        .map(|path| {
            let fan_in = incoming
                .get(&path)
                .map(|neighbors| i64::try_from(neighbors.len()).unwrap_or(i64::MAX))
                .unwrap_or(0);
            let fan_out = outgoing
                .get(&path)
                .map(|neighbors| i64::try_from(neighbors.len()).unwrap_or(i64::MAX))
                .unwrap_or(0);
            (
                path,
                StructuralFacts {
                    fan_in_count: Some(fan_in),
                    fan_out_count: Some(fan_out),
                    ..StructuralFacts::default()
                },
            )
        })
        .collect::<HashMap<_, _>>();

    for path in cycle_members(&outgoing) {
        if let Some(entry) = facts.get_mut(&path) {
            entry.cycle_member = true;
        }
    }

    let Some(structural_policy) = policy.structural.as_ref().filter(|policy| policy.has_zones())
    else {
        return Ok(facts);
    };

    let zone_matches = build_zone_matches(active_paths, structural_policy)?;
    let mut cross_layer = HashMap::<String, (i64, String)>::new();

    for (src_path, neighbors) in &outgoing {
        let src_zone = zone_matches.get(src_path).and_then(|zone| zone.as_deref());
        for dst_path in neighbors {
            let dst_zone = zone_matches.get(dst_path).and_then(|zone| zone.as_deref());
            let Some((src_zone, dst_zone)) = matched_zone_pair(
                src_zone,
                dst_zone,
                structural_policy.unmatched_behavior,
            ) else {
                continue;
            };
            if src_zone == dst_zone {
                continue;
            }
            if let Some(message) = cross_layer_message(structural_policy, src_zone, dst_zone) {
                let entry = cross_layer
                    .entry(src_path.clone())
                    .or_insert((0, message.clone()));
                entry.0 += 1;
                if entry.1.is_empty() {
                    entry.1 = message;
                }
            }
        }
    }

    for (path, (edge_count, message)) in cross_layer {
        if let Some(entry) = facts.get_mut(&path) {
            entry.cross_layer = Some(CrossLayerFacts {
                edge_count,
                message: if edge_count > 1 {
                    format!("{message} ({edge_count} edge(s))")
                } else {
                    message
                },
            });
        }
    }

    for path in active_paths {
        let Some(zone_id) = zone_matches.get(path).and_then(|zone| zone.as_deref()) else {
            continue;
        };
        let fan_in = facts
            .get(path)
            .and_then(|entry| entry.fan_in_count)
            .unwrap_or_default();
        let fan_out = facts
            .get(path)
            .and_then(|entry| entry.fan_out_count)
            .unwrap_or_default();
        if fan_in == 0 && fan_out == 0 && !is_orphan_entrypoint(path) {
            if let Some(entry) = facts.get_mut(path) {
                let _ = zone_id;
                entry.orphan_module = true;
            }
        }
    }

    Ok(facts)
}

fn load_direct_neighbors(
    conn: &rusqlite::Connection,
    active_paths: &HashSet<String>,
) -> Result<(PathNeighbors, PathNeighbors)> {
    let mut outgoing = PathNeighbors::new();
    let mut incoming = PathNeighbors::new();
    for path in active_paths {
        outgoing.entry(path.clone()).or_default();
        incoming.entry(path.clone()).or_default();
    }

    let mut stmt = conn.prepare(
        "SELECT src_path, dst_path
         FROM file_graph_edges
         WHERE edge_kind IN ('ref_exact', 'ref_tail_unique')
         ORDER BY src_path ASC, dst_path ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows {
        let (src_path, dst_path) = row?;
        if !active_paths.contains(&src_path) || !active_paths.contains(&dst_path) {
            continue;
        }
        outgoing
            .entry(src_path.clone())
            .or_default()
            .insert(dst_path.clone());
        incoming.entry(dst_path).or_default().insert(src_path);
    }

    Ok((outgoing, incoming))
}

fn cycle_members(outgoing: &PathNeighbors) -> BTreeSet<String> {
    let reverse = reverse_neighbors(outgoing);
    let mut visited = BTreeSet::new();
    let mut order = Vec::new();
    for path in outgoing.keys() {
        dfs_finish_order(path, outgoing, &mut visited, &mut order);
    }

    let mut assigned = BTreeSet::new();
    let mut members = BTreeSet::new();
    for path in order.into_iter().rev() {
        if assigned.contains(&path) {
            continue;
        }
        let mut component = Vec::new();
        dfs_component(&path, &reverse, &mut assigned, &mut component);
        if component.len() > 1 {
            members.extend(component);
            continue;
        }
        if let Some(only) = component.pop() {
            let has_self_loop = outgoing
                .get(&only)
                .map(|neighbors| neighbors.contains(&only))
                .unwrap_or(false);
            if has_self_loop {
                members.insert(only);
            }
        }
    }
    members
}

fn build_zone_matches(
    active_paths: &HashSet<String>,
    policy: &StructuralPolicy,
) -> Result<HashMap<String, Option<String>>> {
    let zone_scopes = policy
        .zones
        .iter()
        .map(|zone| {
            Ok((
                zone.id.clone(),
                IndexScope::new(&IndexingOptions {
                    profile: None,
                    changed_since: None,
                    changed_since_commit: None,
                    include_paths: zone.paths.clone(),
                    exclude_paths: Vec::new(),
                    reindex: false,
                })?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    let mut matches = HashMap::with_capacity(active_paths.len());
    let mut sorted_paths = active_paths.iter().cloned().collect::<Vec<_>>();
    sorted_paths.sort();
    for path in sorted_paths {
        let matched = zone_scopes
            .iter()
            .filter(|(_, scope)| scope.allows(&path))
            .map(|(zone_id, _)| zone_id.clone())
            .collect::<Vec<_>>();
        if matched.len() > 1 {
            bail!(
                "structural policy matches path `{path}` to multiple zones: {}",
                matched.join(", ")
            );
        }
        matches.insert(path, matched.into_iter().next());
    }
    Ok(matches)
}

fn matched_zone_pair<'a>(
    src_zone: Option<&'a str>,
    dst_zone: Option<&'a str>,
    unmatched_behavior: StructuralUnmatchedBehavior,
) -> Option<(&'a str, &'a str)> {
    match (src_zone, dst_zone) {
        (Some(src_zone), Some(dst_zone)) => Some((src_zone, dst_zone)),
        (None, _) | (_, None) => match unmatched_behavior {
            StructuralUnmatchedBehavior::Allow | StructuralUnmatchedBehavior::Ignore => None,
        },
    }
}

fn cross_layer_message(
    policy: &StructuralPolicy,
    src_zone: &str,
    dst_zone: &str,
) -> Option<String> {
    if let Some(edge) = policy
        .forbidden_edges
        .iter()
        .find(|edge| edge.from == src_zone && edge.to == dst_zone)
    {
        return Some(match &edge.reason {
            Some(reason) => {
                format!("zone `{src_zone}` depends on forbidden zone `{dst_zone}`: {reason}")
            }
            None => format!("zone `{src_zone}` depends on forbidden zone `{dst_zone}`"),
        });
    }

    if policy.allowed_directions.is_empty() {
        return None;
    }

    if policy
        .allowed_directions
        .iter()
        .any(|direction| direction.from == src_zone && direction.to == dst_zone)
    {
        None
    } else {
        Some(format!(
            "zone `{src_zone}` depends on zone `{dst_zone}` outside allowed directions"
        ))
    }
}

fn is_orphan_entrypoint(path: &str) -> bool {
    let stem = Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    ORPHAN_ENTRYPOINTS.contains(&stem)
}

fn reverse_neighbors(outgoing: &PathNeighbors) -> PathNeighbors {
    let mut reverse = PathNeighbors::new();
    for path in outgoing.keys() {
        reverse.entry(path.clone()).or_default();
    }
    for (src, neighbors) in outgoing {
        for dst in neighbors {
            reverse.entry(dst.clone()).or_default().insert(src.clone());
        }
    }
    reverse
}

fn dfs_finish_order(
    path: &str,
    outgoing: &PathNeighbors,
    visited: &mut BTreeSet<String>,
    order: &mut Vec<String>,
) {
    if !visited.insert(path.to_string()) {
        return;
    }
    if let Some(neighbors) = outgoing.get(path) {
        for neighbor in neighbors {
            dfs_finish_order(neighbor, outgoing, visited, order);
        }
    }
    order.push(path.to_string());
}

fn dfs_component(
    path: &str,
    reverse: &PathNeighbors,
    assigned: &mut BTreeSet<String>,
    component: &mut Vec<String>,
) {
    if !assigned.insert(path.to_string()) {
        return;
    }
    component.push(path.to_string());
    if let Some(neighbors) = reverse.get(path) {
        for neighbor in neighbors {
            dfs_component(neighbor, reverse, assigned, component);
        }
    }
}

type PathNeighbors = BTreeMap<String, BTreeSet<String>>;
