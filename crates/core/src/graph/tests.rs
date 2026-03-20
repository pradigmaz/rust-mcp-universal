use std::path::Path;

use super::{
    CURRENT_GRAPH_FINGERPRINT_VERSION, GraphExtraction, GraphFingerprintBuilder, GraphRef,
    GraphSourceKind, GraphSymbol, empty_graph_content_hash, extract_graph, extract_graph_for_kind,
    graph_content_hash, graph_source_kind_for_path, supports_graph_extraction,
};

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

#[test]
fn typescript_extractor_captures_symbols_deps_and_refs() {
    let source = r#"import type { Game } from "./Game";
import { updateGame } from "./GameUpdate";
import * as THREE from "three";

export type GameLoopState = {
  viewDir: THREE.Vector3;
};

export class GameLoop {
  private game: Game;

  constructor(game: Game) {
    this.game = game;
  }

  public tick(): void {
    updateGame(this.game);
  }
}
"#;

    let graph = extract_graph("typescript", source);
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "GameLoopState".to_string(),
        kind: "type".to_string(),
        line: Some(5),
        column: Some(13),
    }));
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "GameLoop".to_string(),
        kind: "class".to_string(),
        line: Some(9),
        column: Some(14),
    }));
    assert!(graph.deps.iter().any(|dep| dep == "./Game"));
    assert!(graph.deps.iter().any(|dep| dep == "./GameUpdate"));
    assert!(graph.deps.iter().any(|dep| dep == "three"));
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Game" && graph_ref.line == Some(1))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "THREE.Vector3" && graph_ref.line == Some(6))
    );
    assert!(graph.refs.contains(&GraphRef {
        symbol: "updateGame".to_string(),
        line: Some(17),
        column: Some(5),
    }));
}

#[test]
fn java_extractor_captures_symbols_deps_and_refs() {
    let source = r#"package example.loop;

import java.util.List;
import example.core.Game;

public record GameLoop(Game game) {
  public static GameLoop create(Game game) {
    return new GameLoop(game);
  }

  public List<Game> tick() {
    GameUpdate.updateGame(this.game);
    return List.of(this.game);
  }
}
"#;

    let graph = extract_graph("java", source);
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "GameLoop".to_string(),
        kind: "record".to_string(),
        line: Some(6),
        column: Some(15),
    }));
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "create".to_string(),
        kind: "method".to_string(),
        line: Some(7),
        column: Some(26),
    }));
    assert!(graph.deps.iter().any(|dep| dep == "java.util.List"));
    assert!(graph.deps.iter().any(|dep| dep == "example.core.Game"));
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "example.core.Game" && graph_ref.line == Some(4))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "GameUpdate.updateGame"
                && graph_ref.line == Some(12))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "List" && graph_ref.line == Some(11))
    );
}

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

#[test]
fn javascript_ast_extractor_handles_multiline_imports_and_export_from() {
    let source = r#"import {
  Game as LocalGame,
  type Runner,
} from "./game";

export {
  LocalGame,
  type Runner,
} from "./runner";

new LocalGame();
"#;

    let graph = extract_graph_for_kind(GraphSourceKind::TypeScript { jsx: false }, source);
    assert!(graph.deps.iter().any(|dep| dep == "./game"));
    assert!(graph.deps.iter().any(|dep| dep == "./runner"));
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Game" && graph_ref.line == Some(2))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Runner" && graph_ref.line == Some(3))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Game" && graph_ref.line == Some(11))
    );
}

#[test]
fn javascript_ast_extractor_normalizes_aliases_and_collects_deps() {
    let source = r#"import type { Game as LocalGame } from "./Game";
import Engine from "./Engine";
import * as THREE from "three";

export class SceneController extends Engine implements LocalGame {
  render(): THREE.Vector3 {
    return new THREE.Scene();
  }
}
"#;

    let graph = extract_graph_for_kind(GraphSourceKind::TypeScript { jsx: false }, source);
    assert!(graph.deps.iter().any(|dep| dep == "./Game"));
    assert!(graph.deps.iter().any(|dep| dep == "./Engine"));
    assert!(graph.deps.iter().any(|dep| dep == "three"));
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "SceneController".to_string(),
        kind: "class".to_string(),
        line: Some(5),
        column: Some(14),
    }));
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Game" && graph_ref.line == Some(1))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Engine" && graph_ref.line == Some(5))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Game" && graph_ref.line == Some(5))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "THREE.Vector3" && graph_ref.line == Some(6))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "THREE.Scene" && graph_ref.line == Some(7))
    );
}

#[test]
fn javascript_ast_extractor_collects_require_dynamic_import_and_type_refs() {
    let source = r#"type ResultBox = Promise<Game | ErrorState & Runner>;

const util = require("./util");
async function load() {
  return import("./dynamic");
}
"#;

    let graph = extract_graph_for_kind(GraphSourceKind::TypeScript { jsx: false }, source);
    assert!(graph.deps.iter().any(|dep| dep == "./util"));
    assert!(graph.deps.iter().any(|dep| dep == "./dynamic"));
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Game" && graph_ref.line == Some(1))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "ErrorState" && graph_ref.line == Some(1))
    );
    assert!(
        graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Runner" && graph_ref.line == Some(1))
    );
}

#[test]
fn javascript_ast_extractor_supports_jsx_and_tsx_component_refs() {
    let jsx = r#"import Widget from "./Widget";
export const App = () => <Widget />;
"#;
    let tsx = r#"import type { ViewModel } from "./view-model";
import Layout from "./Layout";

export const Screen = (): ViewModel => <Layout.Panel />;
"#;

    let jsx_graph = extract_graph_for_kind(GraphSourceKind::JavaScript { jsx: true }, jsx);
    assert!(jsx_graph.deps.iter().any(|dep| dep == "./Widget"));
    assert!(jsx_graph.symbols.contains(&GraphSymbol {
        name: "App".to_string(),
        kind: "function".to_string(),
        line: Some(2),
        column: Some(14),
    }));
    assert!(
        jsx_graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Widget" && graph_ref.line == Some(2))
    );

    let tsx_graph = extract_graph_for_kind(GraphSourceKind::TypeScript { jsx: true }, tsx);
    assert!(tsx_graph.deps.iter().any(|dep| dep == "./view-model"));
    assert!(tsx_graph.deps.iter().any(|dep| dep == "./Layout"));
    assert!(
        tsx_graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "ViewModel" && graph_ref.line == Some(4))
    );
    assert!(
        tsx_graph
            .refs
            .iter()
            .any(|graph_ref| graph_ref.symbol == "Layout.Panel" && graph_ref.line == Some(4))
    );
}

#[test]
fn javascript_ast_extractor_keeps_ast_payload_for_recoverable_parse_errors() {
    let source = r#"import {
  Game as LocalGame,
} from "./Game";

export class Broken {}
const value =
"#;

    let graph = extract_graph_for_kind(GraphSourceKind::TypeScript { jsx: false }, source);
    assert!(graph.deps.iter().any(|dep| dep == "./Game"));
    assert!(graph.symbols.contains(&GraphSymbol {
        name: "Broken".to_string(),
        kind: "class".to_string(),
        line: Some(5),
        column: Some(14),
    }));
}

#[test]
fn javascript_ast_extractor_falls_back_when_parse_errors_produce_empty_graph() {
    let source = "Widget(";

    let graph = extract_graph_for_kind(GraphSourceKind::JavaScript { jsx: false }, source);
    assert!(graph.symbols.is_empty());
    assert!(graph.deps.is_empty());
    assert!(graph.refs.contains(&GraphRef {
        symbol: "Widget".to_string(),
        line: Some(1),
        column: Some(1),
    }));
}

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
