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
                    "Maximum number of sensitive-data findings to return. Defaults to 3 to keep MCP output compact.",
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
