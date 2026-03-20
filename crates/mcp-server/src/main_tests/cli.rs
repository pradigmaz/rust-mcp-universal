use std::path::PathBuf;

use clap::Parser;

use crate::App;

#[test]
fn cli_accepts_project_path_flag_without_explicit_value() {
    let app = App::try_parse_from(["rmu-mcp-server", "--project-path"])
        .expect("project path flag without value should default to current dir");
    assert_eq!(app.project_path, PathBuf::from("."));
}

#[test]
fn cli_accepts_transport_stdio_flag_for_client_compatibility() {
    let app = App::try_parse_from(["rmu-mcp-server", "--transport", "stdio"])
        .expect("transport flag should be accepted");
    assert_eq!(app.transport.as_deref(), Some("stdio"));
    app.validate_runtime_flags()
        .expect("stdio should be accepted by runtime");
}

#[test]
fn runtime_rejects_unsupported_transport_and_network_flags() {
    assert!(App::try_parse_from(["rmu-mcp-server", "--transport", "http"]).is_err());
    assert!(App::try_parse_from(["rmu-mcp-server", "--port", "8080"]).is_err());
}
