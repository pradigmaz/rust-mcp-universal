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
                .and_then(|value| value.to_str())
                .filter(|value| !value.is_empty())
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

pub fn sanitize_error_message(mode: PrivacyMode, raw: &str) -> String {
    match mode {
        PrivacyMode::Off => raw.to_string(),
        PrivacyMode::Mask => "operation failed (privacy_mode=mask); details redacted".to_string(),
        PrivacyMode::Hash => {
            let digest = hash_bytes(raw.as_bytes());
            let short = digest.get(..12).unwrap_or(digest.as_str());
            format!("operation failed (privacy_mode=hash); fingerprint={short}")
        }
    }
}

fn sanitize_content_text(mode: PrivacyMode, raw: &str) -> String {
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

pub fn sanitize_value_for_privacy(mode: PrivacyMode, value: &mut Value) {
    sanitize_value_with_hint(mode, value, None);
}

fn sanitize_value_with_hint(mode: PrivacyMode, value: &mut Value, key: Option<&str>) {
    match value {
        Value::Object(map) => {
            for (entry_key, entry_value) in map.iter_mut() {
                sanitize_value_with_hint(mode, entry_value, Some(entry_key));
            }
        }
        Value::Array(items) => {
            for item in items {
                sanitize_value_with_hint(mode, item, key);
            }
        }
        Value::String(text) => {
            if let Some(kind) = key.and_then(classify_sensitive_key) {
                *text = match kind {
                    SensitiveKind::Path => sanitize_path_text(mode, text),
                    SensitiveKind::Query => sanitize_query_text(mode, text),
                    SensitiveKind::Content => sanitize_content_text(mode, text),
                    SensitiveKind::Error => sanitize_error_message(mode, text),
                };
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
    if lowered == "query" || lowered == "seed" {
        return Some(SensitiveKind::Query);
    }
    if lowered == "project_root"
        || lowered == "memory_root"
        || lowered == "db_path"
        || lowered == "file_path"
        || lowered.ends_with("_path")
        || lowered.ends_with("_paths")
    {
        return Some(SensitiveKind::Path);
    }
    if lowered == "summary"
        || lowered == "reason"
        || lowered == "recommended_action"
        || lowered == "safe_recovery_hint"
    {
        return Some(SensitiveKind::Content);
    }
    None
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{sanitize_path_text, sanitize_query_text, sanitize_value_for_privacy};
    use crate::model::PrivacyMode;

    #[test]
    fn masks_path_and_query() {
        assert_eq!(
            sanitize_path_text(PrivacyMode::Mask, r"C:\vault\decisions\auth-token.md"),
            "<masked:auth-token.md>"
        );
        assert_eq!(
            sanitize_query_text(PrivacyMode::Mask, "token strategy"),
            "<redacted-query>"
        );
    }

    #[test]
    fn scrubs_json_by_key() {
        let mut value = json!({
            "project_root": r"C:\vault",
            "memory_root": r"C:\vault\memory",
            "query": "auth",
            "summary": "secret summary"
        });
        sanitize_value_for_privacy(PrivacyMode::Mask, &mut value);
        assert_eq!(value["project_root"], json!("<masked:vault>"));
        assert_eq!(value["memory_root"], json!("<masked:memory>"));
        assert_eq!(value["query"], json!("<redacted-query>"));
        assert_eq!(value["summary"], json!("<redacted-content>"));
    }
}
