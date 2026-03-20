use std::cmp::Ordering;
use std::collections::HashMap;

use anyhow::Result;
use rusqlite::params;

use super::super::Engine;
use super::common::{db_limit, require_non_empty};
use crate::model::{SymbolMatch, SymbolReferenceHit};
use crate::text_utils::{escape_like_value, i64_to_option_usize, symbol_tail};

#[derive(Debug)]
struct RefMatchRow {
    path: String,
    language: String,
    line: Option<usize>,
    column: Option<usize>,
    match_rank: i64,
}

#[derive(Debug)]
struct SymbolReferenceHitAccumulator {
    path: String,
    language: String,
    ref_count: usize,
    line: Option<usize>,
    column: Option<usize>,
    match_rank: i64,
}

impl SymbolReferenceHitAccumulator {
    fn new(path: String, language: String) -> Self {
        Self {
            path,
            language,
            ref_count: 0,
            line: None,
            column: None,
            match_rank: 0,
        }
    }

    fn consider_position(
        &mut self,
        candidate_rank: i64,
        candidate_line: Option<usize>,
        candidate_column: Option<usize>,
    ) {
        match candidate_rank.cmp(&self.match_rank) {
            Ordering::Greater => {
                self.match_rank = candidate_rank;
                self.line = candidate_line;
                self.column = candidate_column;
            }
            Ordering::Equal => {
                if compare_position((candidate_line, candidate_column), (self.line, self.column))
                    .is_lt()
                {
                    self.line = candidate_line;
                    self.column = candidate_column;
                }
            }
            Ordering::Less => {}
        }
    }

    fn into_hit(self) -> SymbolReferenceHit {
        SymbolReferenceHit {
            path: self.path,
            language: self.language,
            ref_count: self.ref_count,
            line: self.line,
            column: self.column,
            exact: self.match_rank >= 3,
        }
    }
}

impl Engine {
    pub fn symbol_lookup(&self, name: &str, limit: usize) -> Result<Vec<SymbolMatch>> {
        let query = require_non_empty(name, "name")?;
        let db_limit = db_limit(limit, "limit")?;
        let like = format!("%{}%", escape_like_value(query));
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT DISTINCT
                path,
                name,
                kind,
                language,
                line,
                column,
                CASE WHEN name = ?1 THEN 1 ELSE 0 END AS exact_match
            FROM symbols
            WHERE name = ?1 OR name LIKE ?2 ESCAPE '\'
            ORDER BY exact_match DESC,
                     LENGTH(name) ASC,
                     name ASC,
                     path ASC,
                     COALESCE(line, 2147483647) ASC,
                     COALESCE(column, 2147483647) ASC
            LIMIT ?3
            "#,
        )?;

        let rows = stmt
            .query_map(params![query, like, db_limit], |row| {
                Ok(SymbolMatch {
                    path: row.get(0)?,
                    name: row.get(1)?,
                    kind: row.get(2)?,
                    language: row.get(3)?,
                    line: row.get::<_, Option<i64>>(4)?.and_then(i64_to_option_usize),
                    column: row.get::<_, Option<i64>>(5)?.and_then(i64_to_option_usize),
                    exact: row.get::<_, i64>(6)? > 0,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(rows)
    }

    pub fn symbol_references(&self, name: &str, limit: usize) -> Result<Vec<SymbolReferenceHit>> {
        let query = require_non_empty(name, "name")?;
        db_limit(limit, "limit")?;
        let contains = format!("%{}%", escape_like_value(query));
        let tail = symbol_tail(query);
        let suffix = format!("%::{}", escape_like_value(tail));
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(
            r#"
            SELECT
                path,
                language,
                line,
                column,
                CASE
                    WHEN symbol = ?1 THEN 3
                    WHEN symbol LIKE ?2 ESCAPE '\' THEN 2
                    WHEN symbol LIKE ?3 ESCAPE '\' THEN 1
                    ELSE 0
                END AS match_rank
            FROM refs
            WHERE symbol = ?1
               OR symbol LIKE ?2 ESCAPE '\'
               OR symbol LIKE ?3 ESCAPE '\'
            ORDER BY path ASC,
                     COALESCE(line, 2147483647) ASC,
                     COALESCE(column, 2147483647) ASC,
                     symbol ASC
            "#,
        )?;
        let rows = stmt
            .query_map(params![query, suffix, contains], |row| {
                Ok(RefMatchRow {
                    path: row.get(0)?,
                    language: row.get(1)?,
                    line: row.get::<_, Option<i64>>(2)?.and_then(i64_to_option_usize),
                    column: row.get::<_, Option<i64>>(3)?.and_then(i64_to_option_usize),
                    match_rank: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut grouped = HashMap::<String, SymbolReferenceHitAccumulator>::new();
        for row in rows {
            let entry = grouped.entry(row.path.clone()).or_insert_with(|| {
                SymbolReferenceHitAccumulator::new(row.path.clone(), row.language.clone())
            });
            entry.ref_count = entry.ref_count.saturating_add(1);
            entry.consider_position(row.match_rank, row.line, row.column);
            if entry.language.is_empty() {
                entry.language = row.language;
            }
        }

        let mut hits = grouped
            .into_values()
            .map(SymbolReferenceHitAccumulator::into_hit)
            .collect::<Vec<_>>();
        hits.sort_by(compare_symbol_reference_hits);
        hits.truncate(limit);
        Ok(hits)
    }
}

fn compare_symbol_reference_hits(
    left: &SymbolReferenceHit,
    right: &SymbolReferenceHit,
) -> Ordering {
    rank_key(right.exact)
        .cmp(&rank_key(left.exact))
        .then_with(|| right.ref_count.cmp(&left.ref_count))
        .then_with(|| compare_position((left.line, left.column), (right.line, right.column)))
        .then_with(|| left.path.cmp(&right.path))
}

fn rank_key(exact: bool) -> i32 {
    if exact { 1 } else { 0 }
}

fn compare_position(
    left: (Option<usize>, Option<usize>),
    right: (Option<usize>, Option<usize>),
) -> Ordering {
    match (left.0, right.0) {
        (Some(left_line), Some(right_line)) => left_line.cmp(&right_line).then_with(|| {
            left.1
                .unwrap_or(usize::MAX)
                .cmp(&right.1.unwrap_or(usize::MAX))
        }),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
