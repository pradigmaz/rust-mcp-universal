use anyhow::Result;
use serde_json::Value;

use crate::ServerState;

pub(crate) fn query_benchmark(args: &Value, state: &mut ServerState) -> Result<Value> {
    super::benchmark_impl::query_benchmark(args, state)
}
