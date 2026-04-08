use std::cmp::Ordering;
use std::fs;

use anyhow::Result;
use ignore::WalkBuilder;

use crate::engine::Engine;
use crate::model::{
    FindingConfidence, FindingFamily, SensitiveDataExposureScope, SensitiveDataFinding,
    SensitiveDataOptions, SensitiveDataPlaceholderStatus, SensitiveDataResult,
    SensitiveDataRotationUrgency, SensitiveDataSnippetType, SensitiveDataSummary,
    SensitiveDataValidationStatus, SignalMemoryStatus,
};

const MAX_SCAN_BYTES: u64 = 262_144;

pub(super) fn scan_sensitive_data(
    engine: &Engine,
    options: &SensitiveDataOptions,
) -> Result<SensitiveDataResult> {
    let memory = crate::signal_memory::load_signal_memory(&engine.project_root)?;
    let mut evaluated_files = 0_usize;
    let mut hits = Vec::new();

    let walker = WalkBuilder::new(&engine.project_root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .build();
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let Ok(rel_path) = entry.path().strip_prefix(&engine.project_root) else {
            continue;
        };
        let rel_path = rel_path.to_string_lossy().replace('\\', "/");
        if !matches_path_prefix(&rel_path, options.path_prefix.as_deref()) || skip_path(&rel_path) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if metadata.len() > MAX_SCAN_BYTES {
            continue;
        }
        let Ok(bytes) = fs::read(entry.path()) else {
            continue;
        };
        if bytes.contains(&0) {
            continue;
        }
        evaluated_files += 1;
        let text = String::from_utf8_lossy(&bytes);
        hits.extend(scan_file(
            &rel_path,
            &text,
            options.include_low_confidence,
            &memory,
        ));
    }

    hits.sort_by(compare_findings);
    let remembered_noisy_findings = hits
        .iter()
        .filter(|finding| finding.memory_status == Some(SignalMemoryStatus::RememberedNoisy))
        .count();
    let high_confidence_findings = hits
        .iter()
        .filter(|finding| finding.confidence == FindingConfidence::High)
        .count();
    hits.truncate(options.limit);

    Ok(SensitiveDataResult {
        summary: SensitiveDataSummary {
            evaluated_files,
            findings: hits.len(),
            high_confidence_findings,
            remembered_noisy_findings,
        },
        hits,
    })
}

fn scan_file(
    rel_path: &str,
    text: &str,
    include_low_confidence: bool,
    memory: &[crate::model::SignalMemoryEntry],
) -> Vec<SensitiveDataFinding> {
    let mut hits = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if let Some(finding) = private_key_finding(rel_path, idx + 1, line, memory) {
            hits.push(finding);
        }
        hits.extend(pattern_token_findings(rel_path, idx + 1, line, memory));
        if include_low_confidence {
            if let Some(finding) = assignment_finding(rel_path, idx + 1, line, memory) {
                hits.push(finding);
            }
        }
    }
    hits
}

fn private_key_finding(
    rel_path: &str,
    line_no: usize,
    line: &str,
    memory: &[crate::model::SignalMemoryEntry],
) -> Option<SensitiveDataFinding> {
    line.contains("BEGIN PRIVATE KEY").then(|| {
        build_finding(
            rel_path,
            line_no,
            "private_key",
            line,
            FindingConfidence::High,
            SensitiveDataSnippetType::PrivateKeyHeader,
            SensitiveDataRotationUrgency::Critical,
            memory,
        )
    })
}

fn pattern_token_findings(
    rel_path: &str,
    line_no: usize,
    line: &str,
    memory: &[crate::model::SignalMemoryEntry],
) -> Vec<SensitiveDataFinding> {
    let mut hits = Vec::new();
    for word in
        line.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.'))
    {
        if word.is_empty() {
            continue;
        }
        let maybe = if looks_like_aws_access_key(word) {
            Some((
                "aws_access_key",
                FindingConfidence::High,
                SensitiveDataRotationUrgency::Critical,
            ))
        } else if looks_like_github_token(word) {
            Some((
                "github_pat",
                FindingConfidence::High,
                SensitiveDataRotationUrgency::High,
            ))
        } else if looks_like_openai_key(word) {
            Some((
                "openai_api_key",
                FindingConfidence::High,
                SensitiveDataRotationUrgency::High,
            ))
        } else {
            None
        };
        let Some((secret_kind, confidence, urgency)) = maybe else {
            continue;
        };
        if classify_placeholder(word, line) != SensitiveDataPlaceholderStatus::Realistic {
            continue;
        }
        hits.push(build_finding(
            rel_path,
            line_no,
            secret_kind,
            word,
            confidence,
            SensitiveDataSnippetType::InlineToken,
            urgency,
            memory,
        ));
    }
    hits
}

fn assignment_finding(
    rel_path: &str,
    line_no: usize,
    line: &str,
    memory: &[crate::model::SignalMemoryEntry],
) -> Option<SensitiveDataFinding> {
    let lowered = line.to_ascii_lowercase();
    let suspicious_name = ["password", "secret", "token", "api_key", "apikey"]
        .iter()
        .any(|needle| lowered.contains(needle));
    if !suspicious_name {
        return None;
    }
    let value = line
        .split(['=', ':'])
        .nth(1)?
        .trim()
        .trim_matches('"')
        .trim_matches('\'');
    if value.len() < 12 {
        return None;
    }
    if classify_placeholder(value, line) != SensitiveDataPlaceholderStatus::Realistic {
        return None;
    }
    Some(build_finding(
        rel_path,
        line_no,
        "credential_assignment",
        value,
        FindingConfidence::Medium,
        SensitiveDataSnippetType::Assignment,
        SensitiveDataRotationUrgency::Medium,
        memory,
    ))
}

fn build_finding(
    rel_path: &str,
    line_no: usize,
    secret_kind: &str,
    excerpt_source: &str,
    confidence: FindingConfidence,
    snippet_type: SensitiveDataSnippetType,
    rotation_urgency: SensitiveDataRotationUrgency,
    memory: &[crate::model::SignalMemoryEntry],
) -> SensitiveDataFinding {
    let location = Some(crate::model::QualityLocation {
        start_line: line_no,
        start_column: 1,
        end_line: line_no,
        end_column: excerpt_source.len().max(1),
    });
    let redacted = redact_excerpt(excerpt_source);
    let signal_key = crate::signal_memory::build_sensitive_signal_key(
        rel_path,
        secret_kind,
        &redacted,
        location.as_ref(),
    );
    let memory_status = crate::signal_memory::signal_memory_status(memory, &signal_key);
    SensitiveDataFinding {
        signal_key,
        finding_family: FindingFamily::SensitiveData,
        secret_kind: secret_kind.to_string(),
        path: rel_path.to_string(),
        location,
        snippet_type,
        confidence,
        validation_status: SensitiveDataValidationStatus::PatternMatch,
        placeholder_status: SensitiveDataPlaceholderStatus::Realistic,
        exposure_scope: SensitiveDataExposureScope::CommittedText,
        rotation_urgency,
        manual_review_required: true,
        match_excerpt: Some(redacted),
        memory_status,
    }
}

fn compare_findings(left: &SensitiveDataFinding, right: &SensitiveDataFinding) -> Ordering {
    confidence_rank(right.confidence)
        .cmp(&confidence_rank(left.confidence))
        .then_with(|| {
            urgency_rank(right.rotation_urgency).cmp(&urgency_rank(left.rotation_urgency))
        })
        .then_with(|| noisy_rank(left).cmp(&noisy_rank(right)))
        .then_with(|| left.path.cmp(&right.path))
        .then_with(|| {
            left.location
                .as_ref()
                .map(|location| location.start_line)
                .unwrap_or(usize::MAX)
                .cmp(
                    &right
                        .location
                        .as_ref()
                        .map(|location| location.start_line)
                        .unwrap_or(usize::MAX),
                )
        })
}

fn confidence_rank(confidence: FindingConfidence) -> usize {
    match confidence {
        FindingConfidence::High => 3,
        FindingConfidence::Medium => 2,
        FindingConfidence::Low => 1,
    }
}

fn urgency_rank(urgency: SensitiveDataRotationUrgency) -> usize {
    match urgency {
        SensitiveDataRotationUrgency::Critical => 3,
        SensitiveDataRotationUrgency::High => 2,
        SensitiveDataRotationUrgency::Medium => 1,
    }
}

fn noisy_rank(finding: &SensitiveDataFinding) -> usize {
    match (finding.confidence, finding.memory_status) {
        (FindingConfidence::High, _) => 0,
        (_, Some(SignalMemoryStatus::RememberedNoisy)) => 2,
        (_, Some(SignalMemoryStatus::RememberedUseful)) => 0,
        _ => 1,
    }
}

fn skip_path(rel_path: &str) -> bool {
    let path = rel_path.to_ascii_lowercase();
    path.starts_with(".git/")
        || path.starts_with(".rmu/")
        || path.starts_with(".codex/")
        || path.starts_with("target/")
}

fn matches_path_prefix(path: &str, path_prefix: Option<&str>) -> bool {
    path_prefix.is_none_or(|prefix| path.starts_with(prefix))
}

fn looks_like_aws_access_key(word: &str) -> bool {
    word.len() == 20
        && word.starts_with("AKIA")
        && word
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

fn looks_like_github_token(word: &str) -> bool {
    word.starts_with("ghp_")
        && word.len() >= 20
        && word.chars().skip(4).all(|ch| ch.is_ascii_alphanumeric())
}

fn looks_like_openai_key(word: &str) -> bool {
    word.starts_with("sk-")
        && word.len() >= 24
        && word
            .chars()
            .skip(3)
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

fn classify_placeholder(value: &str, line: &str) -> SensitiveDataPlaceholderStatus {
    let lowered = format!("{value} {line}").to_ascii_lowercase();
    if lowered.contains("placeholder")
        || lowered.contains("example")
        || lowered.contains("sample")
        || lowered.contains("dummy")
        || lowered.contains("fake")
        || lowered.contains("test")
    {
        return SensitiveDataPlaceholderStatus::Placeholder;
    }
    if value.contains("****")
        || value.contains("xxxx")
        || value.contains("xxxx")
        || value.contains("<redacted>")
        || value.contains("REDACTED")
    {
        return SensitiveDataPlaceholderStatus::Masked;
    }
    SensitiveDataPlaceholderStatus::Realistic
}

fn redact_excerpt(value: &str) -> String {
    if value.len() <= 8 {
        return "<redacted>".to_string();
    }
    format!(
        "{}…{}",
        &value[..4],
        &value[value.len().saturating_sub(4)..]
    )
}
