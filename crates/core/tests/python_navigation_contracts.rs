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
fn python_navigation_handles_complex_unsupported_syntax_without_panicking() -> Result<()> {
    let root = temp_dir("rmu-python-navigation-contracts");
    fs::create_dir_all(&root)?;

    write_project_file(
        &root,
        "src/helpers.py",
        r#"def helper():
    return "ok"
"#,
    )?;
    write_project_file(
        &root,
        "src/advanced.py",
        r#"from pkg import (
    alpha,
    beta,
)


def traced(fn):
    return fn


@traced
async def decorated_worker():
    return await alpha(beta(helper()))
"#,
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let status = engine.index_status()?;
    assert_eq!(status.files, 2, "complex syntax should still index files");
    assert!(
        status.symbols >= 2,
        "supported defs should still be captured"
    );

    let helper_symbols = engine.symbol_lookup("helper", 10)?;
    assert!(
        helper_symbols
            .iter()
            .any(|hit| hit.path == "src/helpers.py")
    );

    let worker_symbols = engine.symbol_lookup("decorated_worker", 10)?;
    assert!(
        worker_symbols.is_empty()
            || worker_symbols
                .iter()
                .any(|hit| hit.path == "src/advanced.py"),
        "unsupported async/decorated defs must degrade gracefully"
    );

    let helper_refs = engine.symbol_references("helper", 10)?;
    assert!(
        helper_refs.is_empty() || helper_refs.iter().any(|hit| hit.path == "src/advanced.py"),
        "unsupported syntax may miss refs, but should not corrupt navigation state"
    );

    let related = engine.related_files("src/helpers.py", 10)?;
    assert!(related.iter().all(|hit| !hit.path.is_empty()));

    let _ = fs::remove_dir_all(root);
    Ok(())
}
