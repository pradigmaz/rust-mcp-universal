use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GraphSourceKind {
    Java,
    Rust,
    Python,
    JavaScript { jsx: bool },
    TypeScript { jsx: bool },
}

impl GraphSourceKind {
    pub(crate) fn language_label(self) -> &'static str {
        match self {
            Self::Java => "java",
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript { .. } => "javascript",
            Self::TypeScript { .. } => "typescript",
        }
    }

    pub(crate) fn include_type_symbols(self) -> bool {
        matches!(self, Self::TypeScript { .. })
    }

    pub(crate) fn synthetic_path(self) -> &'static str {
        match self {
            Self::Java => "graph.java",
            Self::Rust => "graph.rs",
            Self::Python => "graph.py",
            Self::JavaScript { jsx: false } => "graph.js",
            Self::JavaScript { jsx: true } => "graph.jsx",
            Self::TypeScript { jsx: false } => "graph.ts",
            Self::TypeScript { jsx: true } => "graph.tsx",
        }
    }
}

pub(crate) fn graph_source_kind_from_language(language: &str) -> GraphSourceKind {
    match language {
        "java" => GraphSourceKind::Java,
        "rust" => GraphSourceKind::Rust,
        "python" => GraphSourceKind::Python,
        "javascript" => GraphSourceKind::JavaScript { jsx: false },
        "typescript" => GraphSourceKind::TypeScript { jsx: false },
        _ => GraphSourceKind::Rust,
    }
}

pub(crate) fn supports_graph_extraction(language: &str) -> bool {
    matches!(
        language,
        "java" | "rust" | "python" | "javascript" | "typescript"
    )
}

pub(crate) fn graph_source_kind_for_path(path: &Path, language: &str) -> Option<GraphSourceKind> {
    match path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("java") => Some(GraphSourceKind::Java),
        Some("jsx") => Some(GraphSourceKind::JavaScript { jsx: true }),
        Some("tsx") => Some(GraphSourceKind::TypeScript { jsx: true }),
        Some("ts") => Some(GraphSourceKind::TypeScript { jsx: false }),
        Some("js") | Some("mjs") | Some("cjs") => Some(GraphSourceKind::JavaScript { jsx: false }),
        _ if supports_graph_extraction(language) => Some(graph_source_kind_from_language(language)),
        _ => None,
    }
}
