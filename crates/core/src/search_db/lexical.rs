use std::collections::HashSet;

use anyhow::Result;
use rusqlite::{Connection, params};

use crate::model::SearchHit;
use crate::query_profile::{QueryProfile, derive_query_profile};

use super::boost::{extract_tokens, graph_boost};
use super::helpers::trim_preview;
use super::scoring::{
    compare_hits_desc, keep_top_hits, like_score, path_match_boost, rank_to_score,
};
use super::unicode;

const LIKE_PREFILTER_MULTIPLIER: i64 = 96;
const LIKE_PREFILTER_MIN_ROWS: i64 = 256;
const LIKE_PREFILTER_MAX_ROWS: i64 = 8_192;
const LIKE_GUARDED_SCAN_MULTIPLIER: usize = 384;
const LIKE_GUARDED_SCAN_MIN_ROWS: usize = 2_048;
const LIKE_GUARDED_SCAN_MAX_ROWS: usize = 65_536;

#[derive(Debug, Clone)]
struct CandidateRow {
    path: String,
    sample: String,
    size_bytes: i64,
    language: String,
    base_score: f32,
}

pub fn search_fts(conn: &Connection, query: &str, limit: i64) -> Result<Vec<SearchHit>> {
    let query_profile = derive_query_profile(query);
    let query_tokens = extract_tokens(query);
    let primary_fts_query = prepare_fts_query(&query_tokens, query_profile, true);
    if primary_fts_query.is_empty() {
        return Ok(Vec::new());
    }

    let mut raw = run_fts_query(conn, &primary_fts_query, limit)?;
    let fallback_fts_query = prepare_fts_query(&query_tokens, query_profile, false);
    if raw.is_empty() && fallback_fts_query != primary_fts_query {
        raw = run_fts_query(conn, &fallback_fts_query, limit)?;
    }

    let mut hits = Vec::with_capacity(raw.len());
    for row in raw {
        let graph = graph_boost(conn, &row.path, &query_tokens, query_profile)?;
        let path = path_match_boost(&row.path, &query_tokens);
        hits.push(SearchHit {
            path: row.path,
            preview: trim_preview(&row.sample, 260),
            score: row.base_score + graph + path,
            size_bytes: row.size_bytes,
            language: row.language,
        });
    }

    hits.sort_by(compare_hits_desc);
    Ok(hits)
}

fn run_fts_query(conn: &Connection, fts_query: &str, limit: i64) -> Result<Vec<CandidateRow>> {
    let mut stmt = conn.prepare(
        "SELECT f.path, f.sample, f.size_bytes, f.language, bm25(files_fts) as rank FROM files_fts JOIN files f ON f.path = files_fts.path WHERE files_fts MATCH ?1 ORDER BY rank LIMIT ?2",
    )?;

    let raw = stmt
        .query_map(params![fts_query, limit], |row| {
            let rank: f64 = row.get(4)?;
            Ok(CandidateRow {
                path: row.get(0)?,
                sample: row.get(1)?,
                size_bytes: row.get(2)?,
                language: row.get(3)?,
                base_score: rank_to_score(rank),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(raw)
}

pub fn search_like(conn: &Connection, query: &str, limit: i64) -> Result<Vec<SearchHit>> {
    let normalized = query.trim();
    if normalized.is_empty() {
        return Ok(Vec::new());
    }
    let query_key = unicode::normalize_match_key(normalized);
    if query_key.is_empty() {
        return Ok(Vec::new());
    }
    let query_tokens = extract_tokens(normalized);
    let keep_limit = usize::try_from(limit.max(1)).unwrap_or(usize::MAX);
    let mut best_hits = Vec::new();
    let mut seen_paths = HashSet::new();

    let prefiltered_rows = load_like_prefilter_rows(conn, normalized, limit)?;
    for row in prefiltered_rows {
        if let Some(hit) = candidate_hit_from_row(row, &query_key, &query_tokens) {
            seen_paths.insert(hit.path.clone());
            keep_top_hits(&mut best_hits, hit, keep_limit);
        }
    }
    if best_hits.len() >= keep_limit {
        best_hits.sort_by(compare_hits_desc);
        return Ok(best_hits);
    }

    let scan_budget = like_scan_budget(limit);
    let mut stmt = conn.prepare("SELECT path, sample, size_bytes, language FROM files")?;
    let mut rows = stmt.query([])?;
    let mut scanned_rows = 0_usize;
    while let Some(row) = rows.next()? {
        if scanned_rows >= scan_budget {
            break;
        }
        scanned_rows += 1;

        let candidate = CandidateRow {
            path: row.get(0)?,
            sample: row.get(1)?,
            size_bytes: row.get(2)?,
            language: row.get(3)?,
            base_score: 0.0,
        };
        if seen_paths.contains(&candidate.path) {
            continue;
        }
        if let Some(hit) = candidate_hit_from_row(candidate, &query_key, &query_tokens) {
            keep_top_hits(&mut best_hits, hit, keep_limit);
        }
    }

    best_hits.sort_by(compare_hits_desc);
    Ok(best_hits)
}

fn candidate_hit_from_row(
    row: CandidateRow,
    query_key: &str,
    query_tokens: &[String],
) -> Option<SearchHit> {
    let path_key = unicode::normalize_match_key(&row.path);
    let sample_key = unicode::normalize_match_key(&row.sample);
    if !path_key.contains(query_key) && !sample_key.contains(query_key) {
        return None;
    }

    let base_score = like_score(query_key, &path_key, &sample_key);
    let path_boost = path_match_boost(&row.path, query_tokens);
    Some(SearchHit {
        path: row.path,
        preview: trim_preview(&row.sample, 260),
        score: base_score + path_boost,
        size_bytes: row.size_bytes,
        language: row.language,
    })
}

fn load_like_prefilter_rows(
    conn: &Connection,
    query: &str,
    limit: i64,
) -> Result<Vec<CandidateRow>> {
    let prefilter_limit = like_prefilter_limit(limit);
    let mut stmt = conn.prepare(
        r#"
        SELECT path, sample, size_bytes, language
        FROM files
        WHERE INSTR(LOWER(path), LOWER(?1)) > 0
           OR INSTR(LOWER(sample), LOWER(?1)) > 0
        LIMIT ?2
        "#,
    )?;
    let rows = stmt
        .query_map(params![query, prefilter_limit], |row| {
            Ok(CandidateRow {
                path: row.get(0)?,
                sample: row.get(1)?,
                size_bytes: row.get(2)?,
                language: row.get(3)?,
                base_score: 0.0,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub(super) fn like_prefilter_limit(limit: i64) -> i64 {
    let keep = limit.max(1);
    keep.saturating_mul(LIKE_PREFILTER_MULTIPLIER)
        .clamp(LIKE_PREFILTER_MIN_ROWS, LIKE_PREFILTER_MAX_ROWS)
}

pub(super) fn like_scan_budget(limit: i64) -> usize {
    let keep = usize::try_from(limit.max(1)).unwrap_or(usize::MAX / 2);
    keep.saturating_mul(LIKE_GUARDED_SCAN_MULTIPLIER)
        .clamp(LIKE_GUARDED_SCAN_MIN_ROWS, LIKE_GUARDED_SCAN_MAX_ROWS)
}

fn prepare_fts_query(
    tokens: &[String],
    query_profile: QueryProfile,
    prefer_strict: bool,
) -> String {
    if tokens.is_empty() {
        return String::new();
    }

    if prefer_strict && matches!(query_profile, QueryProfile::Precise) && tokens.len() > 1 {
        return tokens
            .iter()
            .map(|token| format!("{}*", token))
            .collect::<Vec<_>>()
            .join(" ");
    }

    tokens
        .iter()
        .map(|token| format!("{}*", token))
        .collect::<Vec<_>>()
        .join(" OR ")
}
