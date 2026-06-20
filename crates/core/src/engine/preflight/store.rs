use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use super::super::schema::OPEN_DB_READ_ONLY_PRAGMAS_SQL;

pub(super) fn open_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open db {}", path.display()))?;
    conn.execute_batch(OPEN_DB_READ_ONLY_PRAGMAS_SQL)
        .context("failed to apply sqlite pragmas")?;
    Ok(conn)
}

pub(super) fn read_meta_u32(conn: &Connection, key: &str) -> Result<Option<u32>> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    })
    .optional()?
    .map(|raw| {
        raw.parse::<u32>()
            .with_context(|| format!("meta key `{key}` has non-u32 value `{raw}`"))
    })
    .transpose()
}
