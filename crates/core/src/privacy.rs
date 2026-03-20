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
    Error,
}

fn classify_sensitive_key(key: &str) -> Option<SensitiveKind> {
    let lowered = key.to_ascii_lowercase();
    if lowered == "error" {
        return Some(SensitiveKind::Error);
    }
    if lowered == "query" || lowered.ends_with("_query") {
        return Some(SensitiveKind::Query);
    }
    if lowered == "path"
        || lowered.ends_with("_path")
        || lowered.ends_with("_paths")
        || lowered.ends_with("_root")
        || lowered == "removed_files"
        || lowered == "project_root"
        || lowered == "db_path"
    {
        return Some(SensitiveKind::Path);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{sanitize_path_text, sanitize_query_text, sanitize_value_for_privacy};
    use crate::model::PrivacyMode;
    use serde_json::json;

    #[test]
    fn path_sanitization_masks_and_hashes() {
        let masked = sanitize_path_text(PrivacyMode::Mask, r"C:\Users\Alice\repo\src\main.rs");
        assert_eq!(masked, "<masked:main.rs>");
        let hashed = sanitize_path_text(PrivacyMode::Hash, "/home/alice/repo/src/main.rs");
        assert!(hashed.starts_with("<hash:"));
        assert!(hashed.ends_with('>'));
    }

    #[test]
    fn query_sanitization_masks_query() {
        assert_eq!(
            sanitize_query_text(PrivacyMode::Mask, "secret query"),
            "<redacted-query>"
        );
        assert!(sanitize_query_text(PrivacyMode::Hash, "secret query").starts_with("<query-hash:"));
    }

    #[test]
    fn json_sanitization_scrubs_sensitive_keys() {
        let mut payload = json!({
            "project_root": "C:\\Users\\Alice\\repo",
            "db_path": "C:\\Users\\Alice\\repo\\.rmu\\index.db",
            "query": "find secret",
            "nested": {
                "path": "src/main.rs"
            },
            "candidate_paths": ["src/lib.rs"]
        });
        sanitize_value_for_privacy(PrivacyMode::Mask, &mut payload);
        assert_eq!(payload["query"], json!("<redacted-query>"));
        assert_eq!(payload["nested"]["path"], json!("<masked:main.rs>"));
        assert_eq!(payload["project_root"], json!("<masked:repo>"));
        assert_eq!(payload["candidate_paths"][0], json!("<masked:lib.rs>"));
    }

    #[test]
    fn json_sanitization_scrubs_removed_files_arrays() {
        let mut payload = json!({
            "removed_files": [
                "C:\\Users\\Alice\\repo\\.rmu\\index.db",
                "C:\\Users\\Alice\\repo\\.rmu\\index.db-wal"
            ]
        });
        sanitize_value_for_privacy(PrivacyMode::Mask, &mut payload);
        assert_eq!(payload["removed_files"][0], json!("<masked:index.db>"));
        assert_eq!(payload["removed_files"][1], json!("<masked:index.db-wal>"));
    }
}
