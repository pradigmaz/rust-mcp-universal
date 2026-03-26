use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::engine::Engine;
use crate::model::{AgentBootstrapIncludeOptions, PrivacyMode, SemanticFailMode};

fn temp_project_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

#[test]
fn agent_bootstrap_is_fast_by_default() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-agent-bootstrap-fast-default");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn interesting_symbol() {\n    println!(\"ok\");\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let payload = engine.agent_bootstrap_with_auto_index_and_mode(
        Some("interesting_symbol"),
        3,
        false,
        SemanticFailMode::FailOpen,
        PrivacyMode::Off,
        12_000,
        3_000,
        false,
    )?;

    let bundle = payload.query_bundle.expect("query bundle");
    assert!(!bundle.hits.is_empty());
    assert!(bundle.investigation_summary.is_none());
    assert!(bundle.report.is_none());
    assert!(payload.timings.total_ms >= payload.timings.brief_ms);
    assert!(payload.timings.total_ms >= payload.timings.search_ms);
    assert!(payload.timings.total_ms >= payload.timings.context_ms);

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn agent_bootstrap_can_opt_into_report_and_investigation_summary() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-agent-bootstrap-opt-in");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn interesting_symbol() {\n    println!(\"ok\");\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let payload = engine.agent_bootstrap_with_auto_index_and_options(
        Some("interesting_symbol"),
        3,
        false,
        SemanticFailMode::FailOpen,
        PrivacyMode::Off,
        12_000,
        3_000,
        false,
        AgentBootstrapIncludeOptions {
            include_report: true,
            include_investigation_summary: true,
        },
    )?;

    let bundle = payload.query_bundle.expect("query bundle");
    assert!(bundle.investigation_summary.is_some());
    let report = bundle.report.expect("report");
    assert!(report.investigation_summary.is_some());
    assert!(payload.timings.total_ms >= payload.timings.investigation_ms);
    assert!(payload.timings.total_ms >= payload.timings.report_ms);

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}
