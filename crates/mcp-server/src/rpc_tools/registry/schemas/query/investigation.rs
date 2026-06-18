use serde_json::Value;

use super::super::common::{
    boolean_schema, enum_schema, integer_schema, migration_mode_schema, privacy_mode_schema,
    string_schema,
};
use crate::rpc_tools::registry::helpers::json_schema_object;

pub(crate) fn investigation_schema() -> Value {
    json_schema_object(
        &[
            (
                "seed",
                string_schema("Concept seed, symbol, path, or path:line probe.", Some(1)),
            ),
            (
                "seed_kind",
                enum_schema(
                    "How RMU should interpret the seed value.",
                    &["query", "symbol", "path", "path_line"],
                ),
            ),
            (
                "limit",
                integer_schema(
                    "Maximum number of variants or snippets to return. Defaults to 3 to keep MCP output compact.",
                    Some(1),
                ),
            ),
            (
                "auto_index",
                boolean_schema("Automatically build or refresh the index if needed."),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &["seed", "seed_kind"],
    )
}
