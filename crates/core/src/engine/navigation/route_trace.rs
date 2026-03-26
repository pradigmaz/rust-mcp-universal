use std::collections::HashSet;

use anyhow::Result;

use super::super::Engine;
use super::route_trace_targets::{MAX_ROUTE_HOPS, collect_target_candidates};
#[path = "route_trace_build.rs"]
mod route_trace_build;
#[path = "route_trace_ranking.rs"]
mod route_trace_ranking;

use crate::engine::investigation::common::{
    canonical_seed, classify_route_segment, collect_candidates, route_kind_label,
};
use crate::model::{
    ConceptSeedKind, RouteGap, RoutePath, RouteTraceResult,
};
use route_trace_build::{
    dedupe_gaps, fallback_route, route_from_call_path, route_trace_capability,
};
use route_trace_ranking::{
    RankedRoute, compare_ranked_routes, prioritize_start_candidates, route_relevance,
};

pub(crate) fn route_trace(
    engine: &Engine,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
) -> Result<RouteTraceResult> {
    let (seed, starts, unsupported_sources) = collect_candidates(engine, seed, seed_kind, limit)?;
    let starts = prioritize_start_candidates(starts, seed.seed_kind);
    let mut ranked_routes = Vec::new();
    let mut unresolved_gaps = Vec::new();

    for start in starts.iter().take(limit.max(1)) {
        let start_kind = classify_route_segment(&start.path);
        let mut found_cross_layer = false;
        let targets = match collect_target_candidates(engine, start) {
            Ok(targets) => targets,
            Err(err) => {
                unresolved_gaps.push(RouteGap {
                    from_kind: Some(start_kind),
                    to_kind: None,
                    reason: format!("target_collection_failed:{err}"),
                    last_resolved_path: Some(start.path.clone()),
                });
                ranked_routes.push(RankedRoute {
                    sequence: vec![route_kind_label(start_kind).to_string()],
                    route: fallback_route(start),
                    relevance: route_relevance(seed.seed.as_str(), seed.seed_kind, start, None),
                });
                continue;
            }
        };

        for target in targets {
            let call_path = match engine.call_path(&start.path, &target.path, MAX_ROUTE_HOPS) {
                Ok(call_path) => call_path,
                Err(err) => {
                    unresolved_gaps.push(RouteGap {
                        from_kind: Some(start_kind),
                        to_kind: Some(target.kind),
                        reason: format!("call_path_failed:{}:{err}", target.source_kind),
                        last_resolved_path: Some(start.path.clone()),
                    });
                    continue;
                }
            };
            if !call_path.found {
                unresolved_gaps.push(RouteGap {
                    from_kind: Some(start_kind),
                    to_kind: Some(target.kind),
                    reason: format!("call_path_unresolved:{}", target.source_kind),
                    last_resolved_path: Some(start.path.clone()),
                });
                continue;
            }

            let route = route_from_call_path(start, &call_path);
            if route.segments.len() <= 1 {
                continue;
            }
            found_cross_layer = true;
            ranked_routes.push(RankedRoute {
                sequence: route
                    .segments
                    .iter()
                    .map(|segment| route_kind_label(segment.kind).to_string())
                    .collect(),
                relevance: route_relevance(seed.seed.as_str(), seed.seed_kind, start, Some(&route)),
                route,
            });
        }

        if !found_cross_layer {
            ranked_routes.push(RankedRoute {
                sequence: vec![route_kind_label(start_kind).to_string()],
                route: fallback_route(start),
                relevance: route_relevance(seed.seed.as_str(), seed.seed_kind, start, None),
            });
        }
    }

    ranked_routes.sort_by(compare_ranked_routes);

    let mut seen_sequences = HashSet::new();
    let mut deduped = Vec::new();
    for ranked in ranked_routes {
        let key = ranked.sequence.join(">");
        if seen_sequences.insert(key) {
            deduped.push(ranked.route);
        }
    }

    let best_route = deduped.first().cloned().unwrap_or_else(|| RoutePath {
        segments: Vec::new(),
        total_hops: 0,
        total_weight: 0.0,
        collapsed_hops: 0,
        confidence: 0.0,
    });
    let alternate_routes = deduped.into_iter().skip(1).take(2).collect::<Vec<_>>();
    let unresolved_gaps = dedupe_gaps(unresolved_gaps);
    let capability_status = route_trace_capability(
        !best_route.segments.is_empty(),
        !unresolved_gaps.is_empty(),
        &unsupported_sources,
    );

    Ok(RouteTraceResult {
        seed: canonical_seed(seed.seed.as_str(), seed.seed_kind),
        best_route: best_route.clone(),
        alternate_routes,
        unresolved_gaps,
        capability_status,
        unsupported_sources,
        confidence: best_route.confidence,
    })
}
#[cfg(test)]
#[path = "route_trace_tests.rs"]
mod tests;
