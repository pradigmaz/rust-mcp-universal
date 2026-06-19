use std::env;
use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use super::Engine;
use super::compatibility::{
    CURRENT_ANN_VERSION, CURRENT_INDEX_FORMAT_VERSION, CURRENT_SCHEMA_VERSION,
};
use super::preflight_runtime::{detect_running_binary_stale, detect_same_binary_other_pids};
use super::schema::OPEN_DB_READ_ONLY_PRAGMAS_SQL;
use crate::model::{PreflightState, PreflightStatus};

const RUNNING_BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");

impl Engine {
    pub fn preflight_status(&self) -> Result<PreflightStatus> {
        let binary_path = env::current_exe()
            .unwrap_or_else(|_| self.project_root.join("target/unknown-binary"))
            .display()
            .to_string();
        let running_binary_version = RUNNING_BINARY_VERSION.to_string();
        let stale_process_probe_binary_path = None;
        let launcher_recommended =
            cfg!(windows).then(|| "scripts/rmu-mcp-server-fresh.cmd".to_string());
        let safe_recovery_hint = compatibility_hint();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut db_schema_version = None;
        let mut index_format_version = None;
        let mut ann_version = None;
        let running_binary_stale =
            detect_running_binary_stale(Path::new(&binary_path), &mut errors);
        if running_binary_stale {
            errors.push(stale_running_binary_message(&running_binary_version));
        }

        if self.db_path.exists() {
            match open_preflight_db(&self.db_path) {
                Ok(conn) => {
                    db_schema_version = read_meta_u32(&conn, "schema_version")?;
                    index_format_version = read_meta_u32(&conn, "index_format_version")?;
                    ann_version = read_meta_u32(&conn, "ann_version")?;
                    if let Err(err) = super::compatibility::ensure_schema_preflight(&conn) {
                        errors.push(err.to_string());
                    }
                }
                Err(err) => errors.push(err.to_string()),
            }
        }

        let stale_process_probe_target = stale_process_probe_binary_path
            .as_deref()
            .unwrap_or(binary_path.as_str());
        let same_binary_other_pids =
            detect_same_binary_other_pids(stale_process_probe_target, &mut warnings);
        let stale_process_suspected = running_binary_stale && !same_binary_other_pids.is_empty();
        let status = if !errors.is_empty() {
            PreflightState::Incompatible
        } else if stale_process_suspected || !warnings.is_empty() {
            PreflightState::Warning
        } else {
            PreflightState::Ok
        };

        Ok(PreflightStatus {
            status,
            project_path: self.project_root.display().to_string(),
            binary_path,
            running_binary_version,
            running_binary_stale,
            stale_process_probe_binary_path,
            supported_schema_version: Some(CURRENT_SCHEMA_VERSION),
            db_schema_version,
            index_format_version: index_format_version.or(Some(CURRENT_INDEX_FORMAT_VERSION)),
            ann_version: ann_version.or(Some(CURRENT_ANN_VERSION)),
            same_binary_other_pids,
            stale_process_suspected,
            launcher_recommended,
            safe_recovery_hint,
            warnings,
            errors,
        })
    }
}

fn open_preflight_db(path: &Path) -> Result<Connection> {
    let conn = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("failed to open db {}", path.display()))?;
    conn.execute_batch(OPEN_DB_READ_ONLY_PRAGMAS_SQL)
        .context("failed to apply sqlite pragmas")?;
    Ok(conn)
}

fn read_meta_u32(conn: &Connection, key: &str) -> Result<Option<u32>> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    })
    .optional()?
    .map(|raw| {
        raw.parse::<u32>()
            .with_context(|| format!("meta key `{key}` has non-u32 value `{raw}`"))
    })
    .transpose()
}

fn compatibility_hint() -> String {
    if cfg!(windows) {
        "use scripts/rmu-mcp-server-fresh.cmd so the server is rebuilt/restarted if needed, then re-open the index".to_string()
    } else {
        "restart the process with a fresh binary and re-open the index".to_string()
    }
}

fn stale_running_binary_message(running_binary_version: &str) -> String {
    format!(
        "running binary version `{running_binary_version}` is stale: executable was rebuilt after process start; restart with a fresh binary before serving requests"
    )
}

#[cfg(test)]
mod tests {
    use super::{Engine, PreflightState};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn preflight_status_reads_initialized_database_via_read_only_pragmas() {
        let root = temp_dir("rmu-preflight-readonly-pragmas");
        fs::create_dir_all(&root).expect("create temp dir");
        let db_path = root.join(".rmu/index.db");

        let engine = Engine::new(root.clone(), Some(db_path.clone())).expect("initialize db");
        let status = Engine::new_read_only(root.clone(), Some(db_path))
            .expect("open read-only engine")
            .preflight_status()
            .expect("preflight status should succeed");

        assert_eq!(status.project_path, root.display().to_string());
        assert_eq!(
            status.db_schema_version,
            Some(super::CURRENT_SCHEMA_VERSION)
        );
        assert!(status.errors.is_empty());
        assert!(matches!(
            status.status,
            PreflightState::Ok | PreflightState::Warning
        ));

        drop(engine);
        let _ = fs::remove_dir_all(root);
    }
}
