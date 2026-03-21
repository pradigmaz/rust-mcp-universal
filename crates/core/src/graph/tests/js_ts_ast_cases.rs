use super::super::{GraphRef, GraphSourceKind, GraphSymbol, extract_graph, extract_graph_for_kind};

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
