use std::fs;
use std::path::Path;

use time::OffsetDateTime;

use super::process_probe::process_liveness;
use super::{
    LockMetadata, LockSnapshot, ProcessLiveness, REBUILD_LOCK_FORCE_RECLAIM_SECS,
    REBUILD_LOCK_ORPHAN_GRACE_SECS, REBUILD_LOCK_STALE_SECS,
};

pub(super) fn is_lock_stale(lock_path: &Path) -> bool {
    lock_file_age_secs(lock_path).is_some_and(|age_secs| age_secs > REBUILD_LOCK_STALE_SECS)
}

pub(super) fn try_reclaim_stale_lock(lock_path: &Path) -> bool {
    let Some(snapshot) = read_lock_snapshot(lock_path) else {
        return false;
    };
    if !can_reclaim_stale_lock(&snapshot) {
        return false;
    }
    if !lock_snapshot_is_unchanged(lock_path, &snapshot) {
        return false;
    }
    fs::remove_file(lock_path).is_ok()
}

fn can_reclaim_stale_lock(snapshot: &LockSnapshot) -> bool {
    let force_reclaim_age_secs = i64::try_from(REBUILD_LOCK_FORCE_RECLAIM_SECS).unwrap_or(i64::MAX);
    if snapshot.metadata.started_at_utc.is_some_and(|started_at| {
        (OffsetDateTime::now_utc() - started_at).whole_seconds() > force_reclaim_age_secs
    }) {
        return true;
    }

    // Legacy/partial payload without `started_at_utc` must remain reclaimable eventually.
    if snapshot.metadata.started_at_utc.is_none()
        && snapshot.lock_age_secs > REBUILD_LOCK_FORCE_RECLAIM_SECS
    {
        return true;
    }

    if snapshot.lock_age_secs < REBUILD_LOCK_ORPHAN_GRACE_SECS {
        return false;
    }

    match snapshot.metadata.pid {
        Some(pid) => matches!(process_liveness(pid), ProcessLiveness::Dead),
        None => true,
    }
}

fn lock_file_age_secs(lock_path: &Path) -> Option<u64> {
    fs::metadata(lock_path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|modified| modified.elapsed().ok())
        .map(|elapsed| elapsed.as_secs())
}

fn read_lock_snapshot(lock_path: &Path) -> Option<LockSnapshot> {
    let lock_age_secs = lock_file_age_secs(lock_path)?;
    let raw = fs::read_to_string(lock_path).ok()?;
    Some(LockSnapshot {
        lock_age_secs,
        metadata: parse_lock_metadata(&raw),
        raw,
    })
}

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

pub(super) fn lock_snapshot_is_unchanged(lock_path: &Path, snapshot: &LockSnapshot) -> bool {
    fs::read_to_string(lock_path)
        .ok()
        .is_some_and(|current| current == snapshot.raw)
}
