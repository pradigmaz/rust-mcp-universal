pub(super) fn compatibility_hint() -> String {
    if cfg!(windows) {
        "use scripts/rmu-mcp-server-fresh.cmd so the server is rebuilt/restarted if needed, then re-open the index".to_string()
    } else {
        "restart the process with a fresh binary and re-open the index".to_string()
    }
}

pub(super) fn stale_running_binary(running_binary_version: &str) -> String {
    format!(
        "running binary version `{running_binary_version}` is stale: executable was rebuilt after process start; restart with a fresh binary before serving requests"
    )
}
