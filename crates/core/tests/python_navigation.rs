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
fn python_projects_populate_navigation_graph_for_supported_forms() -> Result<()> {
    let root = temp_dir("rmu-python-navigation");
    fs::create_dir_all(&root)?;

    write_project_file(
        &root,
        "src/core/game.py",
        r#"class Game:
    def tick(self):
        return "ok"


def new_game():
    return Game()
"#,
    )?;
    write_project_file(
        &root,
        "src/core/update.py",
        r#"from src.core.game import Game


def update_game(game):
    game.tick()
    return game
"#,
    )?;
    write_project_file(
        &root,
        "src/core/loop.py",
        r#"from src.core.update import update_game


def run_loop(game):
    update_game(game)
    return game
"#,
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let status = engine.index_status()?;
    assert!(
        status.symbols >= 4,
        "expected Python symbols, got {status:?}"
    );
    assert!(status.refs >= 2, "expected Python refs, got {status:?}");
    assert!(
        status.module_deps >= 2,
        "expected Python deps, got {status:?}"
    );

    let symbols = engine.symbol_lookup("update_game", 10)?;
    assert!(symbols.iter().any(|hit| {
        hit.path == "src/core/update.py" && hit.name == "update_game" && hit.language == "python"
    }));

    let refs = engine.symbol_references("update_game", 10)?;
    assert!(refs.iter().any(|hit| hit.path == "src/core/loop.py"));

    let related = engine.related_files("src/core/game.py", 10)?;
    assert!(related.iter().any(|hit| hit.path == "src/core/update.py"));

    let call_path = engine.call_path("update_game", "src/core/game.py", 4)?;
    assert!(call_path.found, "expected best-effort path to Game file");
    assert!(call_path.path.iter().any(|path| path == "src/core/game.py"));

    let _ = fs::remove_dir_all(root);
    Ok(())
}
