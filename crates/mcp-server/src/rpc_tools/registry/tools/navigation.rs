use serde_json::Value;

use super::super::helpers::tool;
use super::super::schemas::{call_path_schema, investigation_schema, navigation_schema};

pub(super) fn tools() -> Vec<Value> {
    vec![
        tool(
            "symbol_lookup",
            "Compatibility-only legacy navigation tool: find symbol definitions by exact or partial name",
            navigation_schema("name"),
        ),
        tool(
            "symbol_lookup_v2",
            "Canonical navigation contract: find symbol definitions in result.structuredContent.hits",
            navigation_schema("name"),
        ),
        tool(
            "symbol_references",
            "Compatibility-only legacy navigation tool: find files that reference a symbol",
            navigation_schema("name"),
        ),
        tool(
            "symbol_references_v2",
            "Canonical navigation contract: find symbol references in result.structuredContent.hits",
            navigation_schema("name"),
        ),
        tool(
            "symbol_body",
            "Resolve symbol/path/query seed into body snippets with typed anchors",
            investigation_schema(),
        ),
        tool(
            "related_files",
            "Compatibility-only legacy navigation tool: show files adjacent by dependency and call relationships",
            navigation_schema("path"),
        ),
        tool(
            "related_files_v2",
            "Canonical navigation contract: show related files in result.structuredContent.hits",
            navigation_schema("path"),
        ),
        tool(
            "call_path",
            "Find a bounded heuristic path over refs/deps between two endpoints",
            call_path_schema(),
        ),
        tool(
            "route_trace",
            "Build implementation variants with typed route segments for a conceptual seed",
            investigation_schema(),
        ),
        tool(
            "constraint_evidence",
            "Collect normalized schema/model/migration constraint evidence for a conceptual seed",
            investigation_schema(),
        ),
        tool(
            "concept_cluster",
            "Cluster multiple implementation variants around a conceptual seed",
            investigation_schema(),
        ),
        tool(
            "contract_trace",
            "Trace cross-layer contract roots, generated lineage, and actionable next steps for a conceptual seed",
            investigation_schema(),
        ),
        tool(
            "divergence_report",
            "Compare implementation variants and surface divergence axes for a conceptual seed",
            investigation_schema(),
        ),
    ]
}
