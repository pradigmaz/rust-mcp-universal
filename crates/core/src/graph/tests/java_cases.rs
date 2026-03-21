use super::super::{GraphSymbol, extract_graph};

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
