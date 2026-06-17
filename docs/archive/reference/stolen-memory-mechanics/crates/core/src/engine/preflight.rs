use std::env;
#[cfg(windows)]
use std::fs;
use std::path::Path;
#[cfg(windows)]
use std::process;
#[cfg(windows)]
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use crate::context::{legacy_project_memory_root, project_memory_root};
use crate::model::{PreflightState, PreflightStatus};

use super::canonical::scan_canonical_locations;
use super::notes::scan_markdown_files;
use super::{CURRENT_SCHEMA_VERSION, Engine};

const RUNNING_BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");
#[cfg(windows)]
const RUNNING_BINARY_STALE_GRACE_MS: i128 = 2_000;

pub(super) fn preflight_status(engine: &Engine) -> Result<PreflightStatus> {
    let binary_path = env::current_exe()
        .unwrap_or_else(|_| engine.project_root.join("target/unknown-binary"))
        .display()
        .to_string();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let missing_canonical_paths = missing_canonical_paths(&engine.memory_root);
    if !missing_canonical_paths.is_empty() {
        warnings.push(format!(
            "{} required canonical path(s) are missing from memory root layout",
            missing_canonical_paths.len()
        ));
    }
    let legacy_root_path = legacy_root_path(&engine.project_root, &engine.memory_root);
    let legacy_root_layout_detected = legacy_root_path.is_some();
    if legacy_root_layout_detected {
        warnings.push(
            "legacy canonical memory layout detected outside the active memory root; use migrate_memory_root before relying on the new storage root"
                .to_string(),
        );
    }
    let running_binary_stale = detect_running_binary_stale(Path::new(&binary_path), &mut warnings);
    if running_binary_stale {
        errors.push(stale_running_binary_message());
    }

    let mut db_schema_version = None;
    if engine.db_path.exists() {
        match Connection::open_with_flags(
            &engine.db_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .with_context(|| format!("failed to open db {}", engine.db_path.display()))
        {
            Ok(conn) => match read_meta_u32(&conn, "schema_version") {
                Ok(version) => {
                    db_schema_version = version;
                    if let Some(version) = db_schema_version {
                        if version > CURRENT_SCHEMA_VERSION {
                            errors.push(format!(
                                "db schema version `{version}` is newer than binary supported version `{CURRENT_SCHEMA_VERSION}`"
                            ));
                        }
                    }
                }
                Err(err) => errors.push(format!(
                    "failed to read derived index schema metadata: {err}"
                )),
            },
            Err(err) => errors.push(err.to_string()),
        }
    } else {
        warnings.push("index database does not exist yet".to_string());
    }

    let same_binary_other_pids = detect_same_binary_other_pids(&binary_path, &mut warnings);
    let stale_process_suspected = !same_binary_other_pids.is_empty();
    let status = if !errors.is_empty() {
        PreflightState::Incompatible
    } else if stale_process_suspected || !warnings.is_empty() {
        PreflightState::Warning
    } else {
        PreflightState::Ok
    };
    let safe_recovery_hint = if errors
        .iter()
        .any(|error| error.contains("db schema version") || error.contains("schema metadata"))
    {
        "use a compatible binary or delete the derived index, then run rebuild_index".to_string()
    } else if running_binary_stale {
        "restart the server with a fresh binary and run rebuild_index if needed".to_string()
    } else if legacy_root_layout_detected {
        "run migrate_memory_root to move canonical Markdown into the active memory root".to_string()
    } else if !missing_canonical_paths.is_empty() {
        "create the missing canonical project paths, then run rebuild_index".to_string()
    } else {
        "run rebuild_index after addressing the reported warnings or errors".to_string()
    };

    Ok(PreflightStatus {
        status,
        project_path: engine.project_root.display().to_string(),
        project_root: engine.project_root.display().to_string(),
        memory_root: engine.memory_root.display().to_string(),
        storage_mode: engine.storage_mode,
        binary_path,
        running_binary_version: RUNNING_BINARY_VERSION.to_string(),
        running_binary_stale,
        supported_schema_version: Some(CURRENT_SCHEMA_VERSION),
        db_schema_version,
        same_binary_other_pids,
        stale_process_suspected,
        safe_recovery_hint,
        missing_canonical_paths,
        legacy_root_layout_detected,
        legacy_root_path: legacy_root_path.map(|path| path.display().to_string()),
        warnings,
        errors,
    })
}

fn missing_canonical_paths(memory_root: &Path) -> Vec<String> {
    scan_canonical_locations(memory_root)
        .into_iter()
        .filter(|path| path != memory_root && !path.exists())
        .map(|path| {
            path.strip_prefix(memory_root)
                .unwrap_or(&path)
                .display()
                .to_string()
        })
        .collect()
}

fn legacy_root_layout_detected(project_root: &Path) -> bool {
    if scan_markdown_files(project_root)
        .map(|files| !files.is_empty())
        .unwrap_or(false)
    {
        return true;
    }
    scan_canonical_locations(project_root)
        .into_iter()
        .filter(|path| path != project_root)
        .any(|path| path.exists())
}

fn legacy_root_path(project_root: &Path, memory_root: &Path) -> Option<std::path::PathBuf> {
    if project_root != memory_root && legacy_root_layout_detected(project_root) {
        return Some(project_root.to_path_buf());
    }

    let visible_project_root = project_memory_root(project_root);
    if visible_project_root != memory_root && legacy_root_layout_detected(&visible_project_root) {
        return Some(visible_project_root);
    }

    let hidden_root = legacy_project_memory_root(project_root);
    if hidden_root != memory_root && legacy_root_layout_detected(&hidden_root) {
        return Some(hidden_root);
    }

    None
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

fn stale_running_binary_message() -> String {
    format!(
        "running binary version `{RUNNING_BINARY_VERSION}` is stale: executable was rebuilt after process start"
    )
}

fn detect_running_binary_stale(binary_path: &Path, warnings: &mut Vec<String>) -> bool {
    #[cfg(windows)]
    {
        match (
            file_modified_unix_ms(binary_path),
            current_process_started_at_unix_ms_windows(),
        ) {
            (Ok(binary_modified_at_ms), Ok(process_started_at_ms)) => {
                binary_modified_at_ms > process_started_at_ms + RUNNING_BINARY_STALE_GRACE_MS
            }
            (Err(err), _) | (_, Err(err)) => {
                warnings.push(format!("running binary stale probe unavailable: {err}"));
                false
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (binary_path, warnings);
        false
    }
}

#[cfg(windows)]
fn file_modified_unix_ms(binary_path: &Path) -> Result<i128> {
    let modified = fs::metadata(binary_path)
        .with_context(|| format!("failed to stat {}", binary_path.display()))?
        .modified()
        .with_context(|| format!("failed to read modified time for {}", binary_path.display()))?;
    let duration = modified
        .duration_since(UNIX_EPOCH)
        .context("binary modified time predates unix epoch")?;
    Ok(i128::from(duration.as_millis() as i64))
}

#[cfg(windows)]
fn current_process_started_at_unix_ms_windows() -> Result<i128> {
    use std::process::Command;

    let current_pid = process::id();
    let script = format!(
        "$ErrorActionPreference='Stop'; $p = Get-Process -Id {current_pid} -ErrorAction Stop; [DateTimeOffset]::new($p.StartTime.ToUniversalTime()).ToUnixTimeMilliseconds()"
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .context("failed to run process start-time probe")?;
    if !output.status.success() {
        anyhow::bail!(
            "process start-time probe failed with exit code {:?}",
            output.status.code()
        );
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    raw.parse::<i128>()
        .with_context(|| format!("failed to parse process start time `{raw}`"))
}

fn detect_same_binary_other_pids(binary_path: &str, warnings: &mut Vec<String>) -> Vec<u32> {
    #[cfg(windows)]
    {
        let current_pid = process::id();
        let escaped = binary_path.replace('\'', "''");
        let script = format!(
            "$p='{escaped}'; Get-CimInstance Win32_Process | Where-Object {{ $_.ExecutablePath -and [System.StringComparer]::OrdinalIgnoreCase.Equals([System.IO.Path]::GetFullPath($_.ExecutablePath), $p) -and $_.ProcessId -ne {current_pid} }} | Select-Object -ExpandProperty ProcessId | ConvertTo-Json -Compress"
        );
        match run_process_probe_script(&script) {
            Ok(pids) => pids,
            Err(err) => {
                warnings.push(format!("stale process probe unavailable: {err}"));
                Vec::new()
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = (binary_path, warnings);
        Vec::new()
    }
}

#[cfg(windows)]
fn run_process_probe_script(script: &str) -> Result<Vec<u32>> {
    use std::process::Command;

    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", script])
        .output()
        .context("failed to run process probe")?;
    if !output.status.success() {
        anyhow::bail!(
            "process probe failed with exit code {:?}",
            output.status.code()
        );
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() || raw == "null" {
        Ok(Vec::new())
    } else if let Ok(single) = serde_json::from_str::<u32>(&raw) {
        Ok(vec![single])
    } else {
        serde_json::from_str::<Vec<u32>>(&raw)
            .with_context(|| format!("failed to parse process probe output `{raw}`"))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    use rusqlite::Connection;

    use super::preflight_status;
    use crate::engine::{CURRENT_SCHEMA_VERSION, Engine};
    use crate::model::{PreflightState, StorageMode};

    fn temp_root(prefix: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("obsidian-memory-preflight-{prefix}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn normalize_test_path(path: &Path) -> String {
        let raw = path.display().to_string();
        #[cfg(windows)]
        {
            raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_string()
        }
        #[cfg(not(windows))]
        {
            raw
        }
    }

    #[test]
    fn preflight_reports_missing_canonical_paths_as_warning() {
        let root = temp_root("layout");
        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

        let status = preflight_status(&engine).expect("preflight");

        assert_eq!(status.status, PreflightState::Warning);
        assert!(!status.missing_canonical_paths.is_empty());
        assert!(status.errors.is_empty());
        assert!(
            status
                .safe_recovery_hint
                .contains("create the missing canonical project paths")
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn preflight_reports_schema_mismatch_as_incompatible() {
        let root = temp_root("schema");
        let db_dir = root.join("memory").join(".derived");
        std::fs::create_dir_all(&db_dir).expect("db dir");
        let db_path = db_dir.join("index.db");
        let conn = Connection::open(&db_path).expect("db");
        conn.execute_batch("CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);")
            .expect("meta");
        conn.execute(
            "INSERT INTO meta(key, value) VALUES ('schema_version', ?1)",
            [(CURRENT_SCHEMA_VERSION + 1).to_string()],
        )
        .expect("schema version");
        drop(conn);

        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
        let status = preflight_status(&engine).expect("preflight");

        assert_eq!(status.status, PreflightState::Incompatible);
        assert_eq!(status.db_schema_version, Some(CURRENT_SCHEMA_VERSION + 1));
        assert!(!status.errors.is_empty());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn preflight_warns_when_legacy_root_layout_exists_outside_active_memory_root() {
        let root = temp_root("legacy-layout");
        std::fs::create_dir_all(root.join("decisions")).expect("create decisions");
        std::fs::write(
            root.join("decisions").join("auth.md"),
            "---\nid: decision-auth\ntype: Decision\ntitle: Auth\nstatus: active\nproject: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# Auth\n\n## Summary\nAuth summary.\n\n## Observations\n\n## Relations\n\n## References\n",
        )
        .expect("write note");

        let engine = Engine::new_with_mode(&root, StorageMode::Codex).expect("engine");
        let status = preflight_status(&engine).expect("preflight");

        assert_eq!(status.status, PreflightState::Warning);
        assert!(status.legacy_root_layout_detected);
        assert!(
            status
                .legacy_root_path
                .as_deref()
                .is_some_and(|value| value.ends_with(&root.display().to_string()))
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn preflight_warns_when_hidden_project_memory_exists_outside_visible_memory_root() {
        let root = temp_root("hidden-project-memory");
        let hidden_root = root.join(".memory");
        std::fs::create_dir_all(hidden_root.join("tasks")).expect("create tasks");
        std::fs::write(
            hidden_root.join("tasks").join("eval.md"),
            "---\nid: task-eval\ntype: Task\ntitle: Eval\nstatus: active\nproject: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# Eval\n\n## Summary\nEval summary.\n\n## Observations\n\n## Relations\n\n## References\n",
        )
        .expect("write note");

        let engine = Engine::new_with_mode(&root, StorageMode::Codex).expect("engine");
        let status = preflight_status(&engine).expect("preflight");
        let actual_hidden_root = status
            .legacy_root_path
            .as_deref()
            .map(Path::new)
            .map(normalize_test_path);
        let expected_hidden_root = normalize_test_path(&hidden_root);

        assert_eq!(status.status, PreflightState::Warning);
        assert!(status.legacy_root_layout_detected);
        assert_eq!(
            actual_hidden_root.as_deref(),
            Some(expected_hidden_root.as_str())
        );

        let _ = std::fs::remove_dir_all(root);
    }
}
