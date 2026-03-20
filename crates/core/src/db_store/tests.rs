use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rusqlite::{Connection, params};
use time::OffsetDateTime;

mod project_key;
mod sidecars;
mod stale_cleanup;

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

fn seed_db(path: &std::path::Path, last_access: OffsetDateTime) -> Result<()> {
    let parent = path.parent().expect("db path parent");
    fs::create_dir_all(parent)?;
    let conn = Connection::open(path)?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;
    let ts = last_access.format(&time::format_description::well_known::Rfc3339)?;
    conn.execute(
        "INSERT INTO meta(key, value) VALUES('last_access_utc', ?1)",
        params![ts],
    )?;
    Ok(())
}
