use std::collections::HashSet;

use anyhow::Result;

use super::refresh_input::build_refresh_input;
use super::refresh_persist::{QualityRefreshRecord, persist_quality_refresh};
use super::scope::{apply_quality_scope_policy, build_full_quality_refresh_plan};
use super::status::write_quality_status_unavailable;
use super::structural::{load_graph_structural_facts, load_layering_facts};
use crate::engine::Engine;
use crate::quality::{
    DuplicationCandidate, analyze_duplication, default_quality_policy, evaluate_quality,
    load_git_risk_facts, load_quality_policy, load_quality_policy_digest, load_test_risk_facts,
    quality_metrics_hash, suppressed_violations_hash, violations_hash, write_duplication_artifact,
};

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
    let structural_facts = match load_graph_structural_facts(&conn, &plan.refresh_paths) {
        Ok(facts) => facts,
        Err(_) => {
            degraded = true;
            if last_error_rule_id.is_none() {
                last_error_rule_id = Some("graph_structural".to_string());
            }
            std::collections::HashMap::new()
        }
    };
    let layering_facts = match load_layering_facts(&conn, &plan.refresh_paths, &policy) {
        Ok(facts) => facts,
        Err(_) => {
            degraded = true;
            if last_error_rule_id.is_none() {
                last_error_rule_id = Some("layering".to_string());
            }
            std::collections::HashMap::new()
        }
    };
    let git_risk_facts =
        match load_git_risk_facts(&engine.project_root, &plan.refresh_paths, &policy.git_risk) {
            Ok(facts) => facts,
            Err(_) => {
                degraded = true;
                if last_error_rule_id.is_none() {
                    last_error_rule_id = Some("git_risk".to_string());
                }
                std::collections::HashMap::new()
            }
        };

    let mut refresh_inputs = Vec::new();
    let mut deleted_paths = plan.deleted_paths.clone();
    for path in sorted_paths(&plan.refresh_paths) {
        let structural = structural_facts.get(&path).cloned().unwrap_or_default();
        let layering = layering_facts.get(&path).cloned().unwrap_or_default();
        let git_risk = git_risk_facts.get(&path).cloned().unwrap_or_default();
        match build_refresh_input(&conn, engine, &path, structural, layering, git_risk) {
            Ok(Some(input)) => refresh_inputs.push(input),
            Ok(None) => {
                deleted_paths.insert(path);
            }
            Err(_) => degraded = true,
        }
    }
    let test_risk_facts = match load_test_risk_facts(
        &engine.project_root,
        &refresh_inputs
            .iter()
            .map(|input| (input.path.as_str(), &input.facts))
            .collect::<Vec<_>>(),
        &policy.test_risk,
    ) {
        Ok(facts) => facts,
        Err(_) => {
            degraded = true;
            if last_error_rule_id.is_none() {
                last_error_rule_id = Some("test_risk".to_string());
            }
            std::collections::HashMap::new()
        }
    };

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
        input.facts.test_risk = test_risk_facts
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

    if !persist_quality_refresh(
        &conn,
        &deleted_paths,
        &records,
        degraded,
        last_error_rule_id.as_deref(),
        &policy_digest,
    ) {
        return Ok(());
    }
    if write_duplication_artifact(&engine.project_root, &duplication.artifact).is_err() {
        let _ = write_quality_status_unavailable(&conn);
    }
    Ok(())
}

fn sorted_paths(paths: &std::collections::HashSet<String>) -> Vec<String> {
    let mut sorted = paths.iter().cloned().collect::<Vec<_>>();
    sorted.sort();
    sorted
}
