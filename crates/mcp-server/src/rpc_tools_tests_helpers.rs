use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::handle_tool_call;
use crate::ServerState;

#[path = "../../core/tests/support/investigation_fixture.rs"]
mod investigation_fixture;
#[path = "rpc_tools_tests_helpers/schema.rs"]
mod schema;

pub(super) fn validate_schema_keyword_coverage(
    schema_value: &Value,
    context: &str,
) -> std::result::Result<(), String> {
    schema::validate_schema_keyword_coverage(schema_value, context)
}

pub(super) fn assert_required_structure(value: &Value, schema_value: &Value, context: &str) {
    schema::assert_required_structure(value, schema_value, context);
}

pub(super) fn assert_schema_rejects(value: &Value, schema_value: &Value, context: &str) {
    schema::assert_schema_rejects(value, schema_value, context);
}

pub(super) fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

pub(super) fn state_for(project_path: PathBuf, db_path: Option<PathBuf>) -> ServerState {
    ServerState::new(project_path, db_path)
}

pub(super) use investigation_fixture::{
    write_cluster_and_divergence_fixture, write_investigation_benchmark_fixture,
    write_route_and_constraint_fixture, write_symbol_body_fixture,
};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(super) fn load_schema(file_name: &str) -> Value {
    let schema_path = repo_root().join("schemas").join(file_name);
    let raw = fs::read_to_string(&schema_path)
        .unwrap_or_else(|err| panic!("failed to read schema {}: {err}", schema_path.display()));
    serde_json::from_str(&raw)
        .unwrap_or_else(|err| panic!("failed to parse schema {}: {err}", schema_path.display()))
}

pub(super) fn assert_tool_args_error(
    name: &str,
    arguments: Value,
    expected_message_fragment: &str,
) {
    let project_dir = temp_dir("rmu-mcp-tests-invalid-args");
    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let err = handle_tool_call(
        Some(json!({
            "name": name,
            "arguments": arguments
        })),
        &mut state,
    )
    .expect_err("tool call should fail for invalid arguments");

    assert!(
        err.to_string().contains(expected_message_fragment),
        "unexpected error for `{name}`: {err}"
    );
}
