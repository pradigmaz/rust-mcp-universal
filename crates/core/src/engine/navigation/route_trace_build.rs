use std::collections::BTreeSet;

use crate::engine::investigation::common::{
    build_anchor, classify_route_segment, classify_route_source_kind, detect_language,
    route_kind_label, source_span_from_position,
};
use crate::model::{CallPathResult, RouteGap, RoutePath, RouteSegment};

pub(super) fn route_from_call_path(
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

pub(super) fn collapse_route_segments(raw_segments: Vec<RouteSegment>) -> Vec<RouteSegment> {
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

pub(super) fn fallback_route(
    start: &crate::engine::investigation::common::CandidateFile,
) -> RoutePath {
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

pub(super) fn dedupe_gaps(gaps: Vec<RouteGap>) -> Vec<RouteGap> {
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

pub(super) fn route_trace_capability(
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

fn route_confidence(segments: &[RouteSegment]) -> f32 {
    if segments.is_empty() {
        return 0.0;
    }
    (segments.iter().map(|segment| segment.score).sum::<f32>() / segments.len() as f32)
        .clamp(0.1, 1.0)
}

fn step_symbol(evidence: &str) -> Option<String> {
    let trimmed = evidence.trim();
    (!trimmed.is_empty() && !trimmed.contains(' ')).then(|| trimmed.to_string())
}
