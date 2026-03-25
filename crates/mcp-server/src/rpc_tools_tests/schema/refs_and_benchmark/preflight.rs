use serde_json::json;

use super::super::*;

#[test]
fn preflight_status_schema_covers_supported_keywords_and_shapes() {
    let schema = load_schema("preflight_status.schema.json");
    validate_schema_keyword_coverage(&schema, "preflight.keyword-coverage")
        .expect("schema keyword coverage");

    let valid_payload = json!({
        "status": "warning",
        "project_path": "/tmp/project",
        "binary_path": "/tmp/bin/rmu-mcp-server",
        "running_binary_version": "0.1.0",
        "running_binary_stale": false,
        "stale_process_probe_binary_path": "/tmp/bin/rmu-mcp-server",
        "supported_schema_version": 11,
        "db_schema_version": 12,
        "index_format_version": 3,
        "ann_version": 1,
        "same_binary_other_pids": [42],
        "stale_process_suspected": true,
        "launcher_recommended": "scripts/rmu-mcp-server-fresh.cmd",
        "safe_recovery_hint": "use fresh launcher",
        "errors": ["db newer than binary supported"]
    });
    assert_required_structure(&valid_payload, &schema, "preflight.valid");

    let mut invalid_status = valid_payload.clone();
    invalid_status["status"] = json!("unexpected");
    assert_schema_rejects(&invalid_status, &schema, "preflight.invalid-status");

    let mut missing_errors = valid_payload;
    missing_errors
        .as_object_mut()
        .expect("object payload")
        .remove("errors");
    assert_schema_rejects(&missing_errors, &schema, "preflight.missing-errors");
}
