use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

pub(super) fn count_rows(conn: &Connection, table: &str) -> Result<usize> {
    let sql = format!("SELECT COUNT(1) FROM {table}");
    let count: i64 = conn.query_row(&sql, [], |row| row.get(0))?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

pub(super) fn meta_u64(conn: &Connection, key: &str) -> Result<u64> {
    let value = conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get::<_, String>(0)
        })
        .optional()
        .with_context(|| format!("failed to read meta key `{key}`"))?;

    match value {
        Some(raw) => raw
            .parse::<u64>()
            .with_context(|| format!("meta key `{key}` contains non-u64 value `{raw}`")),
        None => Ok(0),
    }
}
