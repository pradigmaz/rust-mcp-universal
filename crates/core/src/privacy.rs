use std::path::Path;

use serde_json::Value;

use crate::model::PrivacyMode;
use crate::utils::hash_bytes;

pub fn sanitize_path_text(mode: PrivacyMode, raw: &str) -> String {
    match mode {
        PrivacyMode::Off => raw.to_string(),
        PrivacyMode::Mask => {
            let normalized = raw.replace('\\', "/");
            let file_name = Path::new(&normalized)
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .unwrap_or("***");
            format!("<masked:{file_name}>")
        }
        PrivacyMode::Hash => {
            let digest = hash_bytes(raw.as_bytes());
            let short = digest.get(..12).unwrap_or(digest.as_str());
            format!("<hash:{short}>")
        }
    }
}

pub fn sanitize_query_text(mode: PrivacyMode, raw: &str) -> String {
    match mode {
        PrivacyMode::Off => raw.to_string(),
        PrivacyMode::Mask => "<redacted-query>".to_string(),
        PrivacyMode::Hash => {
            let digest = hash_bytes(raw.as_bytes());
            let short = digest.get(..12).unwrap_or(digest.as_str());
            format!("<query-hash:{short}>")
        }
    }
}

pub fn sanitize_content_text(mode: PrivacyMode, raw: &str) -> String {
    match mode {
        PrivacyMode::Off => raw.to_string(),
        PrivacyMode::Mask => "<redacted-content>".to_string(),
        PrivacyMode::Hash => {
            let digest = hash_bytes(raw.as_bytes());
            let short = digest.get(..12).unwrap_or(digest.as_str());
            format!("<content-hash:{short}>")
        }
    }
}

pub fn sanitize_error_message(mode: PrivacyMode, message: &str) -> String {
    match mode {
        PrivacyMode::Off => message.to_string(),
        PrivacyMode::Mask => "operation failed (privacy_mode=mask); details redacted".to_string(),
        PrivacyMode::Hash => {
            let digest = hash_bytes(message.as_bytes());
            let short = digest.get(..12).unwrap_or(digest.as_str());
            format!("operation failed (privacy_mode=hash); fingerprint={short}")
        }
    }
}

pub fn sanitize_value_for_privacy(mode: PrivacyMode, value: &mut Value) {
    sanitize_value_for_privacy_with_hint(mode, value, None);
}

fn sanitize_value_for_privacy_with_hint(mode: PrivacyMode, value: &mut Value, key: Option<&str>) {
    match value {
        Value::Object(map) => {
            for (entry_key, entry_value) in map.iter_mut() {
                sanitize_value_for_privacy_with_hint(mode, entry_value, Some(entry_key));
            }
        }
        Value::Array(items) => {
            for item in items {
                sanitize_value_for_privacy_with_hint(mode, item, key);
            }
        }
        Value::String(text) => {
            if let Some(kind) = key.and_then(classify_sensitive_key) {
                let sanitized = match kind {
                    SensitiveKind::Path => sanitize_path_text(mode, text),
                    SensitiveKind::Query => sanitize_query_text(mode, text),
                    SensitiveKind::Content => sanitize_content_text(mode, text),
                    SensitiveKind::Error => sanitize_error_message(mode, text),
                };
                *text = sanitized;
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SensitiveKind {
    Path,
    Query,
    Content,
    Error,
}

fn classify_sensitive_key(key: &str) -> Option<SensitiveKind> {
    let lowered = key.to_ascii_lowercase();
    if lowered == "error" {
        return Some(SensitiveKind::Error);
    }
    if lowered == "query" || lowered == "seed" || lowered.ends_with("_query") {
        return Some(SensitiveKind::Query);
    }
    if lowered == "body"
        || lowered == "signature"
        || lowered == "symbol"
        || lowered == "anchor_symbol"
        || lowered == "excerpt"
        || lowered == "normalized_key"
        || lowered == "normalized_keys"
        || lowered == "constraint_keys"
        || lowered == "normalized_text"
        || lowered == "evidence"
        || lowered == "summary"
        || lowered == "reason"
        || lowered == "detail"
        || lowered == "rank_reason"
        || lowered == "checks"
        || lowered == "followups"
        || lowered == "recommended_followups"
        || lowered == "shared_evidence"
        || lowered == "missing_evidence"
        || lowered == "unknowns"
        || lowered == "gaps"
    {
        return Some(SensitiveKind::Content);
    }
    if lowered == "path"
        || lowered.ends_with("_path")
        || lowered.ends_with("_paths")
        || lowered.ends_with("_root")
        || lowered == "related_tests"
        || lowered == "launcher_recommended"
        || lowered == "removed_files"
        || lowered == "project_root"
        || lowered == "db_path"
        || lowered == "resolved_project_path"
        || lowered == "resolved_db_path"
    {
        return Some(SensitiveKind::Path);
    }
    None
}

#[cfg(test)]
#[path = "privacy_tests.rs"]
mod tests;
