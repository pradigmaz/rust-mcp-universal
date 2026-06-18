use serde_json::Value;

use super::super::helpers::tool;
use super::super::schemas::{delete_index_schema, index_schema, scope_preview_schema};

pub(super) fn tools() -> Vec<Value> {
    vec![
        tool(
            "index",
            "Rebuild file index including semantic vectors",
            index_schema(),
        ),
        tool(
            "semantic_index",
            "Alias for semantic index rebuild",
            index_schema(),
        ),
        tool(
            "scope_preview",
            "Preview effective scope and candidate paths before indexing",
            scope_preview_schema(),
        ),
        tool(
            "delete_index",
            "Delete index storage files for current project",
            delete_index_schema(),
        ),
    ]
}
