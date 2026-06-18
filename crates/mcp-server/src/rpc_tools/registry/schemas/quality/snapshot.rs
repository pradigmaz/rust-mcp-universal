use serde_json::Value;

use super::super::common::{
    boolean_schema, enum_schema, migration_mode_schema, privacy_mode_schema, string_schema,
};
use crate::rpc_tools::registry::helpers::json_schema_object;

pub(crate) fn quality_snapshot_schema() -> Value {
    json_schema_object(
        &[
            (
                "snapshot_kind",
                enum_schema(
                    "Snapshot lifecycle phase to capture for the current project.",
                    &["ad_hoc", "before", "after", "baseline"],
                ),
            ),
            (
                "wave_id",
                string_schema(
                    "Wave identifier used for before/after history and wave_before comparisons.",
                    Some(1),
                ),
            ),
            (
                "output_root",
                string_schema(
                    "Optional custom artifact root for quality snapshots and wave history. Relative paths resolve under the bound project root.",
                    Some(1),
                ),
            ),
            (
                "compare_against",
                enum_schema(
                    "Comparison basis for delta and regression gate output.",
                    &["none", "self_baseline", "wave_before"],
                ),
            ),
            (
                "auto_index",
                boolean_schema("Automatically build or refresh the index if needed."),
            ),
            (
                "persist_artifacts",
                boolean_schema(
                    "Persist snapshot artifacts under the canonical baseline and .codex wave history directories.",
                ),
            ),
            (
                "promote_self_baseline",
                boolean_schema(
                    "Update the committed self baseline under baseline/quality/self after capture.",
                ),
            ),
            (
                "fail_on_regression",
                boolean_schema(
                    "Return a tool error when post-refresh status is not ready or new violations were introduced.",
                ),
            ),
            ("privacy_mode", privacy_mode_schema()),
            ("migration_mode", migration_mode_schema()),
        ],
        &[],
    )
}
