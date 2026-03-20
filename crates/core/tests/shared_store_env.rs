use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use rmu_core::{Engine, MigrationMode};

fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

fn env_lock() -> &'static Mutex<()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

impl EnvVarGuard {
    fn set(key: &'static str, value: &Path) -> Self {
        let original = std::env::var(key).ok();
        // SAFETY: tests serialize environment mutations via `env_lock`.
        unsafe {
            std::env::set_var(key, value);
        }
        Self { key, original }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        // SAFETY: tests serialize environment mutations via `env_lock`.
        unsafe {
            if let Some(original) = &self.original {
                std::env::set_var(self.key, original);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }
}

#[test]
fn shared_store_requires_canonical_project_root_for_default_db_path() {
    let _env_guard = env_lock().lock().expect("env lock");
    let root = temp_dir("rmu-shared-store-canonicalize-fail");
    let shared_root = root.join("shared");
    let missing_project = root.join("missing-project");
    fs::create_dir_all(&shared_root).expect("shared root");
    let _shared = EnvVarGuard::set("RMU_DB_ROOT", &shared_root);

    let err = Engine::new(missing_project.clone(), None)
        .expect_err("shared store without canonical project root must fail closed");
    let err_text = err.to_string();
    assert!(err_text.contains("failed to canonicalize shared-store project root"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn shared_store_read_only_requires_canonical_project_root_for_default_db_path() {
    let _env_guard = env_lock().lock().expect("env lock");
    let root = temp_dir("rmu-shared-store-read-only-canonicalize-fail");
    let shared_root = root.join("shared");
    let missing_project = root.join("missing-project");
    fs::create_dir_all(&shared_root).expect("shared root");
    let _shared = EnvVarGuard::set("RMU_DB_ROOT", &shared_root);

    let err = Engine::new_read_only_with_migration_mode(
        missing_project.clone(),
        None,
        MigrationMode::Auto,
    )
    .expect_err("read-only shared store without canonical project root must fail closed");
    let err_text = err.to_string();
    assert!(err_text.contains("failed to canonicalize shared-store project root"));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn explicit_db_path_bypasses_shared_store_project_key_resolution() -> anyhow::Result<()> {
    let _env_guard = env_lock().lock().expect("env lock");
    let root = temp_dir("rmu-shared-store-explicit-db-path");
    let shared_root = root.join("shared");
    let missing_project = root.join("missing-project");
    let explicit_db = root.join("explicit/index.db");
    fs::create_dir_all(&shared_root)?;
    let _shared = EnvVarGuard::set("RMU_DB_ROOT", &shared_root);

    let engine = Engine::new(missing_project.clone(), Some(explicit_db.clone()))?;
    assert_eq!(engine.db_path, explicit_db);
    assert!(explicit_db.exists());

    let _ = fs::remove_dir_all(root);
    Ok(())
}
