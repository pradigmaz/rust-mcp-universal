use anyhow::{Result, bail};
use rusqlite::Connection;

use super::super::Engine;
use super::common::file_exists;
use super::validation::require_non_empty;
use crate::model::{CallPathEndpoint, SymbolMatch};
use crate::text_utils::i64_to_option_usize;

impl Engine {
    pub(super) fn resolve_call_path_endpoint(
        &self,
        conn: &Connection,
        raw: &str,
    ) -> Result<CallPathEndpoint> {
        let input = require_non_empty(raw, "endpoint")?.to_string();
        if let Ok(path) = self.normalize_lookup_path(&input) {
            if file_exists(conn, &path)? {
                return Ok(CallPathEndpoint {
                    input,
                    resolved_path: path,
                    kind: "path".to_string(),
                    symbol: None,
                    line: None,
                    column: None,
                });
            }
        }

        let rows = exact_symbol_matches(conn, &input)?;
        let mut unique_paths = rows.iter().map(|row| row.path.clone()).collect::<Vec<_>>();
        unique_paths.sort();
        unique_paths.dedup();

        match rows.first() {
            Some(first) if unique_paths.len() == 1 => Ok(CallPathEndpoint {
                input,
                resolved_path: first.path.clone(),
                kind: "symbol".to_string(),
                symbol: Some(first.name.clone()),
                line: first.line,
                column: first.column,
            }),
            Some(_) => bail!(
                "symbol endpoint `{}` is ambiguous across {} files; use a path instead",
                raw.trim(),
                unique_paths.len()
            ),
            None => bail!(
                "unable to resolve endpoint `{}` as indexed path or exact symbol",
                raw.trim()
            ),
        }
    }
}

fn exact_symbol_matches(conn: &Connection, input: &str) -> Result<Vec<SymbolMatch>> {
    let mut stmt = conn.prepare(
        "SELECT path, name, line, column
         FROM symbols
         WHERE name = ?1
         ORDER BY path ASC,
                  COALESCE(line, 2147483647) ASC,
                  COALESCE(column, 2147483647) ASC",
    )?;
    let rows = stmt
        .query_map([input], |row| {
            Ok(SymbolMatch {
                path: row.get(0)?,
                name: row.get(1)?,
                kind: "symbol".to_string(),
                language: String::new(),
                line: row.get::<_, Option<i64>>(2)?.and_then(i64_to_option_usize),
                column: row.get::<_, Option<i64>>(3)?.and_then(i64_to_option_usize),
                exact: true,
                reason_codes: vec!["symbol_table".to_string()],
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
