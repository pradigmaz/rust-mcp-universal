use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::engine::Engine;
use crate::model::{
    ConceptSeed, ConceptSeedKind, InvestigationAnchor, PrivacyMode, QueryOptions, RouteSegmentKind,
    SearchHit, SourceSpan,
};

use super::candidate_relevance::retain_query_relevant_candidates;
use super::path_helpers::{display_path, source_fs_path};

#[derive(Debug, Clone)]
pub(crate) struct CandidateFile {
    pub(crate) path: String,
    pub(crate) language: String,
    pub(crate) line: Option<usize>,
    pub(crate) column: Option<usize>,
    pub(crate) symbol: Option<String>,
    pub(crate) symbol_kind: Option<String>,
    pub(crate) source_kind: String,
    pub(crate) match_kind: CandidateMatchKind,
    pub(crate) score: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CandidateMatchKind {
    ExactSymbol,
    PartialSymbol,
    QuerySearch,
    PathAnchor,
}

pub(crate) fn canonical_seed(seed: &str, seed_kind: ConceptSeedKind) -> ConceptSeed {
    ConceptSeed {
        seed: seed.trim().to_string(),
        seed_kind,
    }
}

pub(crate) fn collect_candidates(
    engine: &Engine,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
) -> Result<(ConceptSeed, Vec<CandidateFile>, Vec<String>)> {
    let normalized = canonical_seed(seed, seed_kind);
    let mut candidates = match seed_kind {
        ConceptSeedKind::Query => collect_query_candidates(engine, seed, limit.max(1))?,
        ConceptSeedKind::Symbol => collect_symbol_candidates(engine, seed, limit.max(1))?,
        ConceptSeedKind::Path => vec![candidate_from_path(seed, None)],
        ConceptSeedKind::PathLine => {
            let (path, line) = parse_path_line_seed(seed)?;
            vec![candidate_from_path(&path, Some(line))]
        }
    };
    if matches!(seed_kind, ConceptSeedKind::Query) {
        candidates =
            retain_query_relevant_candidates(seed, candidates, limit.max(1).saturating_mul(3));
    }
    let mut seen = HashSet::new();
    candidates.retain(|candidate| {
        seen.insert((
            candidate.path.clone(),
            candidate.symbol.clone(),
            candidate.line.unwrap_or(0),
        ))
    });
    let unsupported_sources = candidates
        .iter()
        .filter(|candidate| !is_supported_language(&candidate.language, &candidate.path))
        .map(|candidate| format!("{}:{}", candidate.language, candidate.path))
        .collect::<Vec<_>>();
    Ok((normalized, candidates, unsupported_sources))
}

pub(crate) fn capability_status(
    produced_items: usize,
    total_candidates: usize,
    unsupported_sources: &[String],
) -> String {
    if produced_items == 0 {
        if total_candidates == 0 || !unsupported_sources.is_empty() {
            "unsupported".to_string()
        } else {
            "partial".to_string()
        }
    } else if unsupported_sources.is_empty() {
        "supported".to_string()
    } else {
        "partial".to_string()
    }
}

pub(crate) fn classify_route_segment(path: &str) -> RouteSegmentKind {
    let lowered = path.replace('\\', "/").to_ascii_lowercase();
    if lowered.contains("/tests/")
        || lowered.contains("/test/")
        || lowered.ends_with("_test.rs")
        || lowered.ends_with("_test.py")
        || lowered.ends_with("test.rs")
        || lowered.ends_with("test.py")
        || lowered
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("test_"))
    {
        return RouteSegmentKind::Test;
    }
    if lowered.contains("/alembic/")
        || lowered.starts_with("alembic/")
        || lowered.contains("/migrations/")
        || lowered.starts_with("migrations/")
        || lowered.contains("/versions/")
        || lowered.starts_with("versions/")
    {
        return RouteSegmentKind::Migration;
    }
    if lowered.contains("validator") {
        return RouteSegmentKind::Service;
    }
    if lowered.contains("crud") {
        return RouteSegmentKind::Crud;
    }
    if lowered.contains("query") || lowered.ends_with(".sql") {
        return RouteSegmentKind::Query;
    }
    if lowered.contains("service") {
        return RouteSegmentKind::Service;
    }
    if lowered.contains("/lib/api/")
        || lowered.starts_with("lib/api/")
        || lowered.starts_with("frontend/src/lib/api/")
    {
        return RouteSegmentKind::ApiClient;
    }
    if lowered.contains("endpoint")
        || lowered.contains("controller")
        || lowered.contains("/api/")
        || lowered.contains("/routes/")
    {
        return RouteSegmentKind::Endpoint;
    }
    if lowered.contains("client") {
        return RouteSegmentKind::ApiClient;
    }
    if lowered.contains("hook")
        || lowered.contains("/frontend/")
        || lowered.starts_with("frontend/")
        || lowered.contains("/ui/")
        || lowered.starts_with("ui/")
    {
        return RouteSegmentKind::Ui;
    }
    RouteSegmentKind::Unknown
}

pub(crate) fn classify_route_source_kind(path: &str) -> &'static str {
    let lowered = path.replace('\\', "/").to_ascii_lowercase();
    if lowered.contains("/tests/")
        || lowered.contains("/test/")
        || lowered.ends_with("_test.rs")
        || lowered.ends_with("_test.py")
        || lowered.ends_with("test.rs")
        || lowered.ends_with("test.py")
        || lowered
            .rsplit('/')
            .next()
            .is_some_and(|name| name.starts_with("test_"))
    {
        return "test";
    }
    if lowered.contains("/alembic/")
        || lowered.starts_with("alembic/")
        || lowered.contains("/migrations/")
        || lowered.starts_with("migrations/")
        || lowered.contains("/versions/")
        || lowered.starts_with("versions/")
    {
        return "migration";
    }
    if lowered.contains("validator") {
        return "validator";
    }
    if lowered.contains("crud") {
        return "crud";
    }
    if lowered.contains("query") || lowered.ends_with(".sql") {
        return "query";
    }
    if lowered.contains("model") || lowered.contains("schema") {
        return "model";
    }
    if lowered.contains("service") {
        return "service";
    }
    if lowered.contains("/lib/api/")
        || lowered.starts_with("lib/api/")
        || lowered.starts_with("frontend/src/lib/api/")
    {
        return "api_client";
    }
    if lowered.contains("endpoint")
        || lowered.contains("controller")
        || lowered.contains("/api/")
        || lowered.contains("/routes/")
    {
        return "endpoint";
    }
    if lowered.contains("client") {
        return "api_client";
    }
    if lowered.contains("hook")
        || lowered.contains("/frontend/")
        || lowered.starts_with("frontend/")
        || lowered.contains("/ui/")
        || lowered.starts_with("ui/")
    {
        return "ui";
    }
    if lowered.contains("constraint") || lowered.contains("index") {
        return "constraint_source";
    }
    "unknown"
}

pub(crate) fn route_kind_label(kind: RouteSegmentKind) -> &'static str {
    match kind {
        RouteSegmentKind::Ui => "ui",
        RouteSegmentKind::ApiClient => "api_client",
        RouteSegmentKind::Endpoint => "endpoint",
        RouteSegmentKind::Service => "service",
        RouteSegmentKind::Crud => "crud",
        RouteSegmentKind::Query => "query",
        RouteSegmentKind::Test => "test",
        RouteSegmentKind::Migration => "migration",
        RouteSegmentKind::Unknown => "unknown",
    }
}

pub(crate) fn detect_language(path: &str, fallback: &str) -> String {
    if !fallback.trim().is_empty() {
        return fallback.to_string();
    }
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust".to_string(),
        Some("py") => "python".to_string(),
        Some("ts") | Some("tsx") => "typescript".to_string(),
        Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => "javascript".to_string(),
        Some("sql") => "sql".to_string(),
        Some("prisma") => "prisma".to_string(),
        Some("toml") => "toml".to_string(),
        Some("md") => "markdown".to_string(),
        _ => "text".to_string(),
    }
}

pub(crate) fn is_supported_language(language: &str, path: &str) -> bool {
    matches!(
        detect_language(path, language).as_str(),
        "rust" | "python" | "typescript" | "javascript" | "sql" | "prisma"
    )
}

pub(super) fn resolve_source_path(project_root: &Path, path: &str) -> PathBuf {
    let candidate = source_fs_path(path);
    if candidate.is_absolute() {
        candidate
    } else {
        project_root.join(candidate)
    }
}

pub(super) fn read_source(project_root: &Path, path: &str) -> Result<String> {
    let full_path = resolve_source_path(project_root, path);
    fs::read_to_string(&full_path)
        .map_err(|err| anyhow!("failed to read source {}: {err}", full_path.display()))
}

pub(crate) fn build_anchor(candidate: &CandidateFile) -> InvestigationAnchor {
    InvestigationAnchor {
        path: candidate.path.clone(),
        language: candidate.language.clone(),
        symbol: candidate.symbol.clone(),
        kind: candidate.symbol_kind.clone(),
        line: candidate.line,
        column: candidate.column,
    }
}

pub(crate) fn normalized_values(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut set = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            set.insert(trimmed.to_string());
        }
    }
    set.into_iter().collect()
}

pub(crate) fn source_span_from_position(
    line: Option<usize>,
    column: Option<usize>,
) -> Option<SourceSpan> {
    line.map(|start_line| SourceSpan {
        start_line,
        end_line: start_line,
        start_column: column.or(Some(1)),
        end_column: column,
    })
}

fn collect_query_candidates(
    engine: &Engine,
    seed: &str,
    limit: usize,
) -> Result<Vec<CandidateFile>> {
    let mut out = collect_search_hits(engine, seed, limit.saturating_mul(3))?;
    out.extend(collect_symbol_candidates(engine, seed, limit)?);
    Ok(out)
}

fn collect_search_hits(engine: &Engine, seed: &str, limit: usize) -> Result<Vec<CandidateFile>> {
    let hits = engine.search(&QueryOptions {
        query: seed.trim().to_string(),
        limit: limit.max(1),
        detailed: false,
        semantic: false,
        semantic_fail_mode: Default::default(),
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
        agent_intent_mode: None,
    })?;
    Ok(hits.into_iter().map(candidate_from_hit).collect())
}

fn collect_symbol_candidates(
    engine: &Engine,
    seed: &str,
    limit: usize,
) -> Result<Vec<CandidateFile>> {
    Ok(engine
        .symbol_lookup(seed.trim(), limit.max(1))?
        .into_iter()
        .map(|symbol| CandidateFile {
            path: display_path(&symbol.path),
            language: symbol.language,
            line: symbol.line,
            column: symbol.column,
            symbol: Some(symbol.name),
            symbol_kind: Some(symbol.kind),
            source_kind: "symbol_lookup".to_string(),
            match_kind: if symbol.exact {
                CandidateMatchKind::ExactSymbol
            } else {
                CandidateMatchKind::PartialSymbol
            },
            score: if symbol.exact { 1.0 } else { 0.8 },
        })
        .collect())
}

fn candidate_from_hit(hit: SearchHit) -> CandidateFile {
    let path = display_path(&hit.path);
    CandidateFile {
        path: path.clone(),
        language: detect_language(&path, &hit.language),
        line: None,
        column: None,
        symbol: None,
        symbol_kind: None,
        source_kind: "search_candidate".to_string(),
        match_kind: CandidateMatchKind::QuerySearch,
        score: hit.score,
    }
}

fn candidate_from_path(path: &str, line: Option<usize>) -> CandidateFile {
    CandidateFile {
        path: path.trim().to_string(),
        language: detect_language(path, ""),
        line,
        column: None,
        symbol: None,
        symbol_kind: None,
        source_kind: "path_anchor".to_string(),
        match_kind: CandidateMatchKind::PathAnchor,
        score: 1.0,
    }
}

fn parse_path_line_seed(seed: &str) -> Result<(String, usize)> {
    let trimmed = seed.trim();
    let (path, line) = trimmed
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("path_line seed must use `path:line` format"))?;
    let line = line
        .parse::<usize>()
        .map_err(|_| anyhow!("path_line seed must end with integer line number"))?;
    Ok((path.to_string(), line.max(1)))
}
