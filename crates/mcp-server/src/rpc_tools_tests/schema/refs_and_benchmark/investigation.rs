use serde_json::json;

use super::super::*;

#[test]
fn investigation_benchmark_report_schema_covers_threshold_and_non_threshold_shapes() {
    let schema = load_schema("investigation_benchmark_report.schema.json");
    validate_schema_keyword_coverage(&schema, "investigation-benchmark.keyword-coverage")
        .expect("schema keyword coverage");

    let valid_with_thresholds = json!({
        "dataset_path": "/tmp/investigation_dataset.json",
        "limit": 5,
        "case_count": 2,
        "per_tool_metrics": [
            {
                "tool": "divergence_report",
                "case_count": 1,
                "passed_cases": 1,
                "pass_rate": 1.0,
                "unsupported_case_rate": 0.0,
                "latency_p50_ms": 10.0,
                "latency_p95_ms": 10.0
            }
        ],
        "cases": [
            {
                "id": "divergence-origin",
                "tool": "divergence_report",
                "fixture": "tmp",
                "pass": true,
                "assertion_pass_count": 2,
                "assertion_total_count": 2,
                "capability_status": "supported",
                "expected_capability_status": "supported",
                "unsupported_sources": [],
                "privacy_failures": 0,
                "latency_ms": 8.5,
                "notes": []
            }
        ],
        "unsupported_behavior_summary": [],
        "privacy_failures": 0,
        "threshold_verdict": {
            "passed": true,
            "failures": []
        },
        "diff": {
            "baseline_case_count": 2,
            "current_case_count": 2,
            "per_tool_deltas": [
                {
                    "tool": "divergence_report",
                    "metrics": [
                        {
                            "tool": "divergence_report",
                            "metric": "latency_p95_ms",
                            "expectation": "lower_is_better",
                            "baseline": 10.0,
                            "current": 11.0,
                            "delta": 1.0,
                            "delta_ratio": 0.1
                        }
                    ]
                }
            ],
            "regressed_metrics": [],
            "improved_metrics": [],
            "regression_failures": []
        }
    });
    assert_required_structure(
        &valid_with_thresholds,
        &schema,
        "investigation-benchmark.valid.thresholds",
    );

    let mut valid_without_thresholds = valid_with_thresholds.clone();
    valid_without_thresholds
        .as_object_mut()
        .expect("object payload")
        .remove("threshold_verdict");
    assert_required_structure(
        &valid_without_thresholds,
        &schema,
        "investigation-benchmark.valid.no-thresholds",
    );

    let mut valid_without_diff = valid_without_thresholds.clone();
    valid_without_diff
        .as_object_mut()
        .expect("object payload")
        .remove("diff");
    assert_required_structure(
        &valid_without_diff,
        &schema,
        "investigation-benchmark.valid.no-diff",
    );

    let mut invalid_tool = valid_with_thresholds;
    invalid_tool["per_tool_metrics"][0]["tool"] = json!("unexpected_tool");
    assert_schema_rejects(
        &invalid_tool,
        &schema,
        "investigation-benchmark.invalid-tool",
    );

    let mut invalid_diff_tool = valid_without_thresholds;
    invalid_diff_tool["diff"]["per_tool_deltas"][0]["tool"] = json!("unexpected_tool");
    assert_schema_rejects(
        &invalid_diff_tool,
        &schema,
        "investigation-benchmark.invalid-diff-tool",
    );
}

#[test]
fn investigation_result_schemas_accept_expected_shapes_and_reject_invalid_values() {
    let symbol_schema = load_schema("symbol_body.schema.json");
    let route_schema = load_schema("route_trace.schema.json");
    let cluster_schema = load_schema("concept_cluster.schema.json");
    let divergence_schema = load_schema("divergence_report.schema.json");
    let symbol_envelope = load_schema("mcp_symbol_body_tool_result.schema.json");
    let route_envelope = load_schema("mcp_route_trace_tool_result.schema.json");
    let cluster_envelope = load_schema("mcp_concept_cluster_tool_result.schema.json");
    let divergence_envelope = load_schema("mcp_divergence_report_tool_result.schema.json");

    validate_schema_keyword_coverage(&symbol_schema, "investigation.symbol.schema")
        .expect("keyword coverage");
    validate_schema_keyword_coverage(&route_schema, "investigation.route.schema")
        .expect("keyword coverage");
    validate_schema_keyword_coverage(&cluster_schema, "investigation.cluster.schema")
        .expect("keyword coverage");
    validate_schema_keyword_coverage(&divergence_schema, "investigation.divergence.schema")
        .expect("keyword coverage");
    validate_schema_keyword_coverage(&symbol_envelope, "investigation.symbol.envelope")
        .expect("keyword coverage");
    validate_schema_keyword_coverage(&route_envelope, "investigation.route.envelope")
        .expect("keyword coverage");
    validate_schema_keyword_coverage(&cluster_envelope, "investigation.cluster.envelope")
        .expect("keyword coverage");
    validate_schema_keyword_coverage(&divergence_envelope, "investigation.divergence.envelope")
        .expect("keyword coverage");

    let symbol_payload = json!({
        "seed": {"seed": "inspect_body", "seed_kind": "symbol"},
        "items": [
            {
                "anchor": {"path": "src/lib.rs", "language": "rust", "symbol": "inspect_body", "line": 1},
                "signature": "pub fn inspect_body() {",
                "body": "pub fn inspect_body() {\n    println!(\"ok\");\n}",
                "span": {"start_line": 1, "end_line": 3, "start_column": 1},
                "source_kind": "symbol_lookup",
                "resolution_kind": "exact_symbol_span",
                "truncated": false,
                "confidence": 1.0
            }
        ],
        "capability_status": "supported",
        "unsupported_sources": [],
        "ambiguity_status": "none",
        "confidence": 1.0
    });
    assert_required_structure(
        &symbol_payload,
        &symbol_schema,
        "investigation.symbol.valid",
    );
    assert_required_structure(
        &json!({
            "content": [{"type": "text", "text": "ok"}],
            "structuredContent": symbol_payload,
            "isError": false
        }),
        &symbol_envelope,
        "investigation.symbol.envelope.valid",
    );
    let mut invalid_symbol = json!({
        "seed": {"seed": "inspect_body", "seed_kind": "symbol"},
        "items": [],
        "capability_status": "supported",
        "unsupported_sources": [],
        "ambiguity_status": "none",
        "confidence": 1.2
    });
    assert_schema_rejects(
        &invalid_symbol,
        &symbol_schema,
        "investigation.symbol.invalid_confidence",
    );
    invalid_symbol["confidence"] = json!(1.0);
    invalid_symbol
        .as_object_mut()
        .expect("object payload")
        .remove("ambiguity_status");
    assert_schema_rejects(
        &invalid_symbol,
        &symbol_schema,
        "investigation.symbol.missing_ambiguity_status",
    );

    let route_payload = json!({
        "seed": {"seed": "resolve_origin", "seed_kind": "query"},
        "best_route": {
            "segments": [
                {
                    "kind": "service",
                    "path": "src/services/origin_service.rs",
                    "language": "rust",
                    "evidence": "service references validator",
                    "relation_kind": "contains",
                    "source_kind": "service",
                    "score": 1.0
                }
            ],
            "total_hops": 0,
            "total_weight": 0.0,
            "collapsed_hops": 0,
            "confidence": 0.9
        },
        "alternate_routes": [],
        "unresolved_gaps": [],
        "capability_status": "supported",
        "unsupported_sources": [],
        "confidence": 0.9
    });
    assert_required_structure(&route_payload, &route_schema, "investigation.route.valid");
    assert_required_structure(
        &json!({
            "content": [{"type": "text", "text": "ok"}],
            "structuredContent": route_payload,
            "isError": false
        }),
        &route_envelope,
        "investigation.route.envelope.valid",
    );

    let cluster_payload = json!({
        "seed": {"seed": "resolve_origin", "seed_kind": "query"},
        "variants": [
            {
                "id": "variant-1",
                "entry_anchor": {"path": "src/services/origin_service.rs", "language": "rust"},
                "route": [],
                "constraints": [],
                "related_tests": [],
                "confidence": 0.8,
                "gaps": ["needs migration evidence"]
            }
        ],
        "cluster_summary": {
            "variant_count": 1,
            "languages": ["rust"],
            "route_kinds": []
        },
        "gaps": ["needs migration evidence"],
        "capability_status": "partial",
        "unsupported_sources": [],
        "confidence": 0.8
    });
    assert_required_structure(
        &cluster_payload,
        &cluster_schema,
        "investigation.cluster.valid",
    );
    assert_required_structure(
        &json!({
            "content": [{"type": "text", "text": "ok"}],
            "structuredContent": cluster_payload,
            "isError": false
        }),
        &cluster_envelope,
        "investigation.cluster.envelope.valid",
    );

    let divergence_payload = json!({
        "surface_kind": "divergence_explainability",
        "seed": {"seed": "resolve_origin", "seed_kind": "query"},
        "variants": [
            {
                "id": "variant-1",
                "entry_anchor": {"path": "src/services/origin_service.rs", "language": "rust"},
                "route": [],
                "constraints": [],
                "related_tests": [],
                "confidence": 0.8,
                "gaps": []
            }
        ],
        "consensus_axes": [
            {
                "axis": "language",
                "values": [{"variant_id": "variant-1", "values": ["rust"]}]
            }
        ],
        "divergence_axes": [
            {
                "axis": "constraints",
                "values": [{"variant_id": "variant-1", "values": ["runtime_guard"]}]
            }
        ],
        "divergence_signals": [
            {
                "severity": "likely_expected",
                "axis": "constraints",
                "evidence_strength": "weak",
                "classification_reason": "difference is limited to non-authoritative constraint evidence",
                "summary": "constraint evidence differs",
                "variant_ids": ["variant-1"]
            }
        ],
        "overall_severity": "likely_expected",
        "manual_review_required": false,
        "summary": "1 variants, 1 divergence axes; highest severity likely_expected (no material risk axes identified)",
        "shared_evidence": ["validator present"],
        "unknowns": [],
        "missing_evidence": ["missing db constraint"],
        "recommended_followups": ["Collect additional evidence for unresolved gaps before treating this divergence as a bug."],
        "actionability": {
            "recommended_target_path": "src/services/origin_service.rs",
            "recommended_target_role": "service",
            "reason": "highest_confidence_contract_target",
            "next_steps": [
                {"kind": "inspect_primary_target", "detail": "Inspect and update primary service target"}
            ],
            "related_tests": [],
            "adjacent_paths": ["src/validators/origin_validator.rs"],
            "checks": ["review_adjacent_impact"],
            "rollback_sensitive_paths": [],
            "manual_review_required": false
        },
        "overall_confidence": 0.85,
        "capability_status": "supported",
        "unsupported_sources": []
    });
    assert_required_structure(
        &divergence_payload,
        &divergence_schema,
        "investigation.divergence.valid",
    );
    assert_required_structure(
        &json!({
            "content": [{"type": "text", "text": "ok"}],
            "structuredContent": divergence_payload,
            "isError": false
        }),
        &divergence_envelope,
        "investigation.divergence.envelope.valid",
    );
}

#[test]
fn contract_trace_schema_accepts_expected_shape_and_envelope() {
    let schema = load_schema("contract_trace.schema.json");
    let envelope = load_schema("mcp_contract_trace_tool_result.schema.json");

    validate_schema_keyword_coverage(&schema, "investigation.contract_trace.schema")
        .expect("keyword coverage");
    validate_schema_keyword_coverage(&envelope, "investigation.contract_trace.envelope")
        .expect("keyword coverage");

    let payload = json!({
        "seed": {"seed": "origin_resolution", "seed_kind": "query"},
        "chain": [
            {
                "role": "generated_client",
                "anchor": {"path": "src/generated/origin_client.generated.ts", "language": "typescript"},
                "source_kind": "api_client",
                "evidence": "entry_anchor",
                "confidence": 0.8,
                "generated_lineage": {
                    "status": "generated",
                    "detection_basis": "path_convention",
                    "source_of_truth_path": "src/services/origin_service.rs",
                    "source_of_truth_kind": "upstream_contract",
                    "confidence": 0.9
                },
                "rank_score": 0.4,
                "rank_reason": "route_segment_role_priority"
            }
        ],
        "contract_breaks": [
            {
                "expected_role": "test",
                "reason": "related_tests_not_found"
            }
        ],
        "actionability": {
            "recommended_target_path": "src/services/origin_service.rs",
            "recommended_target_role": "schema_or_model",
            "reason": "generated_artifact_redirected_to_source_of_truth",
            "next_steps": [
                {"kind": "inspect_primary_target", "detail": "Inspect source of truth"}
            ],
            "related_tests": ["tests/test_origin_resolution.py"],
            "adjacent_paths": ["src/generated/origin_client.generated.ts"],
            "checks": ["run_related_tests"],
            "rollback_sensitive_paths": ["migrations/001_create_origins.sql"],
            "manual_review_required": true
        },
        "manual_review_required": true,
        "capability_status": "partial",
        "unsupported_sources": [],
        "confidence": 0.82
    });
    assert_required_structure(&payload, &schema, "investigation.contract_trace.valid");
    assert_required_structure(
        &json!({
            "content": [{"type": "text", "text": "ok"}],
            "structuredContent": payload,
            "isError": false
        }),
        &envelope,
        "investigation.contract_trace.envelope.valid",
    );
}