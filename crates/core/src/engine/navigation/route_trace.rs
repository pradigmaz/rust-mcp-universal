use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};

use anyhow::Result;

use super::super::Engine;
use super::route_trace_targets::{MAX_ROUTE_HOPS, collect_target_candidates};
use crate::engine::investigation::common::{
    build_anchor, canonical_seed, classify_route_segment, classify_route_source_kind,
    collect_candidates, detect_language, route_kind_label, source_span_from_position,
};
use crate::model::{
    CallPathResult, ConceptSeedKind, RouteGap, RoutePath, RouteSegment, RouteTraceResult,
};

#[derive(Debug, Clone)]
struct RankedRoute {
    route: RoutePath,
    sequence: Vec<String>,
}

pub(crate) fn route_trace(
    engine: &Engine,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
) -> Result<RouteTraceResult> {
    let (seed, starts, unsupported_sources) = collect_candidates(engine, seed, seed_kind, limit)?;
    let starts = prioritize_start_candidates(starts);
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
                route,
            });
        }

        if !found_cross_layer {
            ranked_routes.push(RankedRoute {
                sequence: vec![route_kind_label(start_kind).to_string()],
                route: fallback_route(start),
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

fn route_from_call_path(
    start: &crate::engine::investigation::common::CandidateFile,
    call_path: &CallPathResult,
) -> RoutePath {
    let mut raw_segments = vec![RouteSegment {
        kind: classify_route_segment(&start.path),
        path: start.path.clone(),
        language: start.language.clone(),
        evidence: start.source_kind.clone(),
        anchor_symbol: start.symbol.clone(),
        source_span: source_span_from_position(start.line, start.column),
        relation_kind: "self".to_string(),
        source_kind: classify_route_source_kind(&start.path).to_string(),
        score: start.score,
    }];

    for step in &call_path.steps {
        raw_segments.push(RouteSegment {
            kind: classify_route_segment(&step.to_path),
            path: step.to_path.clone(),
            language: detect_language(&step.to_path, ""),
            evidence: format!(
                "call_path edge={} raw_count={} weight={:.2} evidence={}",
                step.edge_kind, step.raw_count, step.weight, step.evidence
            ),
            anchor_symbol: step_symbol(step.evidence.as_str()),
            source_span: source_span_from_position(step.line, step.column),
            relation_kind: step.edge_kind.clone(),
            source_kind: classify_route_source_kind(&step.to_path).to_string(),
            score: (step.weight / (step.raw_count.max(1) as f32)).clamp(0.2, 1.0),
        });
    }

    let collapsed = collapse_route_segments(raw_segments);
    RoutePath {
        collapsed_hops: call_path.path.len().saturating_sub(collapsed.len()),
        confidence: route_confidence(&collapsed),
        segments: collapsed,
        total_hops: call_path.hops,
        total_weight: call_path.total_weight,
    }
}

fn collapse_route_segments(raw_segments: Vec<RouteSegment>) -> Vec<RouteSegment> {
    let mut collapsed: Vec<RouteSegment> = Vec::new();
    for segment in raw_segments {
        if let Some(last) = collapsed.last_mut()
            && last.kind == segment.kind
        {
            last.score = last.score.max(segment.score);
            if last.evidence != segment.evidence {
                last.evidence = format!("{} | collapsed:{}", last.evidence, segment.path);
            }
            continue;
        }
        collapsed.push(segment);
    }
    collapsed
}

fn route_confidence(segments: &[RouteSegment]) -> f32 {
    if segments.is_empty() {
        return 0.0;
    }
    (segments.iter().map(|segment| segment.score).sum::<f32>() / segments.len() as f32)
        .clamp(0.1, 1.0)
}

fn fallback_route(start: &crate::engine::investigation::common::CandidateFile) -> RoutePath {
    RoutePath {
        segments: vec![RouteSegment {
            kind: classify_route_segment(&start.path),
            path: build_anchor(start).path,
            language: start.language.clone(),
            evidence: "fallback_start_anchor".to_string(),
            anchor_symbol: start.symbol.clone(),
            source_span: source_span_from_position(start.line, start.column),
            relation_kind: "self".to_string(),
            source_kind: classify_route_source_kind(&start.path).to_string(),
            score: start.score.clamp(0.2, 1.0),
        }],
        total_hops: 0,
        total_weight: 0.0,
        collapsed_hops: 0,
        confidence: (start.score * 0.75).clamp(0.1, 1.0),
    }
}

fn dedupe_gaps(gaps: Vec<RouteGap>) -> Vec<RouteGap> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for gap in gaps {
        let key = (
            gap.from_kind
                .map(route_kind_label)
                .unwrap_or("none")
                .to_string(),
            gap.to_kind
                .map(route_kind_label)
                .unwrap_or("none")
                .to_string(),
            gap.reason.clone(),
            gap.last_resolved_path.clone().unwrap_or_default(),
        );
        if seen.insert(key) {
            out.push(gap);
        }
    }
    out
}

fn route_trace_capability(
    has_best_route: bool,
    has_gaps: bool,
    unsupported_sources: &[String],
) -> String {
    if !has_best_route {
        return if unsupported_sources.is_empty() {
            "partial".to_string()
        } else {
            "unsupported".to_string()
        };
    }
    if has_gaps || !unsupported_sources.is_empty() {
        "partial".to_string()
    } else {
        "supported".to_string()
    }
}

fn compare_ranked_routes(left: &RankedRoute, right: &RankedRoute) -> Ordering {
    right
        .route
        .total_weight
        .total_cmp(&left.route.total_weight)
        .then_with(|| right.route.segments.len().cmp(&left.route.segments.len()))
        .then_with(|| right.route.confidence.total_cmp(&left.route.confidence))
        .then_with(|| left.sequence.cmp(&right.sequence))
}

fn prioritize_start_candidates(
    mut starts: Vec<crate::engine::investigation::common::CandidateFile>,
) -> Vec<crate::engine::investigation::common::CandidateFile> {
    starts.sort_by(|left, right| {
        start_priority(right)
            .cmp(&start_priority(left))
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| left.path.cmp(&right.path))
    });
    starts
}

fn start_priority(candidate: &crate::engine::investigation::common::CandidateFile) -> usize {
    let kind = classify_route_segment(&candidate.path);
    let source_kind = classify_route_source_kind(&candidate.path);
    match kind {
        crate::model::RouteSegmentKind::Endpoint => 6,
        crate::model::RouteSegmentKind::Ui => 5,
        crate::model::RouteSegmentKind::Service => 4,
        crate::model::RouteSegmentKind::Crud => 3,
        crate::model::RouteSegmentKind::ApiClient => 3,
        crate::model::RouteSegmentKind::Query => 2,
        crate::model::RouteSegmentKind::Test | crate::model::RouteSegmentKind::Migration => 1,
        crate::model::RouteSegmentKind::Unknown => {
            if matches!(source_kind, "validator") {
                4
            } else if matches!(source_kind, "model") {
                2
            } else {
                0
            }
        }
    }
}

fn step_symbol(evidence: &str) -> Option<String> {
    let trimmed = evidence.trim();
    (!trimmed.is_empty() && !trimmed.contains(' ')).then(|| trimmed.to_string())
}

#[cfg(test)]
#[path = "route_trace_tests.rs"]
mod tests;
