use crate::artifact_fingerprint::CURRENT_ARTIFACT_FINGERPRINT_VERSION;
use crate::graph::{CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION, CURRENT_GRAPH_FINGERPRINT_VERSION};
use crate::quality::CURRENT_QUALITY_RULESET_VERSION;

use super::ExistingFileState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::engine) enum FileStateSection {
    Artifacts,
    Graph,
    GraphEdges,
    Quality,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in crate::engine) struct StateCompletenessReport {
    pub(in crate::engine) incomplete_sections: Vec<FileStateSection>,
}

impl StateCompletenessReport {
    pub(in crate::engine) fn is_complete(&self) -> bool {
        self.incomplete_sections.is_empty()
    }

    #[cfg(test)]
    pub(in crate::engine) fn contains(&self, section: FileStateSection) -> bool {
        self.incomplete_sections.contains(&section)
    }
}

pub(in crate::engine) fn state_completeness_report(
    state: &ExistingFileState,
) -> StateCompletenessReport {
    let mut incomplete_sections = Vec::new();
    for (section, check) in FILE_STATE_SECTION_CHECKS {
        if !check(state) {
            incomplete_sections.push(*section);
        }
    }
    StateCompletenessReport {
        incomplete_sections,
    }
}

type FileStateSectionCheck = fn(&ExistingFileState) -> bool;

const FILE_STATE_SECTION_CHECKS: &[(FileStateSection, FileStateSectionCheck)] = &[
    (FileStateSection::Artifacts, artifact_section_complete),
    (FileStateSection::Graph, graph_section_complete),
    (FileStateSection::GraphEdges, graph_edge_section_complete),
    (FileStateSection::Quality, quality_section_complete),
];

fn artifact_section_complete(state: &ExistingFileState) -> bool {
    state.artifact_fingerprint_version == Some(CURRENT_ARTIFACT_FINGERPRINT_VERSION)
        && state.fts_sample_hash.as_deref() == state.actual_fts_sample_hash.as_deref()
        && state.chunk_manifest_count == Some(state.actual_chunk_manifest_count)
        && state.chunk_manifest_hash.as_deref() == Some(state.actual_chunk_manifest_hash.as_str())
        && state.chunk_embedding_count == Some(state.actual_chunk_embedding_count)
        && state.chunk_embedding_hash.as_deref() == Some(state.actual_chunk_embedding_hash.as_str())
        && state.semantic_vector_hash.as_deref() == state.actual_semantic_vector_hash.as_deref()
        && state.ann_bucket_count == Some(state.actual_ann_bucket_count)
        && state.ann_bucket_hash.as_deref() == Some(state.actual_ann_bucket_hash.as_str())
}

fn graph_section_complete(state: &ExistingFileState) -> bool {
    state.graph_symbol_count == Some(state.actual_graph_symbol_count)
        && state.graph_ref_count == Some(state.actual_graph_ref_count)
        && state.graph_module_dep_count == Some(state.actual_graph_module_dep_count)
        && state.graph_content_hash.as_deref() == Some(state.actual_graph_content_hash.as_str())
        && state.graph_fingerprint_version == Some(CURRENT_GRAPH_FINGERPRINT_VERSION)
}

fn graph_edge_section_complete(state: &ExistingFileState) -> bool {
    state.graph_edge_out_count == Some(state.actual_graph_edge_out_count)
        && state.graph_edge_in_count == Some(state.actual_graph_edge_in_count)
        && state.graph_edge_hash.as_deref() == Some(state.actual_graph_edge_hash.as_str())
        && state.graph_edge_fingerprint_version == Some(CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION)
}

fn quality_section_complete(state: &ExistingFileState) -> bool {
    state.quality_ruleset_version == Some(CURRENT_QUALITY_RULESET_VERSION)
        && state.quality_violation_count == Some(state.actual_quality_violation_count)
        && state.quality_violation_hash.as_deref()
            == Some(state.actual_quality_violation_hash.as_str())
}
