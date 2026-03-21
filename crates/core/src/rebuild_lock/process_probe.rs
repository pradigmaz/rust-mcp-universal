#[cfg(target_os = "linux")]
use std::fs;
#[cfg(any(windows, all(unix, not(target_os = "linux"))))]
use std::process::Command;

use super::ProcessLiveness;

pub(super) fn process_liveness(pid: u32) -> ProcessLiveness {
    #[cfg(windows)]
    {
        process_liveness_windows(pid)
    }

    #[cfg(target_os = "linux")]
    {
        process_liveness_linux(pid)
    }

    #[cfg(all(unix, not(target_os = "linux")))]
    {
        process_liveness_unix_fallback(pid)
    }

    #[cfg(not(any(windows, unix)))]
    {
        let _ = pid;
        ProcessLiveness::Unknown
    }
}

#[cfg(windows)]
fn process_liveness_windows(pid: u32) -> ProcessLiveness {
    let script = format!(
        r#"$ErrorActionPreference='Stop';
try {{
  $p = Get-CimInstance Win32_Process -Filter "ProcessId = {pid}" -ErrorAction Stop
  if ($null -eq $p) {{ Write-Output "dead" }} else {{ Write-Output "alive" }}
}} catch {{
  Write-Output "unknown"
}}"#
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output();
    output.ok().map_or(ProcessLiveness::Unknown, |value| {
        parse_process_probe_output(&value.stdout)
    })
}

#[cfg(target_os = "linux")]
fn process_liveness_linux(pid: u32) -> ProcessLiveness {
    if pid == 0 {
        return ProcessLiveness::Unknown;
    }
    match fs::metadata(format!("/proc/{pid}")) {
        Ok(_) => ProcessLiveness::Alive,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => ProcessLiveness::Dead,
        Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => ProcessLiveness::Unknown,
        Err(_) => ProcessLiveness::Unknown,
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
pub(super) fn process_liveness_unix_fallback(pid: u32) -> ProcessLiveness {
    if pid == 0 {
        return ProcessLiveness::Unknown;
    }

    let kill_status = Command::new("sh")
        .args(["-c", &format!("kill -0 {pid} >/dev/null 2>&1")])
        .status();
    if kill_status.is_ok_and(|status| status.success()) {
        return ProcessLiveness::Alive;
    }

    let ps_output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output();
    let Ok(ps_output) = ps_output else {
        return ProcessLiveness::Unknown;
    };
    if !ps_output.status.success() {
        return ProcessLiveness::Unknown;
    }
    let listed_pid = String::from_utf8_lossy(&ps_output.stdout);
    if listed_pid.trim().is_empty() {
        ProcessLiveness::Dead
    } else {
        ProcessLiveness::Alive
    }
}

#[cfg(any(windows, test))]
pub(super) fn parse_process_probe_output(stdout: &[u8]) -> ProcessLiveness {
    let raw = String::from_utf8_lossy(stdout);
    let token = raw.trim().to_ascii_lowercase();
    match token.as_str() {
        "alive" => ProcessLiveness::Alive,
        "dead" => ProcessLiveness::Dead,
        _ => ProcessLiveness::Unknown,
    }
}
