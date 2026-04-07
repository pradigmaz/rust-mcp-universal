use serde_json::json;

use super::super::*;

#[test]
fn local_schema_checker_applies_ref_and_sibling_constraints() {
    let report_schema = load_schema("query_report.schema.json");
    let report_value = json!({
        "query_id": "q-1",
        "timestamp_utc": "2026-03-02T00:00:00Z",
        "project_root": "/tmp/project",
        "resolved_mode": "entrypoint_map",
        "mode_source": "default",
        "budget": {
            "max_tokens": 100,
            "used_estimate": 42,
            "hard_truncated": false
        },
        "retrieval_pipeline": [
            {"stage": "lexical", "candidates": 3, "kept": 2}
        ],
        "selected_context": [
            {
                "path": "src/lib.rs",
                "score": 0.7,
                "chars": 120,
                "chunk_idx": 2,
                "start_line": 10,
                "end_line": 24,
                "chunk_source": "chunk_embedding_index",
                "why": ["lexical_match"],
                "explain": {
                    "lexical": 0.5,
                    "graph": 0.1,
                    "semantic": 0.2,
                    "rrf": 0.3,
                    "graph_rrf": 0.05,
                    "rank_before": 2,
                    "rank_after": 1,
                    "semantic_source": "indexed",
                    "semantic_outcome": "applied_indexed",
                    "graph_seed_path": "src/main.rs",
                    "graph_edge_kinds": ["outgoing:ref_exact"],
                    "graph_hops": 1
                },
                "provenance": {
                    "basis": "indexed",
                    "derivation": "context_selection",
                    "freshness": "index_snapshot",
                    "strength": "strong",
                    "reasons": ["indexed_chunk"]
                }
            }
        ],
        "provenance": {
            "basis": "indexed",
            "derivation": "query_report",
            "freshness": "index_snapshot",
            "strength": "strong",
            "reasons": ["indexed_chunk"]
        },
        "confidence": {
            "overall": 0.8,
            "reasons": ["enough signal"],
            "signals": {
                "margin_top1_top2": 0.2,
                "explain_coverage": 1.0,
                "semantic_coverage": 1.0,
                "semantic_outcome": "applied_indexed",
                "stage_drop_ratio": 0.1,
                "hard_truncated": false
            }
        },
        "gaps": [],
        "index_telemetry": {
            "last_index_lock_wait_ms": 0,
            "last_embedding_cache_hits": 0,
            "last_embedding_cache_misses": 0,
            "chunk_coverage": 1.0,
            "chunk_source": "chunk_embedding_index"
        },
        "degradation_reasons": [],
        "deepen_available": false
    });
    assert_required_structure(&report_value, &report_schema, "ref.base-valid");

    let schema_with_sibling = json!({
        "$ref": "./query_report.schema.json",
        "type": "object",
        "maxProperties": 1
    });
    assert_schema_rejects(
        &report_value,
        &schema_with_sibling,
        "ref.sibling.max_properties",
    );
}
