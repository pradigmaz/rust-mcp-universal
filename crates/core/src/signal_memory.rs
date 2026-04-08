use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::model::{
    FindingFamily, QualityViolationEntry, SignalMemoryDecision, SignalMemoryEntry,
    SignalMemoryMarkRequest, SignalMemoryOptions, SignalMemoryResult, SignalMemoryStatus,
};

const SIGNAL_MEMORY_FILE: &str = "signal-memory.json";

pub(crate) fn load_signal_memory(project_root: &Path) -> Result<Vec<SignalMemoryEntry>> {
    let path = signal_memory_path(project_root);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let payload = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&payload)?)
}

pub(crate) fn inspect_signal_memory(
    project_root: &Path,
    options: &SignalMemoryOptions,
) -> Result<SignalMemoryResult> {
    let mut entries = load_signal_memory(project_root)?;
    if let Some(finding_family) = options.finding_family {
        entries.retain(|entry| entry.finding_family == finding_family);
    }
    if let Some(decision) = options.decision {
        entries.retain(|entry| entry.decision == decision);
    }
    entries.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    entries.truncate(options.limit);
    Ok(SignalMemoryResult { entries })
}

pub(crate) fn mark_signal_memory(
    project_root: &Path,
    request: &SignalMemoryMarkRequest,
) -> Result<SignalMemoryEntry> {
    let mut entries = load_signal_memory(project_root)?;
    let updated = SignalMemoryEntry {
        signal_key: request.signal_key.clone(),
        finding_family: request.finding_family,
        scope: request.scope.clone(),
        decision: request.decision,
        reason: request.reason.clone(),
        source: request.source.clone(),
        updated_at: OffsetDateTime::now_utc().format(&Rfc3339)?,
    };
    entries.retain(|entry| entry.signal_key != request.signal_key);
    entries.push(updated.clone());
    persist_signal_memory(project_root, &entries)?;
    Ok(updated)
}

pub(crate) fn signal_memory_status(
    entries: &[SignalMemoryEntry],
    signal_key: &str,
) -> Option<SignalMemoryStatus> {
    entries
        .iter()
        .find(|entry| entry.signal_key == signal_key)
        .map(|entry| match entry.decision {
            SignalMemoryDecision::Useful => SignalMemoryStatus::RememberedUseful,
            SignalMemoryDecision::Noisy => SignalMemoryStatus::RememberedNoisy,
        })
}

pub(crate) fn build_quality_signal_key(path: &str, violation: &QualityViolationEntry) -> String {
    let finding_family = violation
        .finding_family
        .unwrap_or(FindingFamily::Ordinary)
        .as_str();
    let location = violation
        .location
        .as_ref()
        .map(|location| {
            format!(
                "{}:{}:{}:{}",
                location.start_line, location.start_column, location.end_line, location.end_column
            )
        })
        .unwrap_or_else(|| "-".to_string());
    let digest = crate::utils::hash_bytes(
        format!(
            "{finding_family}|{path}|{}|{}|{}|{}",
            violation.rule_id, violation.actual_value, violation.threshold_value, violation.message
        )
        .as_bytes(),
    );
    format!("quality:{finding_family}:{path}:{location}:{digest}")
}

pub(crate) fn build_sensitive_signal_key(
    path: &str,
    secret_kind: &str,
    excerpt: &str,
    location: Option<&crate::model::QualityLocation>,
) -> String {
    let location = location
        .map(|location| {
            format!(
                "{}:{}:{}:{}",
                location.start_line, location.start_column, location.end_line, location.end_column
            )
        })
        .unwrap_or_else(|| "-".to_string());
    let digest = crate::utils::hash_bytes(format!("{path}|{secret_kind}|{excerpt}").as_bytes());
    format!("security:sensitive_data:{path}:{location}:{digest}")
}

fn persist_signal_memory(project_root: &Path, entries: &[SignalMemoryEntry]) -> Result<()> {
    let path = signal_memory_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(entries)?)?;
    Ok(())
}

fn signal_memory_path(project_root: &Path) -> PathBuf {
    project_root.join(".rmu").join(SIGNAL_MEMORY_FILE)
}
