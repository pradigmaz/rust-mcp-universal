use anyhow::{Result, anyhow};
use rusqlite::{Connection, OptionalExtension, Transaction, params};

pub(super) fn count_files(conn: &Connection) -> Result<usize> {
    let count = conn.query_row("SELECT COUNT(1) FROM files", [], |row| row.get::<_, i64>(0))?;
    Ok(usize::try_from(count).unwrap_or(usize::MAX))
}

pub(super) fn read_meta_u32_lossy(
    conn: &Connection,
    key: &str,
    reasons: &mut Vec<String>,
) -> Result<Option<u32>> {
    let Some(raw) = read_meta_raw(conn, key)? else {
        return Ok(None);
    };
    match raw.parse::<u32>() {
        Ok(value) => Ok(Some(value)),
        Err(_) => {
            reasons.push(format!("meta key `{key}` has non-u32 value `{raw}`"));
            Ok(None)
        }
    }
}

pub(super) fn read_meta_u32(conn: &Connection, key: &str) -> Result<Option<u32>> {
    let Some(raw) = read_meta_raw(conn, key)? else {
        return Ok(None);
    };
    let parsed = raw
        .parse::<u32>()
        .map_err(|_| anyhow!("meta key `{key}` has non-u32 value `{raw}`"))?;
    Ok(Some(parsed))
}

pub(super) fn read_meta_raw(conn: &Connection, key: &str) -> Result<Option<String>> {
    if !meta_table_exists(conn)? {
        return Ok(None);
    }
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    })
    .optional()
    .map_err(Into::into)
}

pub(super) fn upsert_meta_conn(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub(super) fn upsert_meta_tx(tx: &Transaction<'_>, key: &str, value: &str) -> Result<()> {
    tx.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn meta_table_exists(conn: &Connection) -> Result<bool> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'meta' LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()?
        .is_some();
    Ok(exists)
}
