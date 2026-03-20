use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use super::{DbStoreConfig, project_key};

const ENV_DB_ROOT: &str = "RMU_DB_ROOT";
const ENV_DB_TTL_DAYS: &str = "RMU_DB_TTL_DAYS";
const DEFAULT_TTL_DAYS: i64 = 15;

pub(super) fn default_db_path_for_project_impl(project_root: &Path) -> Result<PathBuf> {
    let config = store_config_impl(project_root);
    if config.shared_store {
        let key = project_key(project_root)?;
        Ok(config.root_dir.join(format!("{key}.db")))
    } else {
        Ok(config.root_dir.join("index.db"))
    }
}

pub(super) fn store_config_impl(project_root: &Path) -> DbStoreConfig {
    let (root_dir, shared_store) = resolve_store_root(project_root);
    DbStoreConfig {
        root_dir,
        ttl_days: resolve_ttl_days(),
        shared_store,
    }
}

pub(super) fn project_key_impl(project_root: &Path) -> Result<String> {
    let canonical = resolve_project_root_identity(project_root)?;
    let mut hasher = Sha256::new();

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        hasher.update(canonical.as_os_str().as_bytes());
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        let mut normalized = Vec::new();
        for mut unit in canonical.as_os_str().encode_wide() {
            if unit == u16::from(b'\\') {
                unit = u16::from(b'/');
            }
            normalized.extend_from_slice(&unit.to_le_bytes());
        }
        hasher.update(normalized);
    }

    #[cfg(all(not(unix), not(windows)))]
    {
        hasher.update(canonical.to_string_lossy().as_bytes());
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn resolve_project_root_identity(project_root: &Path) -> Result<PathBuf> {
    project_root.canonicalize().with_context(|| {
        format!(
            "failed to canonicalize shared-store project root `{}`; refusing to derive project key from unresolved path",
            project_root.display()
        )
    })
}

fn resolve_store_root(project_root: &Path) -> (PathBuf, bool) {
    if let Ok(raw) = std::env::var(ENV_DB_ROOT) {
        let raw = raw.trim();
        if !raw.is_empty() {
            return (PathBuf::from(raw), true);
        }
    }

    (project_root.join(".rmu"), false)
}

fn resolve_ttl_days() -> i64 {
    std::env::var(ENV_DB_TTL_DAYS)
        .ok()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .unwrap_or(DEFAULT_TTL_DAYS)
}
