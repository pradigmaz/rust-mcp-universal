use super::super::{GraphRef, GraphSymbol, extract_graph};

#[test]
fn rust_extractor_captures_common_item_modifiers_and_positions() {
    let source = r#"pub(crate) async fn alpha() {}
pub struct Widget;
pub enum Kind { One }
pub trait Runner {}
pub mod nested;
impl Widget {
    pub(crate) unsafe fn build() {}
}
"#;

    let graph = extract_graph("rust", source);
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "alpha".to_string(),
        kind: "function".to_string(),
        line: Some(1),
        column: Some(21),
    }));
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "Widget".to_string(),
        kind: "struct".to_string(),
        line: Some(2),
        column: Some(12),
    }));
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "Kind".to_string(),
        kind: "enum".to_string(),
        line: Some(3),
        column: Some(10),
    }));
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "Runner".to_string(),
        kind: "trait".to_string(),
        line: Some(4),
        column: Some(11),
    }));
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "nested".to_string(),
        kind: "module".to_string(),
        line: Some(5),
        column: Some(9),
    }));
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "Widget".to_string(),
        kind: "impl".to_string(),
        line: Some(6),
        column: Some(6),
    }));
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "build".to_string(),
        kind: "function".to_string(),
        line: Some(7),
        column: Some(26),
    }));
}

#[test]
fn rust_extractor_captures_use_paths_and_multiple_call_shapes() {
    let source = r#"pub use crate::shared::helper;

fn entry() {
    helper();
    crate::shared::helper();
    value.method();
    println!("ignored macro");
}
"#;

    let graph = extract_graph("rust", source);
    assert!(graph.deps.iter().any(|dep| dep == "crate::shared::helper"));
    assert!(graph.refs.contains(&GraphRef {
        symbol: "helper".to_string(),
        line: Some(4),
        column: Some(5),
    }));
    assert!(graph.refs.contains(&GraphRef {
        symbol: "crate::shared::helper".to_string(),
        line: Some(5),
        column: Some(5),
    }));
    assert!(graph.refs.contains(&GraphRef {
        symbol: "value.method".to_string(),
        line: Some(6),
        column: Some(5),
    }));
    assert!(
        !graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "println!")
    );
}

#[test]
fn rust_extractor_captures_type_refs_imports_and_struct_literals() {
    let source = r#"use super::{extract_graph, GraphRef, GraphSymbol};

pub struct GraphRef {
    value: usize,
}

pub struct Holder {
    inner: GraphRef,
}

impl GraphRef {
    pub fn from_value(value: usize) -> Self {
        GraphRef { value }
    }
}

fn mirror(input: &GraphRef) -> GraphRef {
    GraphRef { value: input.value }
}
"#;

    let graph = extract_graph("rust", source);
    assert!(graph.refs.contains(&GraphRef {
        symbol: "GraphRef".to_string(),
        line: Some(1),
        column: Some(28),
    }));
    assert!(graph.refs.contains(&GraphRef {
        symbol: "GraphRef".to_string(),
        line: Some(8),
        column: Some(12),
    }));
    assert!(graph.refs.contains(&GraphRef {
        symbol: "GraphRef".to_string(),
        line: Some(11),
        column: Some(6),
    }));
    assert!(graph.refs.contains(&GraphRef {
        symbol: "GraphRef".to_string(),
        line: Some(13),
        column: Some(9),
    }));
    assert!(graph.refs.contains(&GraphRef {
        symbol: "GraphRef".to_string(),
        line: Some(17),
        column: Some(19),
    }));
}
