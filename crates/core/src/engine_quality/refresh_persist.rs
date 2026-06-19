use std::collections::HashSet;

use anyhow::Result;

use super::status::{
    write_quality_status_degraded, write_quality_status_ready, write_quality_status_unavailable,
};
use crate::engine::storage::{
    UpsertQualitySnapshotInput, remove_path_quality, upsert_quality_snapshot,
};

#[derive(Debug)]
pub(super) struct QualityRefreshRecord {
    pub(super) path: String,
    pub(super) language: String,
    pub(super) size_bytes: i64,
    pub(super) total_lines: Option<i64>,
    pub(super) non_empty_lines: Option<i64>,
    pub(super) import_count: Option<i64>,
    pub(super) quality_mode: crate::model::QualityMode,
    pub(super) source_mtime_unix_ms: Option<i64>,
    pub(super) quality_metric_hash: String,
    pub(super) quality_violation_hash: String,
    pub(super) quality_suppressed_violation_hash: String,
    pub(super) metrics: Vec<crate::quality::QualityMetricEntry>,
    pub(super) violations: Vec<crate::model::QualityViolationEntry>,
    pub(super) suppressed_violations: Vec<crate::model::SuppressedQualityViolationEntry>,
}

pub(super) fn persist_quality_refresh(
    conn: &rusqlite::Connection,
    deleted_paths: &HashSet<String>,
    records: &[QualityRefreshRecord],
    degraded: bool,
    last_error_rule_id: Option<&str>,
    policy_digest: &str,
) -> bool {
    let tx_result = match conn.unchecked_transaction() {
        Ok(tx) => {
            let result: Result<()> = (|| {
                for path in sorted_paths(deleted_paths) {
                    remove_path_quality(&tx, &path)?;
                }
                for record in records {
                    upsert_quality_snapshot(
                        &tx,
                        UpsertQualitySnapshotInput {
                            path: &record.path,
                            language: &record.language,
                            size_bytes: record.size_bytes,
                            total_lines: record.total_lines,
                            non_empty_lines: record.non_empty_lines,
                            import_count: record.import_count,
                            quality_mode: record.quality_mode,
                            source_mtime_unix_ms: record.source_mtime_unix_ms,
                            quality_ruleset_version:
                                crate::quality::CURRENT_QUALITY_RULESET_VERSION,
                            quality_metric_hash: &record.quality_metric_hash,
                            quality_violation_hash: &record.quality_violation_hash,
                            quality_suppressed_violation_hash: &record
                                .quality_suppressed_violation_hash,
                            quality_indexed_at_utc: &now_rfc3339()?,
                            metrics: &record.metrics,
                            violations: &record.violations,
                            suppressed_violations: &record.suppressed_violations,
                        },
                    )?;
                }
                if degraded {
                    write_quality_status_degraded(&tx, last_error_rule_id, policy_digest)?;
                } else {
                    write_quality_status_ready(&tx, policy_digest)?;
                }
                tx.commit()?;
                Ok(())
            })();
            result
        }
        Err(err) => Err(err.into()),
    };

    if tx_result.is_err() {
        let _ = write_quality_status_unavailable(conn);
        return false;
    }
    true
}

fn sorted_paths(paths: &HashSet<String>) -> Vec<String> {
    let mut sorted = paths.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    sorted
}

fn now_rfc3339() -> Result<String> {
    Ok(time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?)
}
