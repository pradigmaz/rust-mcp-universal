use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rmu_core::Engine;

pub(crate) const CHILD_TEST_ENV: &str = "RMU_TEST_DEFAULT_STORE_CHILD";
pub(crate) const CHILD_ROOT_ENV: &str = "RMU_TEST_DEFAULT_STORE_ROOT";
pub(crate) const CHILD_PROJECT_ENV: &str = "RMU_TEST_DEFAULT_STORE_PROJECT";
pub(crate) const CHILD_SHARED_ENV: &str = "RMU_TEST_DEFAULT_STORE_SHARED";

pub(crate) fn temp_project_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

pub(crate) fn setup_indexed_project() -> Result<(PathBuf, Engine), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        r#"
fn alpha_beta_gamma() {
    let value = "alpha beta gamma delta";
    println!("{value}");
}
"#,
    )?;
    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;
    Ok((project_dir, engine))
}

pub(crate) fn cleanup_project(path: &Path) {
    let _ = fs::remove_dir_all(path);
}
