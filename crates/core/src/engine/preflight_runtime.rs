use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

use anyhow::{Context, Result};

#[cfg(windows)]
use std::process::{self, Command};

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

pub(super) fn detect_running_binary_stale(binary_path: &Path, errors: &mut Vec<String>) -> bool {
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

pub(super) fn detect_same_binary_other_pids(
    binary_path: &str,
    warnings: &mut Vec<String>,
) -> Vec<u32> {
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
        RUNNING_BINARY_STALE_GRACE_MS, TEST_BINARY_MODIFIED_AT_MS_ENV,
        TEST_PROCESS_STARTED_AT_MS_ENV, is_running_binary_stale, parse_test_timestamp,
        read_running_binary_timestamps, set_thread_running_binary_timestamps_override_for_tests,
    };
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

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
}
