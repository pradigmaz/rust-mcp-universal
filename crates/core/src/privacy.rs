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
    {
        return Some(SensitiveKind::Path);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        sanitize_content_text, sanitize_path_text, sanitize_query_text, sanitize_value_for_privacy,
    };
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
    fn content_sanitization_masks_and_hashes() {
        assert_eq!(
            sanitize_content_text(PrivacyMode::Mask, "secret body"),
            "<redacted-content>"
        );
        assert!(
            sanitize_content_text(PrivacyMode::Hash, "secret body").starts_with("<content-hash:")
        );
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

    #[test]
    fn json_sanitization_scrubs_investigation_payload_content() {
        let mut payload = json!({
            "seed": "resolve_origin",
            "items": [
                {
                    "anchor": {
                        "path": "src/services/origin_service.rs",
                        "symbol": "resolve_origin"
                    },
                    "signature": "pub fn resolve_origin(key: &str) -> bool {",
                    "body": "pub fn resolve_origin(key: &str) -> bool {\n    validate_origin(key);\n    true\n}",
                    "source_kind": "symbol_lookup"
                }
            ],
            "variants": [
                {
                    "route": [
                        {
                            "path": "src/services/origin_service.rs",
                            "evidence": "service references validator"
                        }
                    ],
                    "related_tests": ["tests/test_origin_flow.py"],
                    "gaps": ["missing migration proof"]
                }
            ],
            "shared_evidence": ["both variants validate key"],
            "missing_evidence": ["db constraint not recovered"],
            "unknowns": ["legacy branch behavior"],
            "recommended_followups": ["inspect schema backing"],
            "divergence_signals": [
                {
                    "summary": "validator layer diverges"
                }
            ],
            "constraints": [
                {
                    "path": "migrations/001_create_origins.sql",
                    "excerpt": "create unique index uq_origins_origin_key on origins(origin_key)",
                    "normalized_key": "index_constraint:index_declaration:create unique index uq_origins_origin_key on origins(origin_key)",
                    "source_path": "migrations/001_create_origins.sql",
                    "normalized_text": "create unique index uq_origins_origin_key"
                }
            ]
        });

        sanitize_value_for_privacy(PrivacyMode::Mask, &mut payload);

        assert_eq!(payload["seed"], json!("<redacted-query>"));
        assert_eq!(
            payload["items"][0]["anchor"]["path"],
            json!("<masked:origin_service.rs>")
        );
        assert_eq!(
            payload["items"][0]["signature"],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["items"][0]["anchor"]["symbol"],
            json!("<redacted-content>")
        );
        assert_eq!(payload["items"][0]["body"], json!("<redacted-content>"));
        assert_eq!(
            payload["variants"][0]["route"][0]["path"],
            json!("<masked:origin_service.rs>")
        );
        assert_eq!(
            payload["variants"][0]["route"][0]["evidence"],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["variants"][0]["related_tests"][0],
            json!("<masked:test_origin_flow.py>")
        );
        assert_eq!(
            payload["variants"][0]["gaps"][0],
            json!("<redacted-content>")
        );
        assert_eq!(payload["shared_evidence"][0], json!("<redacted-content>"));
        assert_eq!(payload["missing_evidence"][0], json!("<redacted-content>"));
        assert_eq!(payload["unknowns"][0], json!("<redacted-content>"));
        assert_eq!(
            payload["recommended_followups"][0],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["divergence_signals"][0]["summary"],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["constraints"][0]["path"],
            json!("<masked:001_create_origins.sql>")
        );
        assert_eq!(
            payload["constraints"][0]["excerpt"],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["constraints"][0]["normalized_key"],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["constraints"][0]["source_path"],
            json!("<masked:001_create_origins.sql>")
        );
        assert_eq!(
            payload["constraints"][0]["normalized_text"],
            json!("<redacted-content>")
        );
    }

    #[test]
    fn json_sanitization_scrubs_embedded_investigation_summary_and_hints() {
        let mut payload = json!({
            "investigation_summary": {
                "concept_cluster": {
                    "top_variants": [
                        {
                            "path": "src/services/origin_service.rs",
                            "symbol": "resolve_origin",
                            "confidence": 0.91
                        }
                    ]
                },
                "constraint_evidence": {
                    "normalized_keys": ["index_constraint:index_declaration:create unique index uq_origins_origin_key"]
                },
                "divergence": {
                    "recommended_followups": ["inspect schema backing"]
                }
            },
            "investigation_hints": {
                "top_variants": [
                    {
                        "path": "src/services/origin_service.rs",
                        "symbol": "resolve_origin",
                        "confidence": 0.88
                    }
                ],
                "constraint_keys": ["model_constraint:model:origin_key"],
                "followups": ["compare route validators"]
            }
        });

        sanitize_value_for_privacy(PrivacyMode::Mask, &mut payload);

        assert_eq!(
            payload["investigation_summary"]["concept_cluster"]["top_variants"][0]["path"],
            json!("<masked:origin_service.rs>")
        );
        assert_eq!(
            payload["investigation_summary"]["concept_cluster"]["top_variants"][0]["symbol"],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["investigation_summary"]["constraint_evidence"]["normalized_keys"][0],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["investigation_summary"]["divergence"]["recommended_followups"][0],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["investigation_hints"]["top_variants"][0]["path"],
            json!("<masked:origin_service.rs>")
        );
        assert_eq!(
            payload["investigation_hints"]["top_variants"][0]["symbol"],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["investigation_hints"]["constraint_keys"][0],
            json!("<redacted-content>")
        );
        assert_eq!(
            payload["investigation_hints"]["followups"][0],
            json!("<redacted-content>")
        );
    }
}
