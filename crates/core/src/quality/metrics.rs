use std::path::Path;

use super::{QualityCandidateFacts, QualityMetricEntry};
use crate::model::QualityMode;
use crate::utils::hash_bytes;

pub(super) const MAX_NON_EMPTY_LINES_DEFAULT: i64 = 300;
pub(super) const MAX_NON_EMPTY_LINES_TEST: i64 = 500;
pub(super) const MAX_NON_EMPTY_LINES_CONFIG: i64 = 100;
pub(super) const MAX_SIZE_BYTES: i64 = 262_144;
pub(super) const MAX_IMPORT_COUNT: i64 = 20;
pub(super) const MAX_LINE_LENGTH: i64 = 140;
pub(super) const MAX_SYMBOL_COUNT_PER_FILE: i64 = 80;
pub(super) const MAX_REF_COUNT_PER_FILE: i64 = 200;
pub(super) const MAX_MODULE_DEP_COUNT_PER_FILE: i64 = 40;
pub(super) const MAX_GRAPH_EDGE_OUT_COUNT: i64 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileKind {
    Default,
    Test,
    Config,
}

pub(crate) fn build_indexed_quality_facts(
    rel_path: &str,
    language: &str,
    size_bytes: u64,
    _source_mtime_unix_ms: Option<i64>,
    full_text: &str,
) -> QualityCandidateFacts {
    QualityCandidateFacts {
        size_bytes: i64::try_from(size_bytes).unwrap_or(i64::MAX),
        total_lines: Some(total_lines(full_text)),
        non_empty_lines: Some(non_empty_lines(full_text)),
        import_count: Some(import_count(language, full_text)),
        max_line_length: Some(max_line_length(full_text)),
        quality_mode: QualityMode::Indexed,
        file_kind: classify_file_kind(rel_path),
    }
}

pub(crate) fn build_oversize_quality_facts(
    rel_path: &str,
    _language: &str,
    size_bytes: u64,
    _source_mtime_unix_ms: Option<i64>,
) -> QualityCandidateFacts {
    QualityCandidateFacts {
        size_bytes: i64::try_from(size_bytes).unwrap_or(i64::MAX),
        total_lines: None,
        non_empty_lines: None,
        import_count: None,
        max_line_length: None,
        quality_mode: QualityMode::QualityOnlyOversize,
        file_kind: classify_file_kind(rel_path),
    }
}

pub(crate) fn quality_metrics_hash(metrics: &[QualityMetricEntry]) -> String {
    let mut bytes = Vec::new();
    for metric in metrics {
        bytes.extend_from_slice(metric.metric_id.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(metric.metric_value.to_string().as_bytes());
        bytes.push(b'\n');
    }
    hash_bytes(&bytes)
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

fn max_line_length(full_text: &str) -> i64 {
    full_text
        .lines()
        .map(|line| i64::try_from(line.chars().count()).unwrap_or(i64::MAX))
        .max()
        .unwrap_or(0)
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
