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
fn java_projects_populate_navigation_graph() -> Result<()> {
    let root = temp_dir("rmu-java-navigation");
    fs::create_dir_all(&root)?;

    write_project_file(
        &root,
        "src/main/java/example/core/Game.java",
        r#"package example.core;

public class Game {
  public void tick() {}
}
"#,
    )?;
    write_project_file(
        &root,
        "src/main/java/example/loop/GameUpdate.java",
        r#"package example.loop;

import example.core.Game;

public final class GameUpdate {
  public static void updateGame(Game game) {
    game.tick();
  }
}
"#,
    )?;
    write_project_file(
        &root,
        "src/main/java/example/loop/GameLoop.java",
        r#"package example.loop;

import java.util.List;
import example.core.Game;

public class GameLoop {
  private final Game game;

  public GameLoop(Game game) {
    this.game = game;
  }

  public List<Game> tick() {
    GameUpdate.updateGame(this.game);
    return List.of(this.game);
  }
}
"#,
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let status = engine.index_status()?;
    assert!(status.symbols >= 5, "expected Java symbols, got {status:?}");
    assert!(status.refs >= 8, "expected Java refs, got {status:?}");
    assert!(
        status.module_deps >= 3,
        "expected Java deps, got {status:?}"
    );

    let symbols = engine.symbol_lookup("Game", 10)?;
    assert!(
        symbols.iter().any(|hit| {
            hit.path == "src/main/java/example/core/Game.java" && hit.name == "Game"
        })
    );

    let refs = engine.symbol_references("Game", 10)?;
    assert!(
        refs.iter()
            .any(|hit| hit.path == "src/main/java/example/loop/GameLoop.java")
    );
    assert!(
        refs.iter()
            .any(|hit| hit.path == "src/main/java/example/loop/GameUpdate.java")
    );

    let related = engine.related_files("src/main/java/example/core/Game.java", 10)?;
    assert!(
        related
            .iter()
            .any(|hit| hit.path == "src/main/java/example/loop/GameLoop.java")
    );
    assert!(
        related
            .iter()
            .any(|hit| hit.path == "src/main/java/example/loop/GameUpdate.java")
    );

    let _ = fs::remove_dir_all(root);
    Ok(())
}
