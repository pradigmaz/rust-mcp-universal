use serde_json::Value;

use super::common::{
    boolean_schema, integer_schema, migration_mode_schema, privacy_mode_schema, string_schema,
};
use crate::rpc_tools::registry::helpers::json_schema_object;

pub(crate) fn navigation_schema(arg_name: &str) -> Value {
    let description = match arg_name {
        "name" => "Symbol name to resolve.",
        "path" => "Repository-relative path to inspect.",
        _ => "Lookup key for the navigation request.",
    };

    json_schema_object(
        &[
            (arg_name, string_schema(description, Some(1))),
            ("limit", integer_schema("Maximum number of hits to return.", Some(1))),
            (
                "auto_index",
                boolean_schema("Automatically build or refresh the index if needed."),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[arg_name],
    )
}

pub(crate) fn call_path_schema() -> Value {
    json_schema_object(
        &[
            ("from", string_schema("Start symbol or file for the path search.", Some(1))),
            ("to", string_schema("Destination symbol or file for the path search.", Some(1))),
            (
                "max_hops",
                integer_schema("Maximum number of graph hops to explore.", Some(1)),
            ),
            (
                "auto_index",
                boolean_schema("Automatically build or refresh the index if needed."),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &["from", "to"],
    )
}
