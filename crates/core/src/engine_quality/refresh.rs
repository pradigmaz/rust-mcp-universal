use std::collections::HashSet;
use std::fs;

use anyhow::Result;

use super::scope::{QualityRefreshPlan, apply_quality_scope_policy, build_full_quality_refresh_plan};
use super::status::{
    write_quality_status_degraded, write_quality_status_ready, write_quality_status_unavailable,
};
use super::structural::load_structural_facts;
use crate::engine::Engine;
use crate::engine::storage::{
    UpsertQualitySnapshotInput, remove_path_quality, upsert_quality_snapshot,
};
use crate::quality::{
    IndexedQualityMetrics, build_indexed_quality_facts, build_oversize_quality_facts,
    default_quality_policy, evaluate_quality, load_quality_policy, quality_metrics_hash,
    suppressed_violations_hash, violations_hash,
};
use crate::utils::{INDEX_FILE_LIMIT, infer_language, normalized_path_to_fs_path};

#[derive(Debug)]
struct QualityRefreshRecord {
    path: String,
    language: String,
    size_bytes: i64,
    total_lines: Option<i64>,
    non_empty_lines: Option<i64>,
    import_count: Option<i64>,
    quality_mode: crate::model::QualityMode,
    source_mtime_unix_ms: Option<i64>,
    quality_metric_hash: String,
    quality_violation_hash: String,
    quality_suppressed_violation_hash: String,
    metrics: Vec<crate::quality::QualityMetricEntry>,
    violations: Vec<crate::model::QualityViolationEntry>,
    suppressed_violations: Vec<crate::model::SuppressedQualityViolationEntry>,
    had_rule_errors: bool,
    last_error_rule_id: Option<String>,
}

pub(super) fn refresh_quality_after_index(
    engine: &Engine,
    refresh_paths: &HashSet<String>,
    deleted_paths: &HashSet<String>,
) -> Result<()> {
    let plan = QualityRefreshPlan {
        refresh_paths: refresh_paths.clone(),
        deleted_paths: deleted_paths.clone(),
    };
    let _ = apply_quality_refresh(engine, plan);
    Ok(())
}

pub(super) fn refresh_quality_only(engine: &Engine) -> Result<()> {
    let conn = engine.open_db_read_only()?;
    let plan = build_full_quality_refresh_plan(engine, &conn)?;
    let _ = apply_quality_refresh(engine, plan);
    Ok(())
}

fn apply_quality_refresh(engine: &Engine, plan: QualityRefreshPlan) -> Result<()> {
    let conn = match engine.open_db() {
        Ok(conn) => conn,
        Err(_) => return Ok(()),
    };
    let mut degraded = false;
    let mut last_error_rule_id = None::<String>;
    let mut records = Vec::new();
    let policy = match load_quality_policy(&engine.project_root) {
        Ok(policy) => policy,
        Err(_) => {
            degraded = true;
            last_error_rule_id = Some("quality_policy".to_string());
            default_quality_policy()
        }
    };
    let plan = match apply_quality_scope_policy(&conn, plan, &policy) {
        Ok(plan) => plan,
        Err(_) => {
            degraded = true;
            if last_error_rule_id.is_none() {
                last_error_rule_id = Some("quality_scope".to_string());
            }
            QualityRefreshPlan::default()
        }
    };
    let mut deleted_paths = plan.deleted_paths.clone();
    let structural_facts = match load_structural_facts(&conn, &plan.refresh_paths, &policy) {
        Ok(facts) => facts,
        Err(_) => {
            degraded = true;
            if last_error_rule_id.is_none() {
                last_error_rule_id = Some("structural_policy".to_string());
            }
            std::collections::HashMap::new()
        }
    };

    for path in sorted_paths(&plan.refresh_paths) {
        let structural = structural_facts.get(&path).cloned().unwrap_or_default();
        match build_refresh_record(&conn, engine, &path, &policy, structural) {
            Ok(Some(record)) => {
                if record.had_rule_errors {
                    degraded = true;
                    if last_error_rule_id.is_none() {
                        last_error_rule_id = record.last_error_rule_id.clone();
                    }
                }
                records.push(record);
            }
            Ok(None) => {
                deleted_paths.insert(path);
            }
            Err(_) => {
                degraded = true;
            }
        }
    }

    let tx_result = match conn.unchecked_transaction() {
        Ok(tx) => {
            let result: Result<()> = (|| {
                for path in sorted_paths(&deleted_paths) {
                    remove_path_quality(&tx, &path)?;
                }

                for record in &records {
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
                            quality_suppressed_violation_hash: &record.quality_suppressed_violation_hash,
                            quality_indexed_at_utc: &now_rfc3339()?,
                            metrics: &record.metrics,
                            violations: &record.violations,
                            suppressed_violations: &record.suppressed_violations,
                        },
                    )?;
                }

                if degraded {
                    write_quality_status_degraded(&tx, last_error_rule_id.as_deref())?;
                } else {
                    write_quality_status_ready(&tx)?;
                }
                tx.commit()?;
                Ok(())
            })();
            result
        }
        Err(err) => Err(err.into()),
    };

    if tx_result.is_err() {
        let _ = write_quality_status_unavailable(&conn);
    }

    Ok(())
}

fn build_refresh_record(
    conn: &rusqlite::Connection,
    engine: &Engine,
    path: &str,
    policy: &crate::quality::QualityPolicy,
    structural: crate::quality::StructuralFacts,
) -> Result<Option<QualityRefreshRecord>> {
    let abs_path = engine.project_root.join(normalized_path_to_fs_path(path));
    let metadata = match fs::metadata(&abs_path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };
    let source_mtime_unix_ms = metadata.modified().ok().map(system_time_to_unix_ms);
    let language = infer_language(&abs_path);
    let evaluation = if metadata.len() > INDEX_FILE_LIMIT {
        let mut facts =
            build_oversize_quality_facts(path, &language, metadata.len(), source_mtime_unix_ms);
        facts.structural = structural;
        evaluate_quality(&facts, &IndexedQualityMetrics::default(), policy)
    } else {
        let bytes = match fs::read(&abs_path) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(None),
        };
        if bytes.contains(&0) {
            return Ok(None);
        }
        let full_text = String::from_utf8_lossy(&bytes).to_string();
        let mut facts = build_indexed_quality_facts(
            path,
            &language,
            metadata.len(),
            source_mtime_unix_ms,
            &full_text,
        );
        facts.structural = structural.clone();
        let indexed_metrics = load_indexed_quality_metrics(conn, path)?;
        evaluate_quality(&facts, &indexed_metrics, policy)
    };

    Ok(Some(QualityRefreshRecord {
        path: path.to_string(),
        language,
        size_bytes: evaluation.snapshot.size_bytes,
        total_lines: evaluation.snapshot.total_lines,
        non_empty_lines: evaluation.snapshot.non_empty_lines,
        import_count: evaluation.snapshot.import_count,
        quality_mode: evaluation.snapshot.quality_mode,
        source_mtime_unix_ms,
        quality_metric_hash: quality_metrics_hash(&evaluation.snapshot.metrics),
        quality_violation_hash: violations_hash(&evaluation.snapshot.violations),
        quality_suppressed_violation_hash: suppressed_violations_hash(
            &evaluation.snapshot.suppressed_violations,
        ),
        metrics: evaluation.snapshot.metrics,
        violations: evaluation.snapshot.violations,
        suppressed_violations: evaluation.snapshot.suppressed_violations,
        had_rule_errors: evaluation.had_rule_errors,
        last_error_rule_id: evaluation.last_error_rule_id,
    }))
}

fn load_indexed_quality_metrics(
    conn: &rusqlite::Connection,
    path: &str,
) -> Result<IndexedQualityMetrics> {
    Ok(conn
        .query_row(
            r#"
            SELECT graph_symbol_count, graph_ref_count, graph_module_dep_count, graph_edge_out_count
            FROM files
            WHERE path = ?1
            "#,
            [path],
            |row| {
                Ok(IndexedQualityMetrics {
                    symbol_count: row.get(0)?,
                    ref_count: row.get(1)?,
                    module_dep_count: row.get(2)?,
                    graph_edge_out_count: row.get(3)?,
                })
            },
        )
        .unwrap_or_default())
}

fn sorted_paths(paths: &std::collections::HashSet<String>) -> Vec<String> {
    let mut sorted = paths.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    sorted
}

fn now_rfc3339() -> Result<String> {
    Ok(time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?)
}

fn system_time_to_unix_ms(time: std::time::SystemTime) -> i64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}
