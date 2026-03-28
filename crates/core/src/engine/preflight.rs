use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::Path;
use std::process::{self, Command};
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension};

use super::Engine;
use super::compatibility::{
    CURRENT_ANN_VERSION, CURRENT_INDEX_FORMAT_VERSION, CURRENT_SCHEMA_VERSION,
};
use super::schema::OPEN_DB_READ_ONLY_PRAGMAS_SQL;
use crate::model::{PreflightState, PreflightStatus};

const RUNNING_BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");
const RUNNING_BINARY_STALE_GRACE_MS: i128 = 2_000;
const TEST_PROCESS_STARTED_AT_MS_ENV: &str = "RMU_TEST_PROCESS_STARTED_AT_MS";
const TEST_BINARY_MODIFIED_AT_MS_ENV: &str = "RMU_TEST_BINARY_MODIFIED_AT_MS";

thread_local! {
    static THREAD_RUNNING_BINARY_TIMESTAMPS_OVERRIDE: RefCell<Option<(i128, i128)>> =
        const { RefCell::new(None) };
}

#[doc(hidden)]
pub struct ThreadRunningBinaryTimestampsOverrideGuard {
    previous: Option<(i128, i128)>,
}

impl Drop for ThreadRunningBinaryTimestampsOverrideGuard {
    fn drop(&mut self) {
        THREAD_RUNNING_BINARY_TIMESTAMPS_OVERRIDE.with(|slot| {
            *slot.borrow_mut() = self.previous;
        });
    }
}

#[doc(hidden)]
pub fn set_thread_running_binary_timestamps_override_for_tests(
    process_started_at_ms: i128,
    binary_modified_at_ms: i128,
) -> ThreadRunningBinaryTimestampsOverrideGuard {
    let previous = THREAD_RUNNING_BINARY_TIMESTAMPS_OVERRIDE.with(|slot| {
        slot.borrow_mut()
            .replace((process_started_at_ms, binary_modified_at_ms))
    });
    ThreadRunningBinaryTimestampsOverrideGuard { previous }
}

impl Engine {
    pub fn preflight_status(&self) -> Result<PreflightStatus> {
        let binary_path = env::current_exe()
            .unwrap_or_else(|_| self.project_root.join("target/unknown-binary"))
            .display()
            .to_string();
        let running_binary_version = RUNNING_BINARY_VERSION.to_string();
        let stale_process_probe_binary_path =
            resolve_stale_process_probe_binary_path(Path::new(&binary_path))
                .map(|path| path.display().to_string());
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
        let stale_process_suspected = !same_binary_other_pids.is_empty();
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

fn resolve_stale_process_probe_binary_path(binary_path: &Path) -> Option<std::path::PathBuf> {
    let file_stem = binary_path.file_stem()?.to_str()?;
    if !file_stem.eq_ignore_ascii_case("rmu-cli") {
        return None;
    }

    let probe_path = binary_path.with_file_name("rmu-mcp-server.exe");
    probe_path.exists().then_some(probe_path)
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

fn detect_running_binary_stale(binary_path: &Path, errors: &mut Vec<String>) -> bool {
    match read_running_binary_timestamps(binary_path) {
        Ok(Some((process_started_at_ms, binary_modified_at_ms))) => {
            is_running_binary_stale(process_started_at_ms, binary_modified_at_ms)
        }
        Ok(None) => false,
        Err(_) => {
            let _ = errors;
            false
        }
    }
}

fn read_running_binary_timestamps(binary_path: &Path) -> Result<Option<(i128, i128)>> {
    if let Some(timestamps) = thread_running_binary_timestamps_override() {
        return Ok(Some(timestamps));
    }

    if let Some(timestamps) = test_running_binary_timestamps_override()? {
        return Ok(Some(timestamps));
    }

    #[cfg(windows)]
    {
        let binary_modified_at_ms = file_modified_unix_ms(binary_path)?;
        let process_started_at_ms = current_process_started_at_unix_ms_windows()?;
        Ok(Some((process_started_at_ms, binary_modified_at_ms)))
    }

    #[cfg(not(windows))]
    {
        let _ = binary_path;
        Ok(None)
    }
}

fn thread_running_binary_timestamps_override() -> Option<(i128, i128)> {
    THREAD_RUNNING_BINARY_TIMESTAMPS_OVERRIDE.with(|slot| *slot.borrow())
}

fn test_running_binary_timestamps_override() -> Result<Option<(i128, i128)>> {
    let process_started_at_ms = env::var(TEST_PROCESS_STARTED_AT_MS_ENV).ok();
    let binary_modified_at_ms = env::var(TEST_BINARY_MODIFIED_AT_MS_ENV).ok();
    match (process_started_at_ms, binary_modified_at_ms) {
        (None, None) => Ok(None),
        (Some(process_started_at_ms), Some(binary_modified_at_ms)) => Ok(Some((
            parse_test_timestamp(TEST_PROCESS_STARTED_AT_MS_ENV, &process_started_at_ms)?,
            parse_test_timestamp(TEST_BINARY_MODIFIED_AT_MS_ENV, &binary_modified_at_ms)?,
        ))),
        _ => Err(anyhow::anyhow!(
            "test running-binary timestamp override requires both `{TEST_PROCESS_STARTED_AT_MS_ENV}` and `{TEST_BINARY_MODIFIED_AT_MS_ENV}`"
        )),
    }
}

fn parse_test_timestamp(name: &str, raw: &str) -> Result<i128> {
    raw.parse::<i128>()
        .with_context(|| format!("failed to parse `{name}` value `{raw}` as unix milliseconds"))
}

fn file_modified_unix_ms(binary_path: &Path) -> Result<i128> {
    let modified = fs::metadata(binary_path)
        .with_context(|| format!("failed to stat running binary {}", binary_path.display()))?
        .modified()
        .with_context(|| format!("failed to read modified time for {}", binary_path.display()))?;
    let duration = modified
        .duration_since(UNIX_EPOCH)
        .context("running binary modified time predates unix epoch")?;
    Ok(i128::from(duration.as_millis() as i64))
}

#[cfg(windows)]
fn current_process_started_at_unix_ms_windows() -> Result<i128> {
    let current_pid = process::id();
    let script = format!(
        "$ErrorActionPreference='Stop'; $p = Get-Process -Id {current_pid} -ErrorAction Stop; [DateTimeOffset]::new($p.StartTime.ToUniversalTime()).ToUnixTimeMilliseconds()"
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .context("failed to run current-process start-time probe")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "current-process start-time probe failed with exit code {:?}",
            output.status.code()
        ));
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    raw.parse::<i128>()
        .with_context(|| format!("failed to parse current-process start time probe output `{raw}`"))
}

fn is_running_binary_stale(process_started_at_ms: i128, binary_modified_at_ms: i128) -> bool {
    binary_modified_at_ms > process_started_at_ms + RUNNING_BINARY_STALE_GRACE_MS
}

fn detect_same_binary_other_pids(binary_path: &str, warnings: &mut Vec<String>) -> Vec<u32> {
    #[cfg(windows)]
    {
        let current_pid = process::id();
        match probe_same_binary_other_pids_by_path(binary_path, current_pid) {
            Ok(pids) => pids,
            Err(err) => {
                warnings.push(format!(
                    "stale process exact-path probe unavailable: {err}; falling back to process-name match"
                ));
                match probe_same_binary_other_pids_by_name(binary_path, current_pid) {
                    Ok(pids) => pids,
                    Err(fallback_err) => {
                        warnings.push(format!(
                            "stale process name probe unavailable: {fallback_err}"
                        ));
                        Vec::new()
                    }
                }
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
fn probe_same_binary_other_pids_by_path(binary_path: &str, current_pid: u32) -> Result<Vec<u32>> {
    let escaped = binary_path.replace('\'', "''");
    let script = format!(
        "$p='{escaped}'; Get-CimInstance Win32_Process -Filter \"Name = 'rmu-mcp-server.exe'\" | Where-Object {{ $_.ExecutablePath -and [System.StringComparer]::OrdinalIgnoreCase.Equals([System.IO.Path]::GetFullPath($_.ExecutablePath), $p) -and $_.ProcessId -ne {current_pid} }} | Select-Object -ExpandProperty ProcessId | ConvertTo-Json -Compress"
    );
    run_process_probe_script(&script)
}

#[cfg(windows)]
fn probe_same_binary_other_pids_by_name(binary_path: &str, current_pid: u32) -> Result<Vec<u32>> {
    let process_name = Path::new(binary_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("rmu-mcp-server");
    let escaped = process_name.replace('\'', "''");
    let script = format!(
        "$name='{escaped}'; Get-Process -Name $name -ErrorAction SilentlyContinue | Where-Object {{ $_.Id -ne {current_pid} }} | Select-Object -ExpandProperty Id | ConvertTo-Json -Compress"
    );
    run_process_probe_script(&script)
}

#[cfg(windows)]
fn run_process_probe_script(script: &str) -> Result<Vec<u32>> {
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", script])
        .output()
        .context("failed to run stale process probe")?;
    if !output.status.success() {
        return Err(anyhow::anyhow!(
            "stale process probe failed with exit code {:?}",
            output.status.code()
        ));
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() || raw == "null" {
        Ok(Vec::new())
    } else if let Ok(single) = serde_json::from_str::<u32>(&raw) {
        Ok(vec![single])
    } else if let Ok(many) = serde_json::from_str::<Vec<u32>>(&raw) {
        Ok(many)
    } else {
        Err(anyhow::anyhow!(
            "failed to parse stale process probe output `{raw}`"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Engine, PreflightState, RUNNING_BINARY_STALE_GRACE_MS, TEST_BINARY_MODIFIED_AT_MS_ENV,
        TEST_PROCESS_STARTED_AT_MS_ENV, is_running_binary_stale, parse_test_timestamp,
        read_running_binary_timestamps, resolve_stale_process_probe_binary_path,
        set_thread_running_binary_timestamps_override_for_tests,
    };
    use std::fs;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
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
        fn set(key: &'static str, value: &str) -> Self {
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
    fn cli_binary_uses_server_binary_as_stale_process_probe_target_when_present() {
        let root = temp_dir("rmu-preflight-probe-target");
        fs::create_dir_all(&root).expect("create temp dir");
        let cli_path = root.join("rmu-cli.exe");
        let server_path = root.join("rmu-mcp-server.exe");
        fs::write(&cli_path, b"cli").expect("write cli placeholder");
        fs::write(&server_path, b"server").expect("write server placeholder");

        let resolved = resolve_stale_process_probe_binary_path(Path::new(&cli_path))
            .expect("server probe target should be resolved");
        assert_eq!(resolved, server_path);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn cli_binary_keeps_default_probe_when_server_binary_is_missing() {
        let root = temp_dir("rmu-preflight-probe-fallback");
        fs::create_dir_all(&root).expect("create temp dir");
        let cli_path = root.join("rmu-cli.exe");
        fs::write(&cli_path, b"cli").expect("write cli placeholder");

        let resolved = resolve_stale_process_probe_binary_path(Path::new(&cli_path));
        assert!(resolved.is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn non_cli_binary_does_not_override_stale_process_probe_target() {
        let root = temp_dir("rmu-preflight-probe-non-cli");
        fs::create_dir_all(&root).expect("create temp dir");
        let server_path = root.join("rmu-mcp-server.exe");
        fs::write(&server_path, b"server").expect("write server placeholder");

        let resolved = resolve_stale_process_probe_binary_path(Path::new(&server_path));
        assert!(resolved.is_none());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn running_binary_stale_uses_two_second_grace_window() {
        assert!(!is_running_binary_stale(
            10_000,
            10_000 + RUNNING_BINARY_STALE_GRACE_MS
        ));
        assert!(is_running_binary_stale(
            10_000,
            10_000 + RUNNING_BINARY_STALE_GRACE_MS + 1
        ));
    }

    #[test]
    fn running_binary_timestamp_override_requires_both_values() {
        let _guard = env_lock().lock().expect("env lock");
        let _process_started = EnvVarGuard::set(TEST_PROCESS_STARTED_AT_MS_ENV, "1000");
        let err = read_running_binary_timestamps(Path::new("unused"))
            .expect_err("single override should fail");
        assert!(err.to_string().contains(TEST_BINARY_MODIFIED_AT_MS_ENV));
    }

    #[test]
    fn running_binary_timestamp_override_uses_env_values() {
        let _guard = env_lock().lock().expect("env lock");
        let _process_started = EnvVarGuard::set(TEST_PROCESS_STARTED_AT_MS_ENV, "1000");
        let _binary_modified = EnvVarGuard::set(TEST_BINARY_MODIFIED_AT_MS_ENV, "4001");
        let timestamps =
            read_running_binary_timestamps(Path::new("unused")).expect("override timestamps");
        assert_eq!(timestamps, Some((1000, 4001)));
    }

    #[test]
    fn thread_local_timestamp_override_is_scoped_to_current_thread() {
        let _guard = set_thread_running_binary_timestamps_override_for_tests(1000, 4001);
        let timestamps = read_running_binary_timestamps(Path::new("unused"))
            .expect("thread-local override timestamps");
        assert_eq!(timestamps, Some((1000, 4001)));
    }

    #[test]
    fn test_timestamp_parser_rejects_invalid_values() {
        let err = parse_test_timestamp(TEST_PROCESS_STARTED_AT_MS_ENV, "not-a-number")
            .expect_err("invalid override must fail");
        assert!(err.to_string().contains("unix milliseconds"));
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
        assert_eq!(status.db_schema_version, Some(14));
        assert!(status.errors.is_empty());
        assert!(matches!(
            status.status,
            PreflightState::Ok | PreflightState::Warning
        ));

        drop(engine);
        let _ = fs::remove_dir_all(root);
    }
}
