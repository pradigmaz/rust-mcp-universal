use std::path::Path;

use crate::model::{QualityMode, QualityViolationEntry};
use crate::utils::hash_bytes;

pub(crate) const QUALITY_RULESET_ID: &str = "file-size-v1";
pub(crate) const CURRENT_QUALITY_RULESET_VERSION: i64 = 1;

const MAX_NON_EMPTY_LINES_DEFAULT: i64 = 300;
const MAX_NON_EMPTY_LINES_TEST: i64 = 500;
const MAX_NON_EMPTY_LINES_CONFIG: i64 = 100;
const MAX_SIZE_BYTES: i64 = 262_144;
const MAX_IMPORT_COUNT: i64 = 20;

#[derive(Debug, Clone)]
pub(crate) struct QualitySnapshot {
    pub(crate) size_bytes: i64,
    pub(crate) total_lines: Option<i64>,
    pub(crate) non_empty_lines: Option<i64>,
    pub(crate) import_count: Option<i64>,
    pub(crate) quality_mode: QualityMode,
    pub(crate) violations: Vec<QualityViolationEntry>,
}

pub(crate) fn evaluate_indexed_quality(
    rel_path: &str,
    language: &str,
    size_bytes: u64,
    full_text: &str,
) -> QualitySnapshot {
    let total_lines = total_lines(full_text);
    let non_empty_lines = non_empty_lines(full_text);
    let import_count = import_count(language, full_text);
    let file_kind = classify_file_kind(rel_path);
    let mut violations = Vec::new();
    push_size_violation(
        &mut violations,
        i64::try_from(size_bytes).unwrap_or(i64::MAX),
    );

    let line_limit = match file_kind {
        FileKind::Default => Some(("max_non_empty_lines_default", MAX_NON_EMPTY_LINES_DEFAULT)),
        FileKind::Test => Some(("max_non_empty_lines_test", MAX_NON_EMPTY_LINES_TEST)),
        FileKind::Config => Some(("max_non_empty_lines_config", MAX_NON_EMPTY_LINES_CONFIG)),
    };
    if let Some((rule_id, threshold_value)) = line_limit {
        push_threshold_violation(
            &mut violations,
            rule_id,
            non_empty_lines,
            threshold_value,
            "non-empty line count exceeds the allowed threshold",
        );
    }

    if supports_import_rule(language) {
        push_threshold_violation(
            &mut violations,
            "max_import_count",
            import_count,
            MAX_IMPORT_COUNT,
            "import count exceeds the allowed threshold",
        );
    }

    QualitySnapshot {
        size_bytes: i64::try_from(size_bytes).unwrap_or(i64::MAX),
        total_lines: Some(total_lines),
        non_empty_lines: Some(non_empty_lines),
        import_count: Some(import_count),
        quality_mode: QualityMode::Indexed,
        violations,
    }
}

pub(crate) fn evaluate_oversize_quality(size_bytes: u64) -> QualitySnapshot {
    let size_bytes = i64::try_from(size_bytes).unwrap_or(i64::MAX);
    let mut violations = Vec::new();
    push_size_violation(&mut violations, size_bytes);
    QualitySnapshot {
        size_bytes,
        total_lines: None,
        non_empty_lines: None,
        import_count: None,
        quality_mode: QualityMode::QualityOnlyOversize,
        violations,
    }
}

pub(crate) fn violations_hash(violations: &[QualityViolationEntry]) -> String {
    let mut bytes = Vec::new();
    for violation in violations {
        bytes.extend_from_slice(violation.rule_id.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(violation.actual_value.to_string().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(violation.threshold_value.to_string().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(violation.message.as_bytes());
        bytes.push(b'\n');
    }
    hash_bytes(&bytes)
}

fn push_size_violation(violations: &mut Vec<QualityViolationEntry>, size_bytes: i64) {
    push_threshold_violation(
        violations,
        "max_size_bytes",
        size_bytes,
        MAX_SIZE_BYTES,
        "file size exceeds the allowed threshold",
    );
}

fn push_threshold_violation(
    violations: &mut Vec<QualityViolationEntry>,
    rule_id: &str,
    actual_value: i64,
    threshold_value: i64,
    message: &str,
) {
    if actual_value > threshold_value {
        violations.push(QualityViolationEntry {
            rule_id: rule_id.to_string(),
            actual_value,
            threshold_value,
            message: message.to_string(),
        });
    }
}

fn total_lines(full_text: &str) -> i64 {
    if full_text.is_empty() {
        0
    } else {
        i64::try_from(full_text.lines().count()).unwrap_or(i64::MAX)
    }
}

fn non_empty_lines(full_text: &str) -> i64 {
    i64::try_from(
        full_text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
    )
    .unwrap_or(i64::MAX)
}

fn import_count(language: &str, full_text: &str) -> i64 {
    let count = full_text
        .lines()
        .filter(|line| is_import_like_line(language, line.trim_start()))
        .count();
    i64::try_from(count).unwrap_or(i64::MAX)
}

fn supports_import_rule(language: &str) -> bool {
    matches!(
        language,
        "rust" | "python" | "javascript" | "jsx" | "mjs" | "cjs" | "typescript" | "tsx"
    )
}

fn is_import_like_line(language: &str, trimmed: &str) -> bool {
    match language {
        "rust" => trimmed.starts_with("use ") || trimmed.starts_with("pub use "),
        "python" => trimmed.starts_with("import ") || trimmed.starts_with("from "),
        "javascript" | "jsx" | "mjs" | "cjs" | "typescript" | "tsx" => {
            trimmed.starts_with("import ")
                || (trimmed.starts_with("export ")
                    && trimmed.contains(" from ")
                    && !trimmed.starts_with("export default"))
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileKind {
    Default,
    Test,
    Config,
}

fn classify_file_kind(rel_path: &str) -> FileKind {
    let rel_path_lower = rel_path.to_ascii_lowercase();
    if rel_path_lower.starts_with("tests/")
        || rel_path_lower.contains("/tests/")
        || rel_path_lower.starts_with("benches/")
        || rel_path_lower.contains("/benches/")
        || rel_path_lower.starts_with("examples/")
        || rel_path_lower.contains("/examples/")
        || rel_path_lower.contains(".test.")
        || rel_path_lower.contains(".spec.")
        || rel_path_lower.contains("_test.")
    {
        return FileKind::Test;
    }

    let path = Path::new(rel_path);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if matches!(file_name, "Cargo.lock" | ".gitignore" | ".editorconfig")
        || matches!(extension, "toml" | "json" | "yaml" | "yml" | "ini" | "cfg")
    {
        return FileKind::Config;
    }

    FileKind::Default
}

#[cfg(test)]
mod tests {
    use super::{
        CURRENT_QUALITY_RULESET_VERSION, QUALITY_RULESET_ID, QualitySnapshot, classify_file_kind,
        evaluate_indexed_quality, evaluate_oversize_quality,
    };

    #[test]
    fn quality_constants_are_stable() {
        assert_eq!(QUALITY_RULESET_ID, "file-size-v1");
        assert_eq!(CURRENT_QUALITY_RULESET_VERSION, 1);
    }

    #[test]
    fn classify_file_kind_detects_test_and_config_paths() {
        assert!(matches!(
            classify_file_kind("src/tests/mod.rs"),
            super::FileKind::Test
        ));
        assert!(matches!(
            classify_file_kind("config/app.toml"),
            super::FileKind::Config
        ));
        assert!(matches!(
            classify_file_kind("src/lib.rs"),
            super::FileKind::Default
        ));
    }

    #[test]
    fn indexed_quality_counts_non_empty_lines_and_imports() {
        let snapshot = evaluate_indexed_quality(
            "src/lib.rs",
            "rust",
            64,
            "use std::fmt;\n\npub use std::io;\nfn main() {}\n",
        );
        assert_eq!(snapshot.total_lines, Some(4));
        assert_eq!(snapshot.non_empty_lines, Some(3));
        assert_eq!(snapshot.import_count, Some(2));
    }

    #[test]
    fn oversize_quality_only_emits_size_rule() {
        let snapshot: QualitySnapshot = evaluate_oversize_quality(300_000);
        assert!(snapshot.total_lines.is_none());
        assert!(snapshot.non_empty_lines.is_none());
        assert!(snapshot.import_count.is_none());
        assert_eq!(snapshot.violations.len(), 1);
        assert_eq!(snapshot.violations[0].rule_id, "max_size_bytes");
    }
}
