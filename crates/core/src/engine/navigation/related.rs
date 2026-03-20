use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use anyhow::Result;
use rusqlite::Connection;

use super::super::Engine;
use super::common::{ensure_file_exists, load_string_set};
use crate::model::RelatedFileHit;
use crate::text_utils::symbol_tail;

#[derive(Debug, Default)]
struct RelatedAccumulator {
    language: String,
    dep_overlap: usize,
    incoming_ref_overlap: usize,
    outgoing_ref_overlap: usize,
    symbol_overlap: usize,
}

impl Engine {
    pub fn related_files(&self, path: &str, limit: usize) -> Result<Vec<RelatedFileHit>> {
        let normalized_path = self.normalize_lookup_path(path)?;
        let limit = limit.max(1);
        let conn = self.open_db()?;
        ensure_file_exists(&conn, &normalized_path)?;

        let base_deps = load_string_set(
            &conn,
            "SELECT dep FROM module_deps WHERE path = ?1",
            &normalized_path,
            "base deps",
        )?;
        let base_symbols = load_string_set(
            &conn,
            "SELECT name FROM symbols WHERE path = ?1",
            &normalized_path,
            "base symbols",
        )?;
        let base_ref_tails = load_string_set(
            &conn,
            "SELECT symbol FROM refs WHERE path = ?1",
            &normalized_path,
            "base refs",
        )?
        .into_iter()
        .map(|symbol| symbol_tail(&symbol).to_string())
        .collect::<HashSet<_>>();
        let base_dep_tails = base_deps
            .iter()
            .map(|dep| symbol_tail(dep).to_string())
            .collect::<HashSet<_>>();
        let base_outgoing_symbol_hints = base_ref_tails
            .union(&base_dep_tails)
            .cloned()
            .collect::<HashSet<_>>();

        let mut by_path = HashMap::<String, RelatedAccumulator>::new();
        accumulate_dep_overlaps(&conn, &normalized_path, &base_deps, &mut by_path)?;
        accumulate_symbol_overlaps(
            &conn,
            &normalized_path,
            &base_symbols,
            &base_outgoing_symbol_hints,
            &mut by_path,
        )?;
        accumulate_ref_overlaps(&conn, &normalized_path, &base_symbols, &mut by_path)?;

        let mut hits = by_path
            .into_iter()
            .filter_map(|(candidate_path, acc)| {
                let ref_overlap = acc
                    .incoming_ref_overlap
                    .saturating_add(acc.outgoing_ref_overlap);
                if acc.dep_overlap == 0 && ref_overlap == 0 && acc.symbol_overlap == 0 {
                    return None;
                }

                let score = acc.dep_overlap as f32 * 0.45
                    + ref_overlap as f32 * 1.2
                    + acc.symbol_overlap as f32 * 0.35;

                Some(RelatedFileHit {
                    path: candidate_path,
                    language: acc.language,
                    score,
                    dep_overlap: acc.dep_overlap,
                    ref_overlap,
                    symbol_overlap: acc.symbol_overlap,
                })
            })
            .collect::<Vec<_>>();

        hits.sort_by(compare_related_hits);
        hits.truncate(limit);
        Ok(hits)
    }
}

fn accumulate_dep_overlaps(
    conn: &Connection,
    base_path: &str,
    base_deps: &HashSet<String>,
    by_path: &mut HashMap<String, RelatedAccumulator>,
) -> Result<()> {
    if base_deps.is_empty() {
        return Ok(());
    }

    let mut stmt = conn.prepare(
        "SELECT path, dep, language FROM module_deps WHERE path <> ?1 ORDER BY path ASC",
    )?;
    let rows = stmt.query_map([base_path], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    for row in rows {
        let (path, dep, language) = row?;
        if !base_deps.contains(&dep) {
            continue;
        }
        let entry = by_path.entry(path).or_default();
        if entry.language.is_empty() {
            entry.language = language;
        }
        entry.dep_overlap = entry.dep_overlap.saturating_add(1);
    }

    Ok(())
}

fn accumulate_symbol_overlaps(
    conn: &Connection,
    base_path: &str,
    base_symbols: &HashSet<String>,
    base_outgoing_symbol_hints: &HashSet<String>,
    by_path: &mut HashMap<String, RelatedAccumulator>,
) -> Result<()> {
    if base_symbols.is_empty() && base_outgoing_symbol_hints.is_empty() {
        return Ok(());
    }

    let mut stmt = conn
        .prepare("SELECT path, name, language FROM symbols WHERE path <> ?1 ORDER BY path ASC")?;
    let rows = stmt.query_map([base_path], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    for row in rows {
        let (path, symbol_name, language) = row?;
        let shared_symbol = base_symbols.contains(&symbol_name);
        let called_from_base = base_outgoing_symbol_hints.contains(&symbol_name);
        if !shared_symbol && !called_from_base {
            continue;
        }
        let entry = by_path.entry(path).or_default();
        if entry.language.is_empty() {
            entry.language = language;
        }
        if shared_symbol {
            entry.symbol_overlap = entry.symbol_overlap.saturating_add(1);
        }
        if called_from_base {
            entry.outgoing_ref_overlap = entry.outgoing_ref_overlap.saturating_add(1);
        }
    }

    Ok(())
}

fn accumulate_ref_overlaps(
    conn: &Connection,
    base_path: &str,
    base_symbols: &HashSet<String>,
    by_path: &mut HashMap<String, RelatedAccumulator>,
) -> Result<()> {
    if base_symbols.is_empty() {
        return Ok(());
    }

    let mut stmt =
        conn.prepare("SELECT path, symbol, language FROM refs WHERE path <> ?1 ORDER BY path ASC")?;
    let rows = stmt.query_map([base_path], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    for row in rows {
        let (path, symbol, language) = row?;
        let tail = symbol_tail(&symbol);
        if !base_symbols.contains(&symbol) && !base_symbols.contains(tail) {
            continue;
        }
        let entry = by_path.entry(path).or_default();
        if entry.language.is_empty() {
            entry.language = language;
        }
        entry.incoming_ref_overlap = entry.incoming_ref_overlap.saturating_add(1);
    }

    Ok(())
}

fn compare_related_hits(left: &RelatedFileHit, right: &RelatedFileHit) -> Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| right.ref_overlap.cmp(&left.ref_overlap))
        .then_with(|| right.dep_overlap.cmp(&left.dep_overlap))
        .then_with(|| right.symbol_overlap.cmp(&left.symbol_overlap))
        .then_with(|| left.path.cmp(&right.path))
}
