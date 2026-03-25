use std::collections::HashSet;

use anyhow::Result;

use crate::engine::Engine;
use crate::model::{RouteSegment, RouteSegmentKind};

use super::common::{
    CandidateFile, classify_route_segment, classify_route_source_kind, detect_language,
    source_span_from_position,
};

pub(super) fn build_route(engine: &Engine, candidate: &CandidateFile) -> Result<Vec<RouteSegment>> {
    let mut segments = vec![RouteSegment {
        kind: classify_route_segment(&candidate.path),
        path: candidate.path.clone(),
        language: candidate.language.clone(),
        evidence: candidate.source_kind.clone(),
        anchor_symbol: candidate.symbol.clone(),
        source_span: source_span_from_position(candidate.line, candidate.column),
        relation_kind: "self".to_string(),
        source_kind: classify_route_source_kind(&candidate.path).to_string(),
        score: candidate.score,
    }];
    let mut seen = HashSet::from([candidate.path.clone()]);

    for hit in engine.related_files(&candidate.path, 6)? {
        if !seen.insert(hit.path.clone()) {
            continue;
        }
        let path = hit.path;
        let source_kind = classify_route_source_kind(&path).to_string();
        segments.push(RouteSegment {
            kind: classify_route_segment(&path),
            path,
            language: hit.language,
            evidence: format!(
                "related_files score={:.2} refs={} deps={} symbols={}",
                hit.score, hit.ref_overlap, hit.dep_overlap, hit.symbol_overlap
            ),
            anchor_symbol: None,
            source_span: None,
            relation_kind: "related_file".to_string(),
            source_kind,
            score: hit.score,
        });
    }

    if let Some(symbol) = candidate.symbol.as_deref() {
        for hit in engine.symbol_references(symbol, 4)? {
            if !seen.insert(hit.path.clone()) {
                continue;
            }
            let path = hit.path;
            let source_kind = classify_route_source_kind(&path).to_string();
            segments.push(RouteSegment {
                kind: classify_route_segment(&path),
                path,
                language: hit.language,
                evidence: format!(
                    "symbol_references refs={} exact={}",
                    hit.ref_count, hit.exact
                ),
                anchor_symbol: Some(symbol.to_string()),
                source_span: source_span_from_position(hit.line, hit.column),
                relation_kind: "symbol_neighbor".to_string(),
                source_kind,
                score: if hit.exact { 0.85 } else { 0.7 },
            });
        }
    }

    for (path, language) in adjacent_repo_paths(engine, candidate) {
        if !seen.insert(path.clone()) {
            continue;
        }
        let kind = classify_route_segment(&path);
        let source_kind = classify_route_source_kind(&path).to_string();
        segments.push(RouteSegment {
            kind,
            path: path.clone(),
            language,
            evidence: "adjacent_repo_path".to_string(),
            anchor_symbol: None,
            source_span: None,
            relation_kind: relation_kind_for_adjacent(kind).to_string(),
            source_kind,
            score: 0.45,
        });
    }

    segments[1..].sort_by(|left, right| {
        route_rank(left.kind)
            .cmp(&route_rank(right.kind))
            .then_with(|| right.score.total_cmp(&left.score))
    });
    Ok(segments)
}

fn adjacent_repo_paths(engine: &Engine, candidate: &CandidateFile) -> Vec<(String, String)> {
    let normalized = candidate.path.replace('\\', "/");
    let file_name = normalized.rsplit('/').next().unwrap_or(normalized.as_str());
    let stem = file_name.split('.').next().unwrap_or(file_name);
    let tokens = stem
        .split('_')
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return Vec::new();
    }

    let mut out = Vec::new();
    let Ok(conn) = engine.open_db_read_only() else {
        return out;
    };
    let Ok(mut stmt) =
        conn.prepare("SELECT path, language FROM files WHERE path <> ?1 ORDER BY path ASC")
    else {
        return out;
    };
    let Ok(rows) = stmt.query_map([&normalized], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) else {
        return out;
    };

    for row in rows.flatten() {
        let (relative, language) = row;
        let lowered = relative.to_ascii_lowercase();
        if !tokens.iter().any(|token| lowered.contains(token)) {
            continue;
        }
        let kind = classify_route_segment(&relative);
        if !matches!(
            kind,
            RouteSegmentKind::Test
                | RouteSegmentKind::Migration
                | RouteSegmentKind::Query
                | RouteSegmentKind::Crud
                | RouteSegmentKind::Unknown
        ) {
            continue;
        }
        out.push((relative.clone(), detect_language(&relative, &language)));
        if out.len() >= 6 {
            break;
        }
    }
    out
}

fn route_rank(kind: RouteSegmentKind) -> usize {
    match kind {
        RouteSegmentKind::Ui => 0,
        RouteSegmentKind::ApiClient => 1,
        RouteSegmentKind::Endpoint => 2,
        RouteSegmentKind::Service => 3,
        RouteSegmentKind::Crud => 4,
        RouteSegmentKind::Query => 5,
        RouteSegmentKind::Migration => 6,
        RouteSegmentKind::Test => 7,
        RouteSegmentKind::Unknown => 8,
    }
}

fn relation_kind_for_adjacent(kind: RouteSegmentKind) -> &'static str {
    match kind {
        RouteSegmentKind::Test => "test_anchor",
        RouteSegmentKind::Crud | RouteSegmentKind::Query => "query_anchor",
        RouteSegmentKind::Migration => "constraint_anchor",
        _ => "adjacent_repo_path",
    }
}
