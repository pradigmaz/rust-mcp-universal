use anyhow::Result;

use crate::engine::storage::{self, UpsertQualitySnapshotInput};
use crate::quality::{
    CURRENT_QUALITY_RULESET_VERSION, evaluate_indexed_quality, evaluate_oversize_quality,
    violations_hash,
};

use super::source::{SourceMetadata, SourceSnapshot};

pub(super) fn persist_indexed_quality(
    tx: &rusqlite::Transaction<'_>,
    rel_text: &str,
    source: &SourceSnapshot,
    metadata: &SourceMetadata,
    indexed_at: &str,
) -> Result<()> {
    let snapshot = evaluate_indexed_quality(
        rel_text,
        &source.language,
        metadata.size_bytes,
        &source.full_text,
    );
    storage::upsert_quality_snapshot(
        tx,
        UpsertQualitySnapshotInput {
            path: rel_text,
            language: &source.language,
            size_bytes: snapshot.size_bytes,
            total_lines: snapshot.total_lines,
            non_empty_lines: snapshot.non_empty_lines,
            import_count: snapshot.import_count,
            quality_mode: snapshot.quality_mode,
            source_mtime_unix_ms: metadata.current_mtime_unix_ms,
            quality_ruleset_version: CURRENT_QUALITY_RULESET_VERSION,
            quality_violation_hash: &violations_hash(&snapshot.violations),
            quality_indexed_at_utc: indexed_at,
            violations: &snapshot.violations,
        },
    )
}

pub(super) fn persist_oversize_quality(
    tx: &rusqlite::Transaction<'_>,
    rel_text: &str,
    language: &str,
    metadata: &SourceMetadata,
    indexed_at: &str,
) -> Result<()> {
    let snapshot = evaluate_oversize_quality(metadata.size_bytes);
    storage::upsert_quality_snapshot(
        tx,
        UpsertQualitySnapshotInput {
            path: rel_text,
            language,
            size_bytes: snapshot.size_bytes,
            total_lines: snapshot.total_lines,
            non_empty_lines: snapshot.non_empty_lines,
            import_count: snapshot.import_count,
            quality_mode: snapshot.quality_mode,
            source_mtime_unix_ms: metadata.current_mtime_unix_ms,
            quality_ruleset_version: CURRENT_QUALITY_RULESET_VERSION,
            quality_violation_hash: &violations_hash(&snapshot.violations),
            quality_indexed_at_utc: indexed_at,
            violations: &snapshot.violations,
        },
    )
}
