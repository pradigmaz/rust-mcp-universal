use std::collections::HashMap;

use anyhow::Result;

use super::super::types::RunStats;
use crate::engine::storage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IoFailurePolicy {
    AuthoritativeRemoval,
    PreservePriorSnapshot,
}

pub(super) fn classify_io_failure(err: &std::io::Error) -> IoFailurePolicy {
    match err.kind() {
        std::io::ErrorKind::NotFound => IoFailurePolicy::AuthoritativeRemoval,
        _ => IoFailurePolicy::PreservePriorSnapshot,
    }
}

pub(super) fn mark_authoritative_removed_path_as_skipped(
    tx: &rusqlite::Transaction<'_>,
    existing_files: &HashMap<String, storage::ExistingFileState>,
    rel_text: &str,
    stats: &mut RunStats,
) -> Result<()> {
    if existing_files.contains_key(rel_text) {
        storage::remove_path_index(tx, rel_text)?;
        stats.changed += 1;
    }
    stats.skipped += 1;
    Ok(())
}

pub(super) fn mark_preserved_path_on_failure(stats: &mut RunStats) {
    stats.skipped += 1;
}

pub(super) fn mark_unchanged(stats: &mut RunStats) {
    stats.unchanged += 1;
}

pub(super) fn mark_skipped_before_changed_since(stats: &mut RunStats) {
    stats.skipped_before_changed_since += 1;
}

pub(super) fn mark_embedding_cache_hit(stats: &mut RunStats) {
    stats.embedding_cache_hits += 1;
}

pub(super) fn mark_embedding_cache_miss(stats: &mut RunStats) {
    stats.embedding_cache_misses += 1;
}

pub(super) fn mark_indexed(stats: &mut RunStats, existed_before: bool) {
    if existed_before {
        stats.changed += 1;
    } else {
        stats.added += 1;
    }
    stats.indexed += 1;
}

#[cfg(test)]
mod tests {
    use super::{IoFailurePolicy, classify_io_failure};

    #[test]
    fn io_failure_classifies_not_found_as_authoritative_removal() {
        let err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        assert_eq!(
            classify_io_failure(&err),
            IoFailurePolicy::AuthoritativeRemoval
        );
    }

    #[test]
    fn io_failure_classifies_permission_denied_as_preserve_snapshot() {
        let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        assert_eq!(
            classify_io_failure(&err),
            IoFailurePolicy::PreservePriorSnapshot
        );
    }

    #[test]
    fn io_failure_classifies_other_errors_as_preserve_snapshot() {
        let err = std::io::Error::other("transient");
        assert_eq!(
            classify_io_failure(&err),
            IoFailurePolicy::PreservePriorSnapshot
        );
    }
}
