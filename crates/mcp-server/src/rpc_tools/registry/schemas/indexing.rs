use serde_json::Value;

use super::common::{
    boolean_schema, const_true_schema, enum_schema, migration_mode_schema, privacy_mode_schema,
    string_array_schema, string_schema,
};
use crate::rpc_tools::registry::helpers::json_schema_object;

pub(crate) fn index_schema() -> Value {
    json_schema_object(
        &[
            (
                "profile",
                enum_schema(
                    "Indexing profile tuned for the repository mix.",
                    &["rust-monorepo", "mixed", "docs-heavy"],
                ),
            ),
            (
                "changed_since",
                string_schema(
                    "Only index files changed since this timestamp or relative duration.",
                    None,
                ),
            ),
            (
                "changed_since_commit",
                string_schema("Only index files changed since this git commit.", Some(1)),
            ),
            (
                "include_paths",
                string_array_schema("Restrict indexing scope to these paths."),
            ),
            (
                "exclude_paths",
                string_array_schema("Skip these paths during indexing."),
            ),
            (
                "reindex",
                boolean_schema("Drop existing index data before rebuilding."),
            ),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(crate) fn install_ignore_rules_schema() -> Value {
    json_schema_object(
        &[(
            "target",
            enum_schema(
                "Where to install RMU-managed ignore rules.",
                &["git-info-exclude", "root-gitignore"],
            ),
        )],
        &[],
    )
}

pub(crate) fn scope_preview_schema() -> Value {
    json_schema_object(
        &[
            (
                "profile",
                enum_schema(
                    "Indexing profile tuned for the repository mix.",
                    &["rust-monorepo", "mixed", "docs-heavy"],
                ),
            ),
            (
                "changed_since",
                string_schema(
                    "Preview files changed since this timestamp or relative duration.",
                    None,
                ),
            ),
            (
                "changed_since_commit",
                string_schema("Preview files changed since this git commit.", Some(1)),
            ),
            (
                "include_paths",
                string_array_schema("Restrict preview scope to these paths."),
            ),
            (
                "exclude_paths",
                string_array_schema("Skip these paths when building the preview."),
            ),
            (
                "reindex",
                boolean_schema("Preview a full rebuild instead of an incremental update."),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(crate) fn preflight_schema() -> Value {
    json_schema_object(
        &[
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(crate) fn db_maintenance_schema() -> Value {
    json_schema_object(
        &[
            (
                "integrity_check",
                boolean_schema("Run SQLite integrity_check on the current database."),
            ),
            (
                "checkpoint",
                boolean_schema("Checkpoint the WAL into the main database file."),
            ),
            ("vacuum", boolean_schema("Compact the database file with VACUUM.")),
            (
                "analyze",
                boolean_schema("Refresh SQLite planner statistics with ANALYZE."),
            ),
            (
                "stats",
                boolean_schema("Return maintenance-oriented SQLite statistics."),
            ),
            (
                "prune",
                boolean_schema("Delete stale derived artifacts owned by the local index."),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}

pub(crate) fn delete_index_schema() -> Value {
    json_schema_object(
        &[
            (
                "confirm",
                const_true_schema("Must be true to confirm index deletion."),
            ),
            ("migration_mode", migration_mode_schema()),
        ],
        &["confirm"],
    )
}
