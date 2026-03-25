use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use super::super::types::PassResult;
use crate::engine::storage;
use crate::index_scope::IndexScope;
use crate::quality::CURRENT_QUALITY_RULESET_VERSION;
use crate::utils::{ProjectIgnoreMatcher, is_probably_ignored, normalize_path};

pub(super) fn resolve_scoped_path(
    path: &Path,
    project_root: &Path,
    scope: &IndexScope,
    ignore_matcher: &ProjectIgnoreMatcher,
    pass_result: &mut PassResult,
) -> Option<String> {
    let relative = path.strip_prefix(project_root).ok()?;
    if is_probably_ignored(relative) || ignore_matcher.is_ignored(relative, path.is_dir()) {
        return None;
    }
    let rel_text = normalize_path(relative);
    if !scope.allows(&rel_text) {
        return None;
    }
    pass_result.present_paths.insert(rel_text.clone());
    Some(rel_text)
}

pub(super) fn is_unchanged(
    existing_files: &HashMap<String, storage::ExistingFileState>,
    rel_text: &str,
    sha256: &str,
) -> bool {
    existing_files
        .get(rel_text)
        .is_some_and(|state| state.sha256 == sha256 && is_state_complete(state))
}

pub(crate) fn should_refresh_candidate(
    changed_since_unix_ms: Option<i64>,
    existing_state: Option<&storage::ExistingFileState>,
    current_mtime_unix_ms: Option<i64>,
) -> bool {
    let Some(changed_since_unix_ms) = changed_since_unix_ms else {
        return true;
    };
    let Some(existing_state) = existing_state else {
        return true;
    };
    if existing_state.source_mtime_unix_ms.is_none() || !is_state_complete(existing_state) {
        return true;
    }
    current_mtime_unix_ms.is_none_or(|value| value >= changed_since_unix_ms)
}

pub(crate) fn offset_datetime_to_unix_ms(value: time::OffsetDateTime) -> i64 {
    clamp_i128_to_i64(value.unix_timestamp_nanos() / 1_000_000)
}

pub(crate) fn system_time_to_unix_ms(value: SystemTime) -> i64 {
    offset_datetime_to_unix_ms(time::OffsetDateTime::from(value))
}

pub(crate) fn is_state_complete(state: &storage::ExistingFileState) -> bool {
    storage::state_completeness_report(state).is_complete()
}

pub(crate) fn is_quality_state_complete(state: &storage::ExistingQualityState) -> bool {
    state.quality_ruleset_version == CURRENT_QUALITY_RULESET_VERSION
        && state.quality_metric_count == state.actual_quality_metric_count
        && state.quality_metric_hash == state.actual_quality_metric_hash
        && state.quality_violation_count == state.actual_quality_violation_count
        && state.quality_violation_hash == state.actual_quality_violation_hash
}

fn clamp_i128_to_i64(value: i128) -> i64 {
    if value > i128::from(i64::MAX) {
        i64::MAX
    } else if value < i128::from(i64::MIN) {
        i64::MIN
    } else {
        value as i64
    }
}

#[cfg(test)]
mod tests {
    use super::{is_quality_state_complete, is_state_complete};
    use crate::artifact_fingerprint::CURRENT_ARTIFACT_FINGERPRINT_VERSION;
    use crate::engine::storage::{
        ExistingFileState, ExistingQualityState, FileStateSection, state_completeness_report,
    };
    use crate::graph::{CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION, CURRENT_GRAPH_FINGERPRINT_VERSION};
    use crate::quality::{CURRENT_QUALITY_RULESET_VERSION, violations_hash};

    fn complete_state() -> ExistingFileState {
        ExistingFileState {
            sha256: "abc".to_string(),
            source_mtime_unix_ms: Some(1),
            artifact_fingerprint_version: Some(CURRENT_ARTIFACT_FINGERPRINT_VERSION),
            fts_sample_hash: Some("fts".to_string()),
            chunk_manifest_count: Some(1),
            chunk_manifest_hash: Some("chunk-manifest".to_string()),
            chunk_embedding_count: Some(1),
            chunk_embedding_hash: Some("chunk-embedding".to_string()),
            semantic_vector_hash: Some("semantic-vector".to_string()),
            ann_bucket_count: Some(1),
            ann_bucket_hash: Some("ann-bucket".to_string()),
            graph_symbol_count: Some(0),
            graph_ref_count: Some(0),
            graph_module_dep_count: Some(0),
            graph_content_hash: Some("hash".to_string()),
            graph_fingerprint_version: Some(CURRENT_GRAPH_FINGERPRINT_VERSION),
            graph_edge_out_count: Some(0),
            graph_edge_in_count: Some(0),
            graph_edge_hash: Some("graph-edge-hash".to_string()),
            graph_edge_fingerprint_version: Some(CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION),
            actual_fts_sample_hash: Some("fts".to_string()),
            actual_chunk_manifest_count: 1,
            actual_chunk_manifest_hash: "chunk-manifest".to_string(),
            actual_chunk_embedding_count: 1,
            actual_chunk_embedding_hash: "chunk-embedding".to_string(),
            actual_semantic_vector_hash: Some("semantic-vector".to_string()),
            actual_ann_bucket_count: 1,
            actual_ann_bucket_hash: "ann-bucket".to_string(),
            actual_graph_symbol_count: 0,
            actual_graph_ref_count: 0,
            actual_graph_module_dep_count: 0,
            actual_graph_content_hash: "hash".to_string(),
            actual_graph_edge_out_count: 0,
            actual_graph_edge_in_count: 0,
            actual_graph_edge_hash: "graph-edge-hash".to_string(),
        }
    }

    fn complete_quality_state() -> ExistingQualityState {
        ExistingQualityState {
            source_mtime_unix_ms: Some(1),
            quality_ruleset_version: CURRENT_QUALITY_RULESET_VERSION,
            quality_metric_count: 1,
            quality_metric_hash: "metrics".to_string(),
            quality_violation_count: 0,
            quality_violation_hash: violations_hash(&[]),
            quality_suppressed_violation_count: 0,
            quality_suppressed_violation_hash: crate::quality::suppressed_violations_hash(&[]),
            actual_quality_metric_count: 1,
            actual_quality_metric_hash: "metrics".to_string(),
            actual_quality_violation_count: 0,
            actual_quality_violation_hash: violations_hash(&[]),
            actual_quality_suppressed_violation_count: 0,
            actual_quality_suppressed_violation_hash: crate::quality::suppressed_violations_hash(
                &[],
            ),
        }
    }

    #[test]
    fn state_complete_accepts_legitimate_zero_graph_output() {
        assert!(is_state_complete(&complete_state()));
    }

    #[test]
    fn state_complete_rejects_missing_graph_backfill_metadata() {
        let mut state = complete_state();
        state.graph_ref_count = None;
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_complete_rejects_missing_artifact_backfill_metadata() {
        let mut state = complete_state();
        state.chunk_manifest_hash = None;
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_complete_rejects_artifact_hash_mismatch() {
        let mut state = complete_state();
        state.actual_chunk_embedding_hash = "other".to_string();
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_complete_rejects_artifact_fingerprint_version_mismatch() {
        let mut state = complete_state();
        state.artifact_fingerprint_version = Some(CURRENT_ARTIFACT_FINGERPRINT_VERSION - 1);
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_complete_rejects_graph_count_mismatch() {
        let mut state = complete_state();
        state.actual_graph_symbol_count = 1;
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_complete_rejects_missing_graph_hash_metadata() {
        let mut state = complete_state();
        state.graph_content_hash = None;
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_complete_rejects_graph_hash_mismatch() {
        let mut state = complete_state();
        state.actual_graph_content_hash = "other".to_string();
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_complete_rejects_graph_fingerprint_version_mismatch() {
        let mut state = complete_state();
        state.graph_fingerprint_version = Some(CURRENT_GRAPH_FINGERPRINT_VERSION - 1);
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_complete_rejects_graph_edge_count_mismatch() {
        let mut state = complete_state();
        state.actual_graph_edge_out_count = 1;
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_complete_rejects_graph_edge_hash_mismatch() {
        let mut state = complete_state();
        state.actual_graph_edge_hash = "other".to_string();
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_complete_rejects_graph_edge_fingerprint_version_mismatch() {
        let mut state = complete_state();
        state.graph_edge_fingerprint_version = Some(CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION - 1);
        assert!(!is_state_complete(&state));
    }

    #[test]
    fn state_completeness_report_surfaces_failed_sections() {
        let mut state = complete_state();
        state.graph_edge_hash = None;

        let report = state_completeness_report(&state);
        assert!(!report.is_complete());
        assert!(report.contains(FileStateSection::GraphEdges));
    }

    #[test]
    fn quality_state_complete_accepts_matching_quality_snapshot() {
        assert!(is_quality_state_complete(&complete_quality_state()));
    }

    #[test]
    fn quality_state_complete_rejects_ruleset_version_mismatch() {
        let mut state = complete_quality_state();
        state.quality_ruleset_version = CURRENT_QUALITY_RULESET_VERSION - 1;
        assert!(!is_quality_state_complete(&state));
    }

    #[test]
    fn quality_state_complete_rejects_metric_hash_mismatch() {
        let mut state = complete_quality_state();
        state.actual_quality_metric_hash = "other".to_string();
        assert!(!is_quality_state_complete(&state));
    }
}
