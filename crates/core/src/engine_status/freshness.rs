use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rusqlite::Connection;

use crate::model::IndexFreshnessStatus;

const SAMPLE_LIMIT: usize = 5;

pub(super) fn scan(conn: &Connection, project_root: &Path) -> Result<IndexFreshnessStatus> {
    let mut status = IndexFreshnessStatus::default();
    let mut stmt = conn.prepare("SELECT path, source_mtime_unix_ms FROM files ORDER BY path")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
    })?;

    for row in rows {
        let (path, indexed_mtime) = row?;
        status.checked_files += 1;
        let absolute_path = project_root.join(&path);
        let Ok(metadata) = fs::metadata(&absolute_path) else {
            status.missing_files += 1;
            push_sample(&mut status.sample_paths, path);
            continue;
        };
        let Some(indexed_mtime) = indexed_mtime else {
            continue;
        };
        let current_mtime = metadata.modified().ok().map(system_time_to_unix_ms);
        if current_mtime.is_some_and(|current| current > indexed_mtime) {
            status.stale_files += 1;
            push_sample(&mut status.sample_paths, path);
        }
    }

    if status.stale_files > 0 || status.missing_files > 0 {
        status.hint = Some("refresh the index before relying on retrieval results".to_string());
    }
    Ok(status)
}

fn push_sample(sample_paths: &mut Vec<String>, path: String) {
    if sample_paths.len() < SAMPLE_LIMIT {
        sample_paths.push(path);
    }
}

fn system_time_to_unix_ms(time: SystemTime) -> i64 {
    time.duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}
