use serde_json::Value;

use super::super::helpers::tool;
use super::super::schemas::{
    mark_signal_memory_schema, quality_hotspots_schema, quality_snapshot_schema,
    rule_violations_schema, sensitive_data_schema, signal_memory_schema,
};

pub(super) fn tools() -> Vec<Value> {
    vec![
        tool(
            "rule_violations",
            "Report compact persisted file-level quality violations from the quality index; pass details=true for full fields",
            rule_violations_schema(),
        ),
        tool(
            "quality_hotspots",
            "Report compact aggregated quality hotspots across file, directory, or module buckets; pass details=true for full fields",
            quality_hotspots_schema(),
        ),
        tool(
            "quality_snapshot",
            "Capture a fresh project quality snapshot, persist debt-wave artifacts, and optionally compare against baseline or wave_before",
            quality_snapshot_schema(),
        ),
        tool(
            "sensitive_data",
            "Run dedicated security-sensitive data scan over repo files without mixing results into ordinary quality scoring",
            sensitive_data_schema(),
        ),
        tool(
            "signal_memory",
            "Inspect repo-local remembered useful/noisy signal decisions",
            signal_memory_schema(),
        ),
        tool(
            "mark_signal_memory",
            "Persist a repo-local useful/noisy decision for a specific signal key",
            mark_signal_memory_schema(),
        ),
    ]
}
