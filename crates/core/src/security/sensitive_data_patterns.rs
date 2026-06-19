use crate::model::{
    FindingConfidence, FindingFamily, SensitiveDataExposureScope, SensitiveDataFinding,
    SensitiveDataPlaceholderStatus, SensitiveDataRotationUrgency, SensitiveDataSnippetType,
    SensitiveDataValidationStatus, SignalMemoryEntry,
};

pub(super) fn scan_file(
    rel_path: &str,
    text: &str,
    include_low_confidence: bool,
    memory: &[SignalMemoryEntry],
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
    memory: &[SignalMemoryEntry],
) -> Option<SensitiveDataFinding> {
    line.contains("BEGIN PRIVATE KEY").then(|| {
        build_finding(BuildFindingInput {
            rel_path,
            line_no,
            secret_kind: "private_key",
            excerpt_source: line,
            confidence: FindingConfidence::High,
            snippet_type: SensitiveDataSnippetType::PrivateKeyHeader,
            rotation_urgency: SensitiveDataRotationUrgency::Critical,
            memory,
        })
    })
}

fn pattern_token_findings(
    rel_path: &str,
    line_no: usize,
    line: &str,
    memory: &[SignalMemoryEntry],
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
        hits.push(build_finding(BuildFindingInput {
            rel_path,
            line_no,
            secret_kind,
            excerpt_source: word,
            confidence,
            snippet_type: SensitiveDataSnippetType::InlineToken,
            rotation_urgency: urgency,
            memory,
        }));
    }
    hits
}

fn assignment_finding(
    rel_path: &str,
    line_no: usize,
    line: &str,
    memory: &[SignalMemoryEntry],
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
    Some(build_finding(BuildFindingInput {
        rel_path,
        line_no,
        secret_kind: "credential_assignment",
        excerpt_source: value,
        confidence: FindingConfidence::Medium,
        snippet_type: SensitiveDataSnippetType::Assignment,
        rotation_urgency: SensitiveDataRotationUrgency::Medium,
        memory,
    }))
}

struct BuildFindingInput<'a> {
    rel_path: &'a str,
    line_no: usize,
    secret_kind: &'a str,
    excerpt_source: &'a str,
    confidence: FindingConfidence,
    snippet_type: SensitiveDataSnippetType,
    rotation_urgency: SensitiveDataRotationUrgency,
    memory: &'a [SignalMemoryEntry],
}

fn build_finding(input: BuildFindingInput<'_>) -> SensitiveDataFinding {
    let location = Some(crate::model::QualityLocation {
        start_line: input.line_no,
        start_column: 1,
        end_line: input.line_no,
        end_column: input.excerpt_source.len().max(1),
    });
    let redacted = redact_excerpt(input.excerpt_source);
    let signal_key = crate::signal_memory::build_sensitive_signal_key(
        input.rel_path,
        input.secret_kind,
        &redacted,
        location.as_ref(),
    );
    let memory_status = crate::signal_memory::signal_memory_status(input.memory, &signal_key);
    SensitiveDataFinding {
        signal_key,
        finding_family: FindingFamily::SensitiveData,
        secret_kind: input.secret_kind.to_string(),
        path: input.rel_path.to_string(),
        location,
        snippet_type: input.snippet_type,
        confidence: input.confidence,
        validation_status: SensitiveDataValidationStatus::PatternMatch,
        placeholder_status: SensitiveDataPlaceholderStatus::Realistic,
        exposure_scope: SensitiveDataExposureScope::CommittedText,
        rotation_urgency: input.rotation_urgency,
        manual_review_required: true,
        match_excerpt: Some(redacted),
        memory_status,
    }
}

pub(super) fn skip_path(rel_path: &str) -> bool {
    let path = rel_path.to_ascii_lowercase();
    path.starts_with(".git/")
        || path.starts_with(".rmu/")
        || path.starts_with(".codex/")
        || path.starts_with("target/")
}

pub(super) fn matches_path_prefix(path: &str, path_prefix: Option<&str>) -> bool {
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
