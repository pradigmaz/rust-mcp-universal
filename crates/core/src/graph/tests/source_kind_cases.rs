use std::path::Path;

use super::super::{
    GraphSourceKind, extract_graph, graph_source_kind_for_path, supports_graph_extraction,
};

#[test]
fn graph_source_kind_uses_path_extensions_for_jsx_and_tsx() {
    assert_eq!(
        graph_source_kind_for_path(Path::new("src/app.java"), "java"),
        Some(GraphSourceKind::Java)
    );
    assert_eq!(
        graph_source_kind_for_path(Path::new("src/app.tsx"), "typescript"),
        Some(GraphSourceKind::TypeScript { jsx: true })
    );
    assert_eq!(
        graph_source_kind_for_path(Path::new("src/app.jsx"), "javascript"),
        Some(GraphSourceKind::JavaScript { jsx: true })
    );
    assert_eq!(
        graph_source_kind_for_path(Path::new("src/app.ts"), "javascript"),
        Some(GraphSourceKind::TypeScript { jsx: false })
    );
}

#[test]
fn graph_source_kind_rejects_non_code_paths_and_languages() {
    assert_eq!(
        graph_source_kind_for_path(Path::new("docs/guide.md"), "markdown"),
        None
    );
    assert_eq!(
        graph_source_kind_for_path(Path::new("config/app.toml"), "toml"),
        None
    );
    assert!(!supports_graph_extraction("markdown"));
    assert!(!supports_graph_extraction("json"));
}

#[test]
fn extract_graph_returns_empty_for_non_code_languages() {
    let graph = extract_graph("markdown", "# heading\nchunk visibility\n");
    assert!(graph.symbols.is_empty());
    assert!(graph.deps.is_empty());
    assert!(graph.refs.is_empty());
}
