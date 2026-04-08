use serde_json::Value;

use super::common::{
    boolean_schema, integer_schema, migration_mode_schema, privacy_mode_schema, string_schema,
};
use crate::rpc_tools::registry::helpers::json_schema_object;

pub(crate) fn sensitive_data_schema() -> Value {
    json_schema_object(
        &[
            (
                "limit",
                integer_schema(
                    "Maximum number of sensitive-data findings to return.",
                    Some(1),
                ),
            ),
            (
                "path_prefix",
                string_schema("Only include files under this path prefix.", Some(1)),
            ),
            (
                "include_low_confidence",
                boolean_schema("Include low and medium confidence assignment-style findings."),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(crate) fn signal_memory_schema() -> Value {
    json_schema_object(
        &[
            (
                "limit",
                integer_schema("Maximum number of remembered decisions to return.", Some(1)),
            ),
            (
                "finding_family",
                string_schema(
                    "Filter to one finding family: ordinary, dead_code, security_smells, sensitive_data.",
                    Some(1),
                ),
            ),
            (
                "decision",
                string_schema("Filter to one decision: useful or noisy.", Some(1)),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(crate) fn mark_signal_memory_schema() -> Value {
    json_schema_object(
        &[
            (
                "signal_key",
                string_schema(
                    "Stable signal identifier from a prior result payload.",
                    Some(1),
                ),
            ),
            (
                "finding_family",
                string_schema(
                    "Finding family for the remembered decision: ordinary, dead_code, security_smells, sensitive_data.",
                    Some(1),
                ),
            ),
            (
                "scope",
                string_schema("Optional scope label for the remembered decision.", Some(1)),
            ),
            (
                "decision",
                string_schema("Decision to persist: useful or noisy.", Some(1)),
            ),
            (
                "reason",
                string_schema("Human-readable reason for this memory entry.", Some(1)),
            ),
            (
                "source",
                string_schema("Decision source label, for example `manual`.", Some(1)),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &["signal_key", "finding_family", "decision", "reason"],
    )
}
