use std::collections::HashSet;
use std::fs;

use anyhow::Result;

use super::scope::{apply_quality_scope_policy, build_full_quality_refresh_plan};
use super::status::{
    write_quality_status_degraded, write_quality_status_ready, write_quality_status_unavailable,
};
use super::structural::load_structural_facts;
use crate::engine::Engine;
use crate::engine::storage::{
    UpsertQualitySnapshotInput, remove_path_quality, upsert_quality_snapshot,
};
use crate::quality::{
    DuplicationCandidate, IndexedQualityMetrics, analyze_duplication, build_indexed_quality_facts,
    build_oversize_quality_facts, default_quality_policy, evaluate_quality, load_quality_policy,
    load_quality_policy_digest, quality_metrics_hash, suppressed_violations_hash, violations_hash,
    write_duplication_artifact,
};
use crate::utils::{INDEX_FILE_LIMIT, infer_language, normalized_path_to_fs_path};

#[derive(Debug)]
struct QualityRefreshInput {
    path: String,
    language: String,
    source_mtime_unix_ms: Option<i64>,
    facts: crate::quality::QualityCandidateFacts,
    indexed_metrics: IndexedQualityMetrics,
    source_text: Option<String>,
}

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
}

pub(super) fn refresh_quality_after_index(
    engine: &Engine,
    refresh_paths: &HashSet<String>,
    deleted_paths: &HashSet<String>,
) -> Result<()> {
    let conn = engine.open_db_read_only()?;
    let mut plan = build_full_quality_refresh_plan(engine, &conn)?;
    plan.refresh_paths.extend(refresh_paths.iter().cloned());
    plan.deleted_paths.extend(deleted_paths.iter().cloned());
    let _ = apply_quality_refresh(engine, plan);
    Ok(())
}

pub(super) fn refresh_quality_only(engine: &Engine) -> Result<()> {
    let conn = engine.open_db_read_only()?;
    let plan = build_full_quality_refresh_plan(engine, &conn)?;
    let _ = apply_quality_refresh(engine, plan);
    Ok(())
}

fn apply_quality_refresh(engine: &Engine, plan: super::scope::QualityRefreshPlan) -> Result<()> {
    let conn = match engine.open_db() {
        Ok(conn) => conn,
        Err(_) => return Ok(()),
    };
    let mut degraded = false;
    let mut last_error_rule_id = None::<String>;
    let policy = match load_quality_policy(&engine.project_root) {
        Ok(policy) => policy,
        Err(_) => {
            degraded = true;
            last_error_rule_id = Some("quality_policy".to_string());
            default_quality_policy()
        }
    };
    let policy_digest = match load_quality_policy_digest(&engine.project_root) {
        Ok(digest) => digest,
        Err(_) => {
            degraded = true;
            if last_error_rule_id.is_none() {
                last_error_rule_id = Some("quality_policy_digest".to_string());
            }
            crate::utils::hash_bytes(b"quality-policy-digest-error")
        }
    };
    let plan = match apply_quality_scope_policy(&conn, plan, &policy) {
        Ok(plan) => plan,
        Err(_) => {
            degraded = true;
            if last_error_rule_id.is_none() {
                last_error_rule_id = Some("quality_scope".to_string());
            }
            super::scope::QualityRefreshPlan::default()
        }
    };
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

    let mut refresh_inputs = Vec::new();
    let mut deleted_paths = plan.deleted_paths.clone();
    for path in sorted_paths(&plan.refresh_paths) {
        let structural = structural_facts.get(&path).cloned().unwrap_or_default();
        match build_refresh_input(&conn, engine, &path, structural) {
            Ok(Some(input)) => refresh_inputs.push(input),
            Ok(None) => {
                deleted_paths.insert(path);
            }
            Err(_) => degraded = true,
        }
    }

    let duplication = analyze_duplication(
        &policy,
        crate::quality::QUALITY_RULESET_ID,
        &policy_digest,
        &refresh_inputs
            .iter()
            .map(|input| DuplicationCandidate {
                path: &input.path,
                language: &input.language,
                non_empty_lines: input.facts.non_empty_lines,
                source_text: input.source_text.as_deref(),
            })
            .collect::<Vec<_>>(),
    );

    let mut records = Vec::new();
    for mut input in refresh_inputs {
        input.facts.duplication = duplication
            .file_facts
            .get(&input.path)
            .cloned()
            .unwrap_or_default();
        let evaluation = evaluate_quality(&input.facts, &input.indexed_metrics, &policy);
        if evaluation.had_rule_errors {
            degraded = true;
            if last_error_rule_id.is_none() {
                last_error_rule_id = evaluation.last_error_rule_id.clone();
            }
        }
        records.push(QualityRefreshRecord {
            path: input.path,
            language: input.language,
            size_bytes: evaluation.snapshot.size_bytes,
            total_lines: evaluation.snapshot.total_lines,
            non_empty_lines: evaluation.snapshot.non_empty_lines,
            import_count: evaluation.snapshot.import_count,
            quality_mode: evaluation.snapshot.quality_mode,
            source_mtime_unix_ms: input.source_mtime_unix_ms,
            quality_metric_hash: quality_metrics_hash(&evaluation.snapshot.metrics),
            quality_violation_hash: violations_hash(&evaluation.snapshot.violations),
            quality_suppressed_violation_hash: suppressed_violations_hash(
                &evaluation.snapshot.suppressed_violations,
            ),
            metrics: evaluation.snapshot.metrics,
            violations: evaluation.snapshot.violations,
            suppressed_violations: evaluation.snapshot.suppressed_violations,
        });
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
                    write_quality_status_degraded(
                        &tx,
                        last_error_rule_id.as_deref(),
                        &policy_digest,
                    )?;
                } else {
                    write_quality_status_ready(&tx, &policy_digest)?;
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
        return Ok(());
    }
    if write_duplication_artifact(&engine.project_root, &duplication.artifact).is_err() {
        let _ = write_quality_status_unavailable(&conn);
    }
    Ok(())
}

fn build_refresh_input(
    conn: &rusqlite::Connection,
    engine: &Engine,
    path: &str,
    structural: crate::quality::StructuralFacts,
) -> Result<Option<QualityRefreshInput>> {
    let abs_path = engine.project_root.join(normalized_path_to_fs_path(path));
    let metadata = match fs::metadata(&abs_path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };
    let source_mtime_unix_ms = metadata.modified().ok().map(system_time_to_unix_ms);
    let language = infer_language(&abs_path);
    let indexed_metrics = load_indexed_quality_metrics(conn, path)?;
    if metadata.len() > INDEX_FILE_LIMIT {
        let mut facts =
            build_oversize_quality_facts(path, &language, metadata.len(), source_mtime_unix_ms);
        facts.structural = structural;
        let source_text = fs::read(&abs_path).ok().and_then(|bytes| {
            (!bytes.contains(&0)).then(|| String::from_utf8_lossy(&bytes).to_string())
        });
        return Ok(Some(QualityRefreshInput {
            path: path.to_string(),
            language,
            source_mtime_unix_ms,
            facts,
            indexed_metrics,
            source_text,
        }));
    }
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
    facts.structural = structural;
    Ok(Some(QualityRefreshInput {
        path: path.to_string(),
        language,
        source_mtime_unix_ms,
        facts,
        indexed_metrics,
        source_text: Some(full_text),
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
