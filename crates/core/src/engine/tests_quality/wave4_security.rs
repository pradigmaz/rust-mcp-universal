use super::{Engine, RuleViolationsOptions, temp_dir, write_project_file};
use crate::model::{
    FindingConfidence, FindingFamily, SensitiveDataOptions, SignalMemoryDecision,
    SignalMemoryMarkRequest, SignalMemoryOptions, SignalMemoryStatus,
};

#[test]
fn dead_code_and_security_smells_surface_as_manual_review_signals() -> anyhow::Result<()> {
    let root = temp_dir("rmu-wave4-quality-signals");
    std::fs::create_dir_all(&root)?;
    write_project_file(&root, "src/orphan.rs", "pub fn orphan_helper() {}\n")?;
    write_project_file(
        &root,
        "src/shell.rs",
        "use std::process::Command;\npub fn run(cmd: &str) { let _ = Command::new(cmd); }\n",
    )?;
    write_project_file(&root, "src/lib.rs", "mod shell;\npub use shell::run;\n")?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.rule_violations(&RuleViolationsOptions::default())?;
    let orphan = result
        .hits
        .iter()
        .find(|hit| hit.path == "src/orphan.rs")
        .expect("dead-code hit should exist");
    let dead_code = orphan
        .violations
        .iter()
        .find(|violation| violation.rule_id == "dead_code_unused_export_candidate")
        .expect("dead code violation should exist");
    assert_eq!(dead_code.finding_family, Some(FindingFamily::DeadCode));
    assert!(dead_code.manual_review_required);
    assert!(dead_code.signal_key.is_some());

    let shell = result
        .hits
        .iter()
        .find(|hit| hit.path == "src/shell.rs")
        .expect("security smell hit should exist");
    let smell = shell
        .violations
        .iter()
        .find(|violation| violation.rule_id == "security_smell_shell_exec")
        .expect("shell exec smell should exist");
    assert_eq!(smell.finding_family, Some(FindingFamily::SecuritySmells));
    assert!(smell.manual_review_required);
    let risk_score = shell.risk_score.as_ref().expect("risk score should exist");
    let non_smell_violations = shell
        .violations
        .iter()
        .filter(|violation| violation.finding_family != Some(FindingFamily::SecuritySmells))
        .count() as f64;
    let non_smell_severity = shell
        .violations
        .iter()
        .filter(|violation| violation.finding_family != Some(FindingFamily::SecuritySmells))
        .map(|violation| match violation.severity {
            crate::model::QualitySeverity::Low => 1.0,
            crate::model::QualitySeverity::Medium => 2.0,
            crate::model::QualitySeverity::High => 4.0,
            crate::model::QualitySeverity::Critical => 8.0,
        })
        .sum::<f64>();
    assert_eq!(risk_score.components.violation_count, non_smell_violations);
    assert_eq!(risk_score.components.severity, non_smell_severity);

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn signal_memory_marks_quality_signal_as_noisy() -> anyhow::Result<()> {
    let root = temp_dir("rmu-wave4-signal-memory");
    std::fs::create_dir_all(&root)?;
    write_project_file(&root, "src/orphan.rs", "pub fn orphan_helper() {}\n")?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let result = engine.rule_violations(&RuleViolationsOptions::default())?;
    let signal = result
        .hits
        .iter()
        .flat_map(|hit| hit.violations.iter())
        .find(|violation| violation.rule_id == "dead_code_unused_export_candidate")
        .and_then(|violation| violation.signal_key.clone())
        .expect("signal key should be present");

    engine.mark_signal_memory(&SignalMemoryMarkRequest {
        signal_key: signal.clone(),
        finding_family: FindingFamily::DeadCode,
        scope: Some("src/orphan.rs".to_string()),
        decision: SignalMemoryDecision::Noisy,
        reason: "known plugin export".to_string(),
        source: "test".to_string(),
    })?;

    let rerun = engine.rule_violations(&RuleViolationsOptions::default())?;
    let memory_status = rerun
        .hits
        .iter()
        .flat_map(|hit| hit.violations.iter())
        .find(|violation| violation.signal_key.as_deref() == Some(signal.as_str()))
        .and_then(|violation| violation.memory_status);
    assert_eq!(memory_status, Some(SignalMemoryStatus::RememberedNoisy));

    let stored = engine.signal_memory(&SignalMemoryOptions::default())?;
    assert!(
        stored
            .entries
            .iter()
            .any(|entry| entry.signal_key == signal && entry.reason == "known plugin export")
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn sensitive_data_skips_placeholders_but_reports_real_patterns() -> anyhow::Result<()> {
    let root = temp_dir("rmu-wave4-sensitive-data");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "docs/example.md",
        "Use token sk-example-placeholder for local docs only.\n",
    )?;
    write_project_file(
        &root,
        "fixtures/sample.env",
        "GITHUB_TOKEN=ghp_exampleplaceholdertokenvalue\n",
    )?;
    write_project_file(
        &root,
        "src/keys.txt",
        "OPENAI_API_KEY=sk-AbCdEfGhIjKlMnOpQrStUvWxYz123456\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    let result = engine.sensitive_data(&SensitiveDataOptions::default())?;

    assert_eq!(result.summary.findings, 1);
    let finding = result.hits.first().expect("real secret should be reported");
    assert_eq!(finding.secret_kind, "openai_api_key");
    assert_eq!(finding.confidence, FindingConfidence::High);
    assert_eq!(finding.finding_family, FindingFamily::SensitiveData);

    engine.mark_signal_memory(&SignalMemoryMarkRequest {
        signal_key: finding.signal_key.clone(),
        finding_family: FindingFamily::SensitiveData,
        scope: Some("src/keys.txt".to_string()),
        decision: SignalMemoryDecision::Noisy,
        reason: "accepted fixture in test".to_string(),
        source: "test".to_string(),
    })?;
    let rerun = engine.sensitive_data(&SensitiveDataOptions::default())?;
    assert_eq!(
        rerun.summary.findings, 1,
        "high-confidence secret must stay visible"
    );
    assert_eq!(
        rerun.hits[0].memory_status,
        Some(SignalMemoryStatus::RememberedNoisy)
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
