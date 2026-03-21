use super::{IndexedQualityMetrics, QualityCandidateFacts, QualityEvaluation, QualitySnapshot};
use super::rules::evaluate_rules;

pub(crate) fn build_indexed_quality_facts(
    rel_path: &str,
    language: &str,
    size_bytes: u64,
    source_mtime_unix_ms: Option<i64>,
    full_text: &str,
) -> QualityCandidateFacts {
    super::metrics::build_indexed_quality_facts(
        rel_path,
        language,
        size_bytes,
        source_mtime_unix_ms,
        full_text,
    )
}

pub(crate) fn build_oversize_quality_facts(
    rel_path: &str,
    language: &str,
    size_bytes: u64,
    source_mtime_unix_ms: Option<i64>,
) -> QualityCandidateFacts {
    super::metrics::build_oversize_quality_facts(
        rel_path,
        language,
        size_bytes,
        source_mtime_unix_ms,
    )
}

pub(crate) fn evaluate_quality(
    facts: &QualityCandidateFacts,
    indexed_metrics: &IndexedQualityMetrics,
) -> QualityEvaluation {
    let evaluation = evaluate_rules(facts, indexed_metrics);
    QualityEvaluation {
        snapshot: QualitySnapshot {
            size_bytes: facts.size_bytes,
            total_lines: facts.total_lines,
            non_empty_lines: facts.non_empty_lines,
            import_count: facts.import_count,
            quality_mode: facts.quality_mode,
            metrics: evaluation.metrics,
            violations: evaluation.violations,
        },
        had_rule_errors: evaluation.had_rule_errors,
        last_error_rule_id: evaluation.last_error_rule_id,
    }
}
