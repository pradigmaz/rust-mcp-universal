#[path = "javascript_ast.rs"]
mod ast;
#[path = "javascript_heuristic.rs"]
mod heuristic;

use super::{GraphExtraction, GraphSourceKind};

pub(super) fn extract_javascript_ast_first(kind: GraphSourceKind, source: &str) -> GraphExtraction {
    ast::extract_javascript_ast_first(kind, source)
}
