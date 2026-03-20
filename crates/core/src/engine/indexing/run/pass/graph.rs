use std::path::Path;

use crate::graph::{
    CURRENT_GRAPH_FINGERPRINT_VERSION, GraphExtraction, empty_graph_content_hash,
    extract_graph_for_kind, graph_content_hash, graph_source_kind_for_path,
};

pub(super) struct GraphArtifacts {
    pub(super) graph: GraphExtraction,
    pub(super) graph_symbol_count: i64,
    pub(super) graph_module_dep_count: i64,
    pub(super) graph_ref_count: i64,
    pub(super) graph_content_hash: String,
    pub(super) graph_fingerprint_version: i64,
}

pub(super) fn build_graph_artifacts(
    path: &Path,
    language: &str,
    full_text: &str,
) -> GraphArtifacts {
    let (graph, graph_content_hash) = graph_source_kind_for_path(path, language)
        .map(|graph_kind| {
            let graph = extract_graph_for_kind(graph_kind, full_text);
            let hash = graph_content_hash(graph_kind.language_label(), &graph);
            (graph, hash)
        })
        .unwrap_or_else(|| (GraphExtraction::default(), empty_graph_content_hash()));
    GraphArtifacts {
        graph_symbol_count: i64::try_from(graph.symbols.len()).unwrap_or(i64::MAX),
        graph_module_dep_count: i64::try_from(graph.deps.len()).unwrap_or(i64::MAX),
        graph_ref_count: i64::try_from(graph.refs.len()).unwrap_or(i64::MAX),
        graph_content_hash,
        graph_fingerprint_version: CURRENT_GRAPH_FINGERPRINT_VERSION,
        graph,
    }
}
