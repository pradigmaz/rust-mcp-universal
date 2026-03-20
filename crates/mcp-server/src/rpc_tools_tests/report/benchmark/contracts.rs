use serde_json::Value;

use super::*;

pub(super) fn assert_query_benchmark_baseline_contracts(result: &Value) {
    let envelope_schema = load_schema("mcp_query_benchmark_tool_result.schema.json");
    let report_schema = load_schema("query_benchmark_report.schema.json");
    assert_required_structure(result, &envelope_schema, "benchmark.mcp_result");
    assert_required_structure(
        &result["structuredContent"],
        &report_schema,
        "benchmark.mcp_result.structuredContent",
    );
}

pub(super) fn assert_query_benchmark_compare_contracts(result: &Value) {
    let envelope_schema = load_schema("mcp_query_benchmark_tool_result.schema.json");
    let report_schema = load_schema("query_benchmark_report.schema.json");
    assert_required_structure(result, &envelope_schema, "benchmark_compare.mcp_result");
    assert_required_structure(
        &result["structuredContent"],
        &report_schema,
        "benchmark_compare.mcp_result.structuredContent",
    );
}
