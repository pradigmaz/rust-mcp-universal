use anyhow::Result;
use rusqlite::Connection;

use super::table_ensure::ensure_table_columns;

pub(super) fn ensure_file_quality_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS file_quality (
            path TEXT PRIMARY KEY,
            language TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            total_lines INTEGER,
            non_empty_lines INTEGER,
            import_count INTEGER,
            quality_mode TEXT NOT NULL,
            source_mtime_unix_ms INTEGER,
            quality_ruleset_version INTEGER NOT NULL,
            quality_metric_count INTEGER NOT NULL DEFAULT 0,
            quality_metric_hash TEXT NOT NULL DEFAULT '',
            quality_violation_count INTEGER NOT NULL,
            quality_violation_hash TEXT NOT NULL,
            quality_suppressed_violation_count INTEGER NOT NULL DEFAULT 0,
            quality_suppressed_violation_hash TEXT NOT NULL DEFAULT '',
            suppressed_violations_json TEXT NOT NULL DEFAULT '[]',
            quality_indexed_at_utc TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_file_quality_language ON file_quality(language);
        CREATE INDEX IF NOT EXISTS idx_file_quality_violation_count
            ON file_quality(quality_violation_count);

        CREATE TABLE IF NOT EXISTS file_quality_metrics (
            path TEXT NOT NULL,
            metric_id TEXT NOT NULL,
            metric_value INTEGER NOT NULL,
            source TEXT,
            start_line INTEGER,
            start_column INTEGER,
            end_line INTEGER,
            end_column INTEGER,
            PRIMARY KEY(path, metric_id)
        );
        CREATE INDEX IF NOT EXISTS idx_file_quality_metrics_metric
            ON file_quality_metrics(metric_id);

        CREATE TABLE IF NOT EXISTS file_rule_violations (
            path TEXT NOT NULL,
            rule_id TEXT NOT NULL,
            actual_value INTEGER NOT NULL,
            threshold_value INTEGER NOT NULL,
            message TEXT NOT NULL,
            severity TEXT NOT NULL DEFAULT 'medium',
            category TEXT NOT NULL DEFAULT 'maintainability',
            source TEXT,
            finding_family TEXT,
            confidence TEXT,
            manual_review_required INTEGER NOT NULL DEFAULT 0,
            noise_reason TEXT,
            recommended_followups_json TEXT NOT NULL DEFAULT '[]',
            start_line INTEGER,
            start_column INTEGER,
            end_line INTEGER,
            end_column INTEGER,
            PRIMARY KEY(path, rule_id)
        );
        CREATE INDEX IF NOT EXISTS idx_file_rule_violations_rule
            ON file_rule_violations(rule_id);
        "#,
    )?;
    ensure_table_columns(
        conn,
        "file_quality",
        &[
            ("quality_metric_count", "INTEGER NOT NULL DEFAULT 0"),
            ("quality_metric_hash", "TEXT NOT NULL DEFAULT ''"),
            (
                "quality_suppressed_violation_count",
                "INTEGER NOT NULL DEFAULT 0",
            ),
            (
                "quality_suppressed_violation_hash",
                "TEXT NOT NULL DEFAULT ''",
            ),
            ("suppressed_violations_json", "TEXT NOT NULL DEFAULT '[]'"),
        ],
    )?;
    ensure_table_columns(
        conn,
        "file_quality_metrics",
        &[
            ("source", "TEXT"),
            ("start_line", "INTEGER"),
            ("start_column", "INTEGER"),
            ("end_line", "INTEGER"),
            ("end_column", "INTEGER"),
        ],
    )?;
    ensure_table_columns(
        conn,
        "file_rule_violations",
        &[
            ("severity", "TEXT NOT NULL DEFAULT 'medium'"),
            ("category", "TEXT NOT NULL DEFAULT 'maintainability'"),
            ("source", "TEXT"),
            ("finding_family", "TEXT"),
            ("confidence", "TEXT"),
            ("manual_review_required", "INTEGER NOT NULL DEFAULT 0"),
            ("noise_reason", "TEXT"),
            ("recommended_followups_json", "TEXT NOT NULL DEFAULT '[]'"),
            ("start_line", "INTEGER"),
            ("start_column", "INTEGER"),
            ("end_line", "INTEGER"),
            ("end_column", "INTEGER"),
        ],
    )?;
    Ok(())
}
