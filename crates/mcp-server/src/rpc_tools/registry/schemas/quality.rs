use serde_json::Value;

use super::common::{
    boolean_schema, enum_schema, integer_schema, migration_mode_schema, privacy_mode_schema,
    string_array_schema, string_schema,
};
use crate::rpc_tools::registry::helpers::json_schema_object;

pub(crate) fn rule_violations_schema() -> Value {
    json_schema_object(
        &[
            ("limit", integer_schema("Maximum number of entries to return.", Some(1))),
            (
                "path_prefix",
                string_schema("Only include files under this path prefix.", Some(1)),
            ),
            (
                "language",
                string_schema("Only include files detected as this language.", Some(1)),
            ),
            (
                "rule_ids",
                string_array_schema("Filter to these quality rule identifiers."),
            ),
            (
                "metric_ids",
                string_array_schema("Filter to violations tied to these metric identifiers."),
            ),
            (
                "sort_metric_id",
                string_schema("Metric identifier to use when sorting by metric_value.", Some(1)),
            ),
            (
                "sort_by",
                enum_schema(
                    "How to sort returned violations.",
                    &["violation_count", "size_bytes", "non_empty_lines", "metric_value"],
                ),
            ),
            (
                "auto_index",
                boolean_schema("Automatically build or refresh the index if needed."),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(crate) fn quality_hotspots_schema() -> Value {
    json_schema_object(
        &[
            (
                "aggregation",
                enum_schema(
                    "Bucket type used to aggregate hotspots.",
                    &["file", "directory", "module"],
                ),
            ),
            ("limit", integer_schema("Maximum number of buckets to return.", Some(1))),
            (
                "path_prefix",
                string_schema("Only include hotspots under this path prefix.", Some(1)),
            ),
            (
                "language",
                string_schema("Only include hotspots for this language.", Some(1)),
            ),
            (
                "rule_ids",
                string_array_schema("Only count these quality rule identifiers."),
            ),
            (
                "sort_by",
                enum_schema(
                    "How to rank hotspot buckets.",
                    &["hotspot_score", "risk_score_delta", "new_violations"],
                ),
            ),
            (
                "auto_index",
                boolean_schema("Automatically build or refresh the index if needed."),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}
