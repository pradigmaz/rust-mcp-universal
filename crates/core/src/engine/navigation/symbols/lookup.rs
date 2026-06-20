use anyhow::Result;
use rusqlite::params;

use crate::engine::Engine;
use crate::model::SymbolMatch;
use crate::text_utils::{escape_like_value, i64_to_option_usize};

impl Engine {
    pub fn symbol_lookup(&self, name: &str, limit: usize) -> Result<Vec<SymbolMatch>> {
        let query = super::super::validation::require_non_empty(name, "name")?;
        let db_limit = super::super::validation::db_limit(limit, "limit")?;
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
                let exact = row.get::<_, i64>(6)? > 0;
                Ok(SymbolMatch {
                    path: row.get(0)?,
                    name: row.get(1)?,
                    kind: row.get(2)?,
                    language: row.get(3)?,
                    line: row.get::<_, Option<i64>>(4)?.and_then(i64_to_option_usize),
                    column: row.get::<_, Option<i64>>(5)?.and_then(i64_to_option_usize),
                    exact,
                    reason_codes: symbol_lookup_reason_codes(exact),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(rows)
    }
}

fn symbol_lookup_reason_codes(exact: bool) -> Vec<String> {
    let mut codes = vec!["symbol_table".to_string()];
    if !exact {
        codes.push("lexical_fallback".to_string());
    }
    codes
}
