use std::collections::HashSet;

use anyhow::Result;

use super::super::Engine;
use crate::engine::investigation::common::{
    CandidateFile, classify_route_segment, classify_route_source_kind, detect_language,
    is_supported_language,
};
use crate::engine::investigation::path_helpers::display_path;
use crate::model::RouteSegmentKind;

pub(super) const MAX_ROUTE_HOPS: usize = 6;
const MAX_TARGETS_PER_START: usize = 8;

#[derive(Debug, Clone)]
pub(super) struct TargetCandidate {
    pub(super) path: String,
    pub(super) kind: RouteSegmentKind,
    pub(super) source_kind: String,
}

pub(super) fn collect_target_candidates(
    engine: &Engine,
    start: &CandidateFile,
) -> Result<Vec<TargetCandidate>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for hit in engine.related_files(&start.path, MAX_TARGETS_PER_START)? {
        push_target(&mut out, &mut seen, hit.path, hit.language);
    }
    if let Some(symbol) = start.symbol.as_deref() {
        for hit in engine.symbol_references(symbol, MAX_TARGETS_PER_START)? {
            push_target(&mut out, &mut seen, hit.path, hit.language);
        }
    }
    for (path, language) in adjacent_repo_targets(engine, start) {
        push_target(&mut out, &mut seen, path, language);
    }
    Ok(out)
}

fn push_target(
    out: &mut Vec<TargetCandidate>,
    seen: &mut HashSet<String>,
    path: String,
    _language: String,
) {
    let path = display_path(&path);
    let kind = classify_route_segment(&path);
    let source_kind = classify_route_source_kind(&path).to_string();
    if !seen.insert(path.clone()) || !target_is_meaningful(kind, source_kind.as_str()) {
        return;
    }
    out.push(TargetCandidate {
        path,
        kind,
        source_kind,
    });
}

fn target_is_meaningful(kind: RouteSegmentKind, source_kind: &str) -> bool {
    !matches!(kind, RouteSegmentKind::Unknown) || matches!(source_kind, "validator" | "model")
}

fn adjacent_repo_targets(engine: &Engine, start: &CandidateFile) -> Vec<(String, String)> {
    let normalized = start.path.replace('\\', "/");
    let stem = normalized
        .rsplit('/')
        .next()
        .unwrap_or(normalized.as_str())
        .split('.')
        .next()
        .unwrap_or(normalized.as_str());
    let tokens = stem
        .split('_')
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        return Vec::new();
    }

    let Ok(conn) = engine.open_db_read_only() else {
        return Vec::new();
    };
    let Ok(mut stmt) =
        conn.prepare("SELECT path, language FROM files WHERE path <> ?1 ORDER BY path ASC")
    else {
        return Vec::new();
    };
    let Ok(rows) = stmt.query_map([&normalized], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for row in rows.flatten() {
        let (raw_path, language) = row;
        let path = display_path(&raw_path);
        let lowered = path.to_ascii_lowercase();
        if !tokens.iter().any(|token| lowered.contains(token)) {
            continue;
        }
        let kind = classify_route_segment(&path);
        let source_kind = classify_route_source_kind(&path);
        if !matches!(
            kind,
            RouteSegmentKind::Crud
                | RouteSegmentKind::Query
                | RouteSegmentKind::Test
                | RouteSegmentKind::Migration
        ) && !matches!(source_kind, "model")
        {
            continue;
        }
        let detected_language = detect_language(&path, &language);
        if !is_supported_language(&detected_language, &path) {
            continue;
        }
        out.push((path.clone(), detected_language));
        if out.len() >= MAX_TARGETS_PER_START {
            break;
        }
    }
    out
}
