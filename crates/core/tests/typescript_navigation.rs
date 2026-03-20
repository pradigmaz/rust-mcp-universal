use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rmu_core::Engine;

fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

fn write_project_file(root: &Path, relative: &str, contents: &str) -> Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

#[test]
fn typescript_projects_populate_navigation_graph() -> Result<()> {
    let root = temp_dir("rmu-typescript-navigation");
    fs::create_dir_all(&root)?;

    write_project_file(
        &root,
        "src/core/Game.ts",
        r#"export class Game {
  public tick(): void {}
}
"#,
    )?;
    write_project_file(
        &root,
        "src/core/GameUpdate.ts",
        r#"import type { Game } from "./Game";

export function updateGame(game: Game): void {
  void game;
}
"#,
    )?;
    write_project_file(
        &root,
        "src/core/GameLoop.ts",
        r#"import type { Game } from "./Game";
import { updateGame } from "./GameUpdate";

export type GameLoopState = {
  ticks: number;
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
"#,
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let status = engine.index_status()?;
    assert!(status.symbols >= 4, "expected TS symbols, got {status:?}");
    assert!(status.refs >= 4, "expected TS refs, got {status:?}");
    assert!(status.module_deps >= 2, "expected TS deps, got {status:?}");

    let symbols = engine.symbol_lookup("Game", 10)?;
    assert!(symbols.iter().any(|hit| {
        hit.path == "src/core/Game.ts" && hit.name == "Game" && hit.language == "typescript"
    }));

    let refs = engine.symbol_references("Game", 10)?;
    assert!(refs.iter().any(|hit| hit.path == "src/core/GameLoop.ts"));
    assert!(refs.iter().any(|hit| hit.path == "src/core/GameUpdate.ts"));

    let related = engine.related_files("src/core/Game.ts", 10)?;
    assert!(related.iter().any(|hit| hit.path == "src/core/GameLoop.ts"));
    assert!(
        related
            .iter()
            .any(|hit| hit.path == "src/core/GameUpdate.ts")
    );

    let _ = fs::remove_dir_all(root);
    Ok(())
}
