use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

use anyhow::Result;
use rmu_core::{PrivacyMode, sanitize_error_message};

const JSON_FALLBACK_PATH_ENV: &str = "RMU_JSON_FALLBACK_PATH";
const JSON_AUTOMATIC_FALLBACK_FILE_NAME: &str = "rmu-cli-json-error-latest.json";
const RUNNING_BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn print_json(serialized: std::result::Result<String, serde_json::Error>) -> Result<()> {
    write_stdout_line(&serialized?)?;
    Ok(())
}

pub(crate) fn print_line(line: impl AsRef<str>) {
    println!("{}", line.as_ref());
}

pub(crate) fn print_app_error(
    json_output: bool,
    code: &str,
    error: &str,
    privacy_mode: PrivacyMode,
) -> Result<()> {
    let error = sanitize_error_message(privacy_mode, error);
    if json_output {
        write_json_error_with_fallback(&json_error_payload(code, &error))?;
    } else {
        write_stderr_line(&format!("Error: {error}"))?;
    }
    Ok(())
}

pub(crate) fn json_error_payload(code: &str, error: &str) -> String {
    let mut payload = serde_json::json!({
        "ok": false,
        "code": code,
        "error": error
    });
    if let Some(details) = compatibility_details(code, error) {
        payload["details"] = details;
    }
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
        r#"{"ok":false,"code":"E_RUNTIME","error":"internal serialization error"}"#.to_string()
    })
}

fn write_json_error_with_fallback(payload: &str) -> io::Result<()> {
    match write_stdout_line(payload) {
        Ok(()) => Ok(()),
        Err(stdout_err) => match write_stderr_line(payload) {
            Ok(()) => Ok(()),
            Err(stderr_err) => match write_json_error_to_env_file(payload) {
                Ok(()) => Ok(()),
                Err(file_err) if json_fallback_path_from_env().is_some() => {
                    write_json_error_to_automatic_file(payload).or(Err(file_err))
                }
                Err(_) => match write_json_error_to_automatic_file(payload) {
                    Ok(()) => Ok(()),
                    Err(_) if stderr_err.kind() != io::ErrorKind::BrokenPipe => Err(stderr_err),
                    Err(_) => Err(stdout_err),
                },
            },
        },
    }
}

fn write_stdout_line(line: &str) -> io::Result<()> {
    write_line(&mut io::stdout().lock(), line)
}

fn write_stderr_line(line: &str) -> io::Result<()> {
    write_line(&mut io::stderr().lock(), line)
}

fn write_line(writer: &mut impl Write, line: &str) -> io::Result<()> {
    writer.write_all(line.as_bytes())?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn write_json_error_to_env_file(payload: &str) -> io::Result<()> {
    let path = json_fallback_path_from_env().ok_or_else(|| {
        io::Error::new(io::ErrorKind::BrokenPipe, "json fallback path unavailable")
    })?;
    write_json_error_to_file(&path, payload)
}

fn json_fallback_path_from_env() -> Option<PathBuf> {
    let value = env::var_os(JSON_FALLBACK_PATH_ENV)?;
    if value.to_string_lossy().trim().is_empty() {
        return None;
    }
    Some(PathBuf::from(value))
}

fn write_json_error_to_automatic_file(payload: &str) -> io::Result<()> {
    let path = automatic_json_fallback_path();
    write_json_error_to_file(&path, payload)
}

pub(crate) fn automatic_json_fallback_path() -> PathBuf {
    env::temp_dir().join(JSON_AUTOMATIC_FALLBACK_FILE_NAME)
}

fn write_json_error_to_file(path: &PathBuf, payload: &str) -> io::Result<()> {
    fs::write(path, format!("{payload}\n"))
}

fn compatibility_details(code: &str, error: &str) -> Option<serde_json::Value> {
    if code != crate::error::CODE_COMPATIBILITY {
        return None;
    }
    Some(serde_json::json!({
        "kind": "compatibility",
        "running_binary_version": RUNNING_BINARY_VERSION,
        "safe_recovery_hint": if cfg!(windows) {
            "use scripts/rmu-mcp-server-fresh.cmd or restart the process with a fresh binary, then re-open the index"
        } else {
            "restart the process with a fresh binary and re-open the index"
        },
        "reason": error
    }))
}

#[cfg(test)]
mod tests {
    use super::{automatic_json_fallback_path, json_error_payload};

    #[test]
    fn json_error_payload_is_valid_json_object() {
        let raw = json_error_payload("E_SAMPLE", "sample error");
        let value: serde_json::Value =
            serde_json::from_str(&raw).expect("payload should be valid json");
        assert_eq!(value["ok"], serde_json::json!(false));
        assert_eq!(value["code"], serde_json::json!("E_SAMPLE"));
        assert_eq!(value["error"], serde_json::json!("sample error"));
    }

    #[test]
    fn automatic_json_fallback_path_points_to_stable_temp_file() {
        let path = automatic_json_fallback_path();
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("rmu-cli-json-error-latest.json")
        );
    }

    #[test]
    fn compatibility_payload_reports_running_binary_version_without_fake_stale_flag() {
        let raw = json_error_payload("E_COMPATIBILITY", "db newer than binary supported");
        let value: serde_json::Value =
            serde_json::from_str(&raw).expect("payload should be valid json");
        assert!(value["details"]["running_binary_version"].is_string());
        assert!(value["details"].get("stale_process_suspected").is_none());
    }
}
