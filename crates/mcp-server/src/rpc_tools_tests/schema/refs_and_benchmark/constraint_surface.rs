use serde_json::json;

use super::super::*;

#[test]
fn constraint_evidence_schema_accepts_canonical_and_legacy_fields() {
    let constraint_schema = load_schema("constraint_evidence.schema.json");
    let constraint_envelope = load_schema("mcp_constraint_evidence_tool_result.schema.json");

    validate_schema_keyword_coverage(&constraint_schema, "investigation.constraint.schema")
        .expect("keyword coverage");
    validate_schema_keyword_coverage(&constraint_envelope, "investigation.constraint.envelope")
        .expect("keyword coverage");

    let constraint_payload = json!({
        "seed": {"seed": "resolve_origin", "seed_kind": "query"},
        "items": [
            {
                "constraint_kind": "index_constraint",
                "source_kind": "index_declaration",
                "path": "migrations/001_create_origins.sql",
                "line_start": 1,
                "line_end": 1,
                "excerpt": "CREATE UNIQUE INDEX uq_origins_origin_key ON origins(origin_key);",
                "confidence": 1.0,
                "normalized_key": "index_constraint:index_declaration:create unique index uq_origins_origin_key on origins(origin_key);",
                "kind": "index_constraint",
                "strength": "strong",
                "scope": "database",
                "source_path": "migrations/001_create_origins.sql",
                "source_span": {"start_line": 1, "end_line": 1, "start_column": 1},
                "normalized_text": "CREATE UNIQUE INDEX uq_origins_origin_key ON origins(origin_key);"
            }
        ],
        "capability_status": "supported",
        "unsupported_sources": [],
        "confidence": 1.0
    });

    assert_required_structure(
        &constraint_payload,
        &constraint_schema,
        "investigation.constraint.valid",
    );
    assert_required_structure(
        &json!({
            "content": [{"type": "text", "text": "ok"}],
            "structuredContent": constraint_payload,
            "isError": false
        }),
        &constraint_envelope,
        "investigation.constraint.envelope.valid",
    );
}
