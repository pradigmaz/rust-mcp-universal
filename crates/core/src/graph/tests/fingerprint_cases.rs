use super::super::{
    CURRENT_GRAPH_FINGERPRINT_VERSION, GraphExtraction, GraphFingerprintBuilder, GraphRef,
    GraphSymbol, empty_graph_content_hash, graph_content_hash,
};

#[test]
fn graph_content_hash_is_stable_for_unsorted_equivalent_graphs() {
    let first = GraphExtraction {
        symbols: vec![
            GraphSymbol {
                name: "Beta".to_string(),
                kind: "struct".to_string(),
                line: Some(4),
                column: Some(2),
            },
            GraphSymbol {
                name: "Alpha".to_string(),
                kind: "function".to_string(),
                line: Some(1),
                column: Some(1),
            },
        ],
        deps: vec!["crate::z".to_string(), "crate::a".to_string()],
        refs: vec![
            GraphRef {
                symbol: "Widget".to_string(),
                line: Some(9),
                column: Some(1),
            },
            GraphRef {
                symbol: "Alpha".to_string(),
                line: Some(3),
                column: Some(5),
            },
        ],
    };
    let second = GraphExtraction {
        symbols: first.symbols.iter().cloned().rev().collect(),
        deps: first.deps.iter().cloned().rev().collect(),
        refs: first.refs.iter().cloned().rev().collect(),
    };

    assert_eq!(
        graph_content_hash("rust", &first),
        graph_content_hash("rust", &second)
    );
}

#[test]
fn empty_graph_content_hash_matches_empty_extraction_hash() {
    let empty = GraphExtraction::default();
    assert_eq!(
        empty_graph_content_hash(),
        graph_content_hash("txt", &empty)
    );
}

#[test]
fn graph_content_hash_is_stable_for_empty_graph() {
    let graph = GraphExtraction::default();
    let left = graph_content_hash("rust", &graph);
    let right = GraphFingerprintBuilder::default().finish();
    assert_eq!(left, right);
    assert_eq!(CURRENT_GRAPH_FINGERPRINT_VERSION, 3);
}

#[test]
fn graph_content_hash_changes_when_symbol_payload_changes_without_count_change() {
    let left = GraphExtraction {
        symbols: vec![GraphSymbol {
            name: "Widget".to_string(),
            kind: "struct".to_string(),
            line: Some(1),
            column: Some(1),
        }],
        deps: vec![],
        refs: vec![],
    };
    let right = GraphExtraction {
        symbols: vec![GraphSymbol {
            name: "BrokenWidget".to_string(),
            kind: "struct".to_string(),
            line: Some(1),
            column: Some(1),
        }],
        deps: vec![],
        refs: vec![],
    };

    assert_ne!(
        graph_content_hash("rust", &left),
        graph_content_hash("rust", &right)
    );
}
