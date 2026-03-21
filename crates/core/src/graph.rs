mod common;
mod fingerprint;
mod java;
mod java_support;
mod javascript;
mod policy;
mod python;
mod rust;
#[cfg(test)]
mod tests;

pub(crate) const CURRENT_GRAPH_FINGERPRINT_VERSION: i64 = 3;
pub(crate) const CURRENT_GRAPH_EDGE_FINGERPRINT_VERSION: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphSymbol {
    pub name: String,
    pub kind: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GraphRef {
    pub symbol: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct GraphExtraction {
    pub symbols: Vec<GraphSymbol>,
    pub deps: Vec<String>,
    pub refs: Vec<GraphRef>,
}

pub(crate) use fingerprint::{
    GraphEdgeFingerprintBuilder, GraphFingerprintBuilder, empty_graph_content_hash,
    empty_graph_edge_content_hash, graph_content_hash,
};
pub(crate) use policy::{
    GraphSourceKind, graph_source_kind_for_path, graph_source_kind_from_language,
    supports_graph_extraction,
};

pub(crate) fn extract_graph_for_kind(kind: GraphSourceKind, source: &str) -> GraphExtraction {
    match kind {
        GraphSourceKind::Java => java::extract_java_heuristic(source),
        GraphSourceKind::Rust => rust::extract_rust_heuristic(source),
        GraphSourceKind::Python => python::extract_python_heuristic(source),
        GraphSourceKind::JavaScript { .. } | GraphSourceKind::TypeScript { .. } => {
            javascript::extract_javascript_ast_first(kind, source)
        }
    }
}

#[allow(dead_code)]
pub fn extract_graph(language: &str, source: &str) -> GraphExtraction {
    if supports_graph_extraction(language) {
        extract_graph_for_kind(graph_source_kind_from_language(language), source)
    } else {
        GraphExtraction::default()
    }
}
