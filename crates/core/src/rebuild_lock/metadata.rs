use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::Ordering;

use anyhow::{Context, Result};
use time::OffsetDateTime;

use super::LOCK_NONCE;

#[derive(Debug, Clone)]
pub(super) struct LockSnapshot {
    pub(super) lock_age_secs: u64,
    pub(super) raw: String,
    pub(super) metadata: LockMetadata,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LockMetadata {
    pub(super) pid: Option<u32>,
    pub(super) started_at_utc: Option<OffsetDateTime>,
}

pub(super) fn write_lock_payload(lock_file: &mut File, lock_path: &Path) -> Result<String> {
    let lock_token = format!(
        "{}-{}",
        std::process::id(),
        LOCK_NONCE.fetch_add(1, Ordering::Relaxed)
    );
    let payload = format!(
        "pid={}\nlock_token={}\nstarted_at_utc={}\n",
        std::process::id(),
        lock_token,
        OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?
    );
    lock_file
        .write_all(payload.as_bytes())
        .with_context(|| format!("failed to write lock {}", lock_path.display()))?;
    Ok(lock_token)
}

pub(super) fn lock_belongs_to_token(lock_path: &Path, token: &str) -> bool {
    let Ok(raw) = fs::read_to_string(lock_path) else {
        return false;
    };
    raw.lines()
        .find_map(|line| line.strip_prefix("lock_token="))
        .is_some_and(|value| value.trim() == token)
}

#[cfg(test)]
pub(super) fn parse_lock_metadata(raw: &str) -> LockMetadata {
    let pid = raw
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .and_then(|value| value.trim().parse::<u32>().ok());
    let started_at_utc = raw
        .lines()
        .find_map(|line| line.strip_prefix("started_at_utc="))
        .and_then(|value| {
            OffsetDateTime::parse(value.trim(), &time::format_description::well_known::Rfc3339).ok()
        });
    LockMetadata {
        pid,
        started_at_utc,
    }
}
