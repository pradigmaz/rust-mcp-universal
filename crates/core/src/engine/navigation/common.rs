use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use rusqlite::Connection;

pub(super) fn require_non_empty<'a>(value: &'a str, field: &str) -> Result<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("`{field}` must be non-empty");
    }
    Ok(trimmed)
}

pub(super) fn db_limit(limit: usize, field: &str) -> Result<i64> {
    if limit == 0 {
        bail!("`{field}` must be >= 1");
    }
    i64::try_from(limit).with_context(|| {
        format!(
            "`{field}` value {limit} exceeds maximum supported value {}",
            i64::MAX
        )
    })
}

pub(super) fn ensure_file_exists(conn: &Connection, path: &str) -> Result<()> {
    if file_exists(conn, path)? {
        return Ok(());
    }
    bail!("indexed file not found for path `{path}`");
}

pub(super) fn file_exists(conn: &Connection, path: &str) -> Result<bool> {
    let exists = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM files WHERE path = ?1)",
        [path],
        |row| row.get::<_, i64>(0),
    )?;
    Ok(exists > 0)
}

pub(super) fn load_string_set(
    conn: &Connection,
    sql: &str,
    path: &str,
    label: &str,
) -> Result<HashSet<String>> {
    let mut stmt = conn
        .prepare(sql)
        .with_context(|| format!("failed to prepare query for {label}"))?;
    let rows = stmt
        .query_map([path], |row| row.get::<_, String>(0))
        .with_context(|| format!("failed to query {label} for path={path}"))?
        .collect::<rusqlite::Result<Vec<_>>>()
        .with_context(|| format!("failed to collect {label} for path={path}"))?;
    Ok(rows.into_iter().collect())
}
