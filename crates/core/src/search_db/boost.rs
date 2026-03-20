use std::collections::BTreeSet;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

use super::helpers::escape_like_value;
use crate::query_profile::{QueryProfile, graph_boost_scale};

pub(crate) fn graph_boost(
    conn: &Connection,
    path: &str,
    tokens: &[String],
    profile: QueryProfile,
) -> Result<f32> {
    if tokens.is_empty() {
        return Ok(0.0);
    }

    let path_lc = path.to_lowercase();
    let mut total_boost = 0.0_f32;

    for token in tokens {
        let like = format!("%{}%", escape_like_value(token));

        let symbols_hits = query_count(
            conn,
            "SELECT COUNT(1) FROM symbols WHERE path = ?1 AND name LIKE ?2 ESCAPE '\\'",
            path,
            &like,
            "symbols",
        )?;
        let refs_hits = query_count(
            conn,
            "SELECT COUNT(1) FROM refs WHERE path = ?1 AND symbol LIKE ?2 ESCAPE '\\'",
            path,
            &like,
            "refs",
        )?;
        let deps_hits = query_count(
            conn,
            "SELECT COUNT(1) FROM module_deps WHERE path = ?1 AND dep LIKE ?2 ESCAPE '\\'",
            path,
            &like,
            "module_deps",
        )?;

        if symbols_hits > 0 {
            total_boost += 0.12 + (symbols_hits.min(3) as f32 * 0.08);
        }
        if refs_hits > 0 {
            total_boost += 0.08 + (refs_hits.min(3) as f32 * 0.05);
        }
        if deps_hits > 0 {
            total_boost += 0.05 + (deps_hits.min(3) as f32 * 0.04);
        }
        if path_lc.contains(token) {
            total_boost += 0.07;
        }
    }

    Ok(total_boost.min(1.5) * graph_boost_scale(profile))
}

fn query_count(
    conn: &Connection,
    sql: &str,
    path: &str,
    like: &str,
    table_name: &str,
) -> Result<i64> {
    conn.query_row(sql, params![path, like], |row| row.get::<_, i64>(0))
        .with_context(|| format!("failed to query {table_name} boost for path={path}"))
}

pub(crate) fn extract_tokens(query: &str) -> Vec<String> {
    let mut set = BTreeSet::new();
    for token in query
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .map(|s| s.trim().to_lowercase())
    {
        if token.chars().count() >= 2 {
            set.insert(token);
        }
    }
    set.into_iter().collect()
}
