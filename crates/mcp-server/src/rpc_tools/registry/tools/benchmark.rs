use serde_json::Value;

use super::super::helpers::tool;
use super::super::schemas::query_benchmark_schema;

pub(super) fn tools() -> Vec<Value> {
    vec![tool(
        "query_benchmark",
        "Run query benchmark (legacy metrics or baseline-vs-candidate compare mode)",
        query_benchmark_schema(),
    )]
}
