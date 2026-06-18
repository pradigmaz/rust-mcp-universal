use serde_json::Value;

use super::super::helpers::tool;
use super::super::schemas::db_maintenance_schema;

pub(super) fn tools() -> Vec<Value> {
    vec![tool(
        "db_maintenance",
        "Run SQLite maintenance tasks (integrity/checkpoint/vacuum/analyze/stats/prune)",
        db_maintenance_schema(),
    )]
}
