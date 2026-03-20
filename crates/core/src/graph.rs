use std::path::Path;

mod common;
mod fingerprint;
mod javascript;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphSourceKind {
    Rust,
    Python,
    JavaScript { jsx: bool },
    TypeScript { jsx: bool },
}

impl GraphSourceKind {
    pub(crate) fn language_label(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript { .. } => "javascript",
            Self::TypeScript { .. } => "typescript",
        }
    }

    pub(crate) fn include_type_symbols(self) -> bool {
        matches!(self, Self::TypeScript { .. })
    }

    fn synthetic_path(self) -> &'static str {
        match self {
            Self::Rust => "graph.rs",
            Self::Python => "graph.py",
            Self::JavaScript { jsx: false } => "graph.js",
            Self::JavaScript { jsx: true } => "graph.jsx",
            Self::TypeScript { jsx: false } => "graph.ts",
            Self::TypeScript { jsx: true } => "graph.tsx",
        }
    }
}

fn graph_source_kind_from_language(language: &str) -> GraphSourceKind {
    match language {
        "rust" => GraphSourceKind::Rust,
        "python" => GraphSourceKind::Python,
        "javascript" => GraphSourceKind::JavaScript { jsx: false },
        "typescript" => GraphSourceKind::TypeScript { jsx: false },
        _ => GraphSourceKind::Rust,
    }
}

pub(crate) fn supports_graph_extraction(language: &str) -> bool {
    matches!(language, "rust" | "python" | "javascript" | "typescript")
}

pub(crate) fn graph_source_kind_for_path(path: &Path, language: &str) -> Option<GraphSourceKind> {
    match path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("jsx") => Some(GraphSourceKind::JavaScript { jsx: true }),
        Some("tsx") => Some(GraphSourceKind::TypeScript { jsx: true }),
        Some("ts") => Some(GraphSourceKind::TypeScript { jsx: false }),
        Some("js") | Some("mjs") | Some("cjs") => Some(GraphSourceKind::JavaScript { jsx: false }),
        _ if supports_graph_extraction(language) => Some(graph_source_kind_from_language(language)),
        _ => None,
    }
}

pub(crate) fn extract_graph_for_kind(kind: GraphSourceKind, source: &str) -> GraphExtraction {
    match kind {
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
