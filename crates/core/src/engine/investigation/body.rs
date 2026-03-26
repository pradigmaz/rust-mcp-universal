use std::time::Instant;

use anyhow::Result;
use rusqlite::{OptionalExtension, params};

use crate::engine::Engine;
use crate::model::{
    ConceptSeedKind, SourceSpan, SymbolBodyAmbiguityStatus, SymbolBodyItem,
    SymbolBodyResolutionKind, SymbolBodyResult, SymbolBodyTimings,
};

use super::common::{
    CandidateFile, CandidateMatchKind, build_anchor, capability_status, collect_candidates,
    detect_language, is_supported_language, read_source,
};
use super::path_helpers::index_lookup_paths;

pub(super) fn symbol_body(
    engine: &Engine,
    seed: &str,
    seed_kind: ConceptSeedKind,
    limit: usize,
) -> Result<SymbolBodyResult> {
    let started = Instant::now();
    let phase_started = Instant::now();
    let (seed, candidates, _) = collect_candidates(engine, seed, seed_kind, limit)?;
    let candidate_collection_ms = elapsed_ms(phase_started);
    let (selected_candidates, ambiguity_status) =
        select_symbol_body_candidates(seed_kind, candidates);
    let unsupported_sources = collect_unsupported_sources(&selected_candidates);
    let mut source_read_ms = 0_u64;
    let mut chunk_excerpt_ms = 0_u64;
    let mut items = Vec::new();
    for candidate in &selected_candidates {
        if let Some(item) = extract_body_for_candidate_with_timings(
            engine,
            candidate,
            &mut source_read_ms,
            &mut chunk_excerpt_ms,
        )? {
            items.push(item);
            if items.len() >= limit.max(1) {
                break;
            }
        }
    }
    let capability_status =
        capability_status(items.len(), selected_candidates.len(), &unsupported_sources);
    let confidence = if items.is_empty() {
        0.0
    } else {
        items.iter().map(|item| item.confidence).sum::<f32>() / items.len() as f32
    };
    Ok(SymbolBodyResult {
        seed,
        items,
        capability_status,
        unsupported_sources,
        ambiguity_status,
        confidence,
        timings: SymbolBodyTimings {
            candidate_collection_ms,
            source_read_ms,
            chunk_excerpt_ms,
            total_ms: elapsed_ms(started),
        },
    })
}

fn select_symbol_body_candidates(
    seed_kind: ConceptSeedKind,
    candidates: Vec<CandidateFile>,
) -> (Vec<CandidateFile>, SymbolBodyAmbiguityStatus) {
    if !matches!(seed_kind, ConceptSeedKind::Symbol) {
        return (candidates, SymbolBodyAmbiguityStatus::None);
    }

    let exact_matches = candidates
        .iter()
        .filter(|candidate| candidate.match_kind == CandidateMatchKind::ExactSymbol)
        .cloned()
        .collect::<Vec<_>>();
    if !exact_matches.is_empty() {
        let ambiguity_status = if exact_matches.len() > 1 {
            SymbolBodyAmbiguityStatus::MultipleExact
        } else {
            SymbolBodyAmbiguityStatus::None
        };
        return (exact_matches, ambiguity_status);
    }

    let partial_matches = candidates
        .iter()
        .filter(|candidate| candidate.match_kind == CandidateMatchKind::PartialSymbol)
        .cloned()
        .collect::<Vec<_>>();
    if !partial_matches.is_empty() {
        return (partial_matches, SymbolBodyAmbiguityStatus::PartialOnly);
    }

    (candidates, SymbolBodyAmbiguityStatus::None)
}

fn collect_unsupported_sources(candidates: &[CandidateFile]) -> Vec<String> {
    candidates
        .iter()
        .filter(|candidate| !is_supported_language(&candidate.language, &candidate.path))
        .map(|candidate| format!("{}:{}", candidate.language, candidate.path))
        .collect()
}

pub(super) fn extract_body_for_candidate(
    engine: &Engine,
    candidate: &CandidateFile,
) -> Result<Option<SymbolBodyItem>> {
    let mut source_read_ms = 0_u64;
    let mut chunk_excerpt_ms = 0_u64;
    extract_body_for_candidate_with_timings(
        engine,
        candidate,
        &mut source_read_ms,
        &mut chunk_excerpt_ms,
    )
}

fn extract_body_for_candidate_with_timings(
    engine: &Engine,
    candidate: &CandidateFile,
    source_read_ms: &mut u64,
    chunk_excerpt_ms: &mut u64,
) -> Result<Option<SymbolBodyItem>> {
    if !is_supported_language(&candidate.language, &candidate.path) {
        return Ok(None);
    }

    let phase_started = Instant::now();
    let source = read_source(&engine.project_root, &candidate.path)?;
    add_elapsed_ms(source_read_ms, phase_started);
    let lines = source.lines().map(str::to_string).collect::<Vec<_>>();
    if lines.is_empty() {
        return Ok(None);
    }

    let extracted = try_exact_symbol_span(candidate, &lines)
        .map(|fields| (fields, SymbolBodyResolutionKind::ExactSymbolSpan))
        .or_else(|| {
            try_nearest_indexed_lines(candidate, &lines)
                .map(|fields| (fields, SymbolBodyResolutionKind::NearestIndexedLines))
        })
        .or_else(|| {
            let phase_started = Instant::now();
            let excerpt = extract_chunk_excerpt(engine, candidate).ok().flatten();
            add_elapsed_ms(chunk_excerpt_ms, phase_started);
            excerpt.map(|fields| (fields, SymbolBodyResolutionKind::ChunkExcerptAnchor))
        });

    Ok(extracted.map(
        |((signature, body, span, truncated), resolution_kind)| SymbolBodyItem {
            anchor: build_anchor(candidate),
            signature,
            body,
            span,
            source_kind: candidate.source_kind.clone(),
            resolution_kind,
            truncated,
            confidence: candidate.score.clamp(0.2, 1.0),
        },
    ))
}

fn try_exact_symbol_span(
    candidate: &CandidateFile,
    lines: &[String],
) -> Option<(String, String, SourceSpan, bool)> {
    candidate.line?;
    let language = detect_language(&candidate.path, &candidate.language);
    match language.as_str() {
        "rust" => extract_rust_block(candidate, lines),
        "python" => extract_python_block(candidate, lines),
        "typescript" | "javascript" => extract_js_ts_block(candidate, lines),
        _ => None,
    }
}

fn try_nearest_indexed_lines(
    candidate: &CandidateFile,
    lines: &[String],
) -> Option<(String, String, SourceSpan, bool)> {
    let anchor_line = candidate.line?;
    let anchor = anchor_line
        .saturating_sub(1)
        .min(lines.len().saturating_sub(1));
    let language = detect_language(&candidate.path, &candidate.language);
    let (before, after) = match language.as_str() {
        "sql" | "prisma" => (2, 6),
        _ => (2, 8),
    };
    let start = anchor.saturating_sub(before);
    let end = (anchor + after).min(lines.len().saturating_sub(1));
    build_body(lines, start, end)
}

fn extract_chunk_excerpt(
    engine: &Engine,
    candidate: &CandidateFile,
) -> Result<Option<(String, String, SourceSpan, bool)>> {
    let conn = engine.open_db_read_only()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT start_line, end_line, excerpt
        FROM file_chunks
        WHERE path = ?1
        ORDER BY
            CASE
                WHEN ?2 IS NULL THEN chunk_idx
                WHEN start_line <= ?2 AND end_line >= ?2 THEN 0
                ELSE ABS(start_line - ?2)
            END ASC,
            chunk_idx ASC
        LIMIT 1
        "#,
    )?;
    let anchor_line = candidate
        .line
        .map(|line| i64::try_from(line).unwrap_or(i64::MAX));
    for lookup_path in index_lookup_paths(&candidate.path) {
        let row = stmt
            .query_row(params![&lookup_path, anchor_line], |row| {
                let start_line = row.get::<_, Option<i64>>(0)?.unwrap_or(1);
                let end_line = row.get::<_, Option<i64>>(1)?.unwrap_or(start_line);
                let excerpt = row.get::<_, String>(2)?;
                Ok((start_line, end_line, excerpt))
            })
            .optional()?;
        if let Some((start_line, end_line, excerpt)) = row {
            return Ok(excerpt_to_body(
                &excerpt,
                usize::try_from(start_line).unwrap_or(1),
                usize::try_from(end_line).unwrap_or(usize::try_from(start_line).unwrap_or(1)),
            ));
        }
    }

    if candidate.line.is_none() {
        let source = read_source(&engine.project_root, &candidate.path)?;
        let head = source
            .lines()
            .take(12)
            .map(str::to_string)
            .collect::<Vec<_>>();
        if !head.is_empty() {
            return Ok(build_body(&head, 0, head.len() - 1));
        }
    }

    Ok(None)
}

fn extract_rust_block(
    candidate: &CandidateFile,
    lines: &[String],
) -> Option<(String, String, SourceSpan, bool)> {
    let anchor = candidate
        .line
        .unwrap_or(1)
        .saturating_sub(1)
        .min(lines.len() - 1);
    let start = (0..=anchor).rev().find(|index| {
        let line = lines[*index].trim_start();
        line.starts_with("fn ")
            || line.starts_with("pub fn ")
            || line.starts_with("async fn ")
            || line.starts_with("pub async fn ")
            || line.starts_with("struct ")
            || line.starts_with("pub struct ")
            || line.starts_with("enum ")
            || line.starts_with("pub enum ")
            || line.starts_with("impl ")
            || line.starts_with("trait ")
            || line.starts_with("pub trait ")
    })?;
    let mut brace_balance = 0_i32;
    let mut seen_open = false;
    let mut end = start;
    for (index, line) in lines.iter().enumerate().skip(start) {
        brace_balance += line.matches('{').count() as i32;
        if line.contains('{') {
            seen_open = true;
        }
        brace_balance -= line.matches('}').count() as i32;
        end = index;
        if seen_open && brace_balance <= 0 {
            break;
        }
        if !seen_open && index >= start + 12 {
            break;
        }
    }
    build_body(lines, start, end)
}

fn extract_python_block(
    candidate: &CandidateFile,
    lines: &[String],
) -> Option<(String, String, SourceSpan, bool)> {
    let anchor = candidate
        .line
        .unwrap_or(1)
        .saturating_sub(1)
        .min(lines.len() - 1);
    let mut start = (0..=anchor).rev().find(|index| {
        let line = lines[*index].trim_start();
        line.starts_with("def ") || line.starts_with("async def ") || line.starts_with("class ")
    })?;
    while start > 0 && lines[start - 1].trim_start().starts_with('@') {
        start -= 1;
    }
    let header_end = python_header_end(lines, start)?;
    let base_indent = indentation(lines[start].as_str());
    let mut end = lines.len() - 1;
    for (index, raw) in lines.iter().enumerate().skip(header_end + 1) {
        let raw = raw.as_str();
        if raw.trim().is_empty() {
            continue;
        }
        if indentation(raw) <= base_indent && !raw.trim_start().starts_with('#') {
            end = index.saturating_sub(1);
            break;
        }
    }
    build_body(lines, start, end)
}

fn python_header_end(lines: &[String], start: usize) -> Option<usize> {
    let mut paren_balance = 0_i32;
    for (index, raw) in lines.iter().enumerate().skip(start).take(16) {
        let line = raw.as_str();
        paren_balance += line.matches('(').count() as i32;
        paren_balance -= line.matches(')').count() as i32;
        if paren_balance <= 0 && line.trim_end().ends_with(':') {
            return Some(index);
        }
    }
    None
}

fn extract_js_ts_block(
    candidate: &CandidateFile,
    lines: &[String],
) -> Option<(String, String, SourceSpan, bool)> {
    let anchor = candidate
        .line
        .unwrap_or(1)
        .saturating_sub(1)
        .min(lines.len() - 1);
    let start = (0..=anchor)
        .rev()
        .find(|index| looks_like_js_ts_declaration(lines[*index].trim_start()))?;
    let mut brace_balance = 0_i32;
    let mut seen_open = false;
    let mut end = start;
    for (index, line) in lines.iter().enumerate().skip(start) {
        brace_balance += line.matches('{').count() as i32;
        if line.contains('{') {
            seen_open = true;
        }
        brace_balance -= line.matches('}').count() as i32;
        end = index;
        if seen_open && brace_balance <= 0 {
            break;
        }
        if !seen_open && index >= start + 12 {
            break;
        }
    }
    build_body(lines, start, end)
}

fn build_body(
    lines: &[String],
    start: usize,
    end: usize,
) -> Option<(String, String, SourceSpan, bool)> {
    if start > end || end >= lines.len() {
        return None;
    }
    let excerpt = lines[start..=end].join("\n");
    excerpt_to_body(&excerpt, start + 1, end + 1)
}

fn excerpt_to_body(
    excerpt: &str,
    start_line: usize,
    end_line: usize,
) -> Option<(String, String, SourceSpan, bool)> {
    let trimmed_excerpt = excerpt.trim_end();
    if trimmed_excerpt.is_empty() {
        return None;
    }
    let truncated = trimmed_excerpt.len() > 2000;
    let body = if truncated {
        trimmed_excerpt.chars().take(2000).collect::<String>()
    } else {
        trimmed_excerpt.to_string()
    };
    let signature = body
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        })
        .unwrap_or_default();
    Some((
        signature,
        body,
        SourceSpan {
            start_line,
            end_line: end_line.max(start_line),
            start_column: Some(1),
            end_column: None,
        },
        truncated,
    ))
}

fn indentation(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

fn looks_like_js_ts_declaration(line: &str) -> bool {
    if line.starts_with("function ")
        || line.starts_with("export function ")
        || line.starts_with("export default function ")
        || line.starts_with("async function ")
        || line.starts_with("export async function ")
        || line.starts_with("class ")
        || line.starts_with("export class ")
        || line.starts_with("export default class ")
        || line.starts_with("const ")
        || line.starts_with("let ")
        || line.starts_with("var ")
    {
        return line.contains("=>") || line.contains('{') || line.starts_with("class ");
    }

    if line.contains('(')
        && line.contains(')')
        && line.contains('{')
        && !line.starts_with("if ")
        && !line.starts_with("for ")
        && !line.starts_with("while ")
        && !line.starts_with("switch ")
        && !line.starts_with("catch ")
    {
        return true;
    }

    false
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

fn add_elapsed_ms(accumulator: &mut u64, started: Instant) {
    *accumulator = accumulator.saturating_add(elapsed_ms(started));
}
