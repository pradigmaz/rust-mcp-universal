use crate::model::{
    ConstraintEvidence, InvestigationAnchor, RouteSegment, SemanticState, VariantScoreBreakdown,
};

use super::*;

#[test]
fn shared_strong_constraints_keep_only_common_entries() {
    let variants = vec![
        variant_with(
            "left",
            vec![strong_constraint("uq_origin")],
            vec!["test_left"],
        ),
        variant_with(
            "right",
            vec![
                strong_constraint("uq_origin"),
                strong_constraint("uq_other"),
            ],
            vec!["test_right"],
        ),
    ];

    assert_eq!(
        shared_strong_constraints(&variants),
        vec!["uq_origin".to_string()]
    );
}

#[test]
fn classify_signal_marks_entrypoints_as_informational_proxy_only() {
    let variants = vec![variant_with("left", Vec::new(), vec!["test_left"])];
    let classification = classify_signal("entrypoints", &[], false, &variants, 0);

    assert_eq!(classification.severity, "informational");
    assert_eq!(classification.evidence_strength, "proxy_only");
    assert_eq!(classification.classification_reason, "entrypoint_only");
}

#[test]
fn classify_signal_marks_db_query_divergence_without_tests_as_high_risk() {
    let variants = vec![
        variant_with("left", Vec::new(), Vec::<&str>::new()),
        variant_with("right", Vec::new(), vec!["test_right"]),
    ];
    let classification = classify_signal("db_entities_and_queries", &[], true, &variants, 1);

    assert_eq!(classification.severity, "high_risk");
    assert_eq!(classification.evidence_strength, "corroborated_proxy");
    assert_eq!(
        classification.classification_reason,
        "db_query_without_test_backing"
    );
}

#[test]
fn classify_signal_keeps_single_guard_divergence_likely_expected_without_corroboration() {
    let variants = vec![
        variant_with(
            "left",
            vec![strong_constraint("uq_origin")],
            vec!["test_left"],
        ),
        variant_with(
            "right",
            vec![strong_constraint("uq_origin")],
            vec!["test_right"],
        ),
    ];
    let classification = classify_signal(
        "guards_and_validators",
        &["uq_origin".to_string()],
        false,
        &variants,
        1,
    );

    assert_eq!(classification.severity, "likely_expected");
    assert_eq!(classification.evidence_strength, "proxy_only");
    assert_eq!(
        classification.classification_reason,
        "single_axis_proxy_only"
    );
}

#[test]
fn classify_signal_marks_multi_axis_proxy_divergence_as_suspicious() {
    let variants = vec![
        variant_with("left", Vec::new(), vec!["test_left"]),
        variant_with("right", Vec::new(), vec!["test_right"]),
    ];
    let classification = classify_signal("guards_and_validators", &[], false, &variants, 2);

    assert_eq!(classification.severity, "suspicious");
    assert_eq!(classification.evidence_strength, "corroborated_proxy");
    assert_eq!(
        classification.classification_reason,
        "multi_axis_proxy_corroboration"
    );
}

#[test]
fn classify_signal_marks_conflicting_strong_constraints_as_high_risk() {
    let variants = vec![
        variant_with(
            "left",
            vec![strong_constraint("uq_left")],
            vec!["test_left"],
        ),
        variant_with(
            "right",
            vec![strong_constraint("uq_right")],
            vec!["test_right"],
        ),
    ];
    let classification = classify_signal("constraint_evidence", &[], false, &variants, 1);

    assert_eq!(classification.severity, "high_risk");
    assert_eq!(classification.evidence_strength, "hard");
    assert_eq!(
        classification.classification_reason,
        "conflicting_hard_constraints"
    );
}

#[test]
fn classify_signal_treats_downstream_symbol_difference_as_likely_expected_with_shared_backing() {
    let variants = vec![
        variant_with(
            "left",
            vec![strong_constraint("uq_origin")],
            vec!["test_left"],
        ),
        variant_with(
            "right",
            vec![strong_constraint("uq_origin")],
            vec!["test_right"],
        ),
    ];
    let classification = classify_signal(
        "downstream_symbols",
        &["uq_origin".to_string()],
        false,
        &variants,
        0,
    );

    assert_eq!(classification.severity, "likely_expected");
    assert_eq!(classification.evidence_strength, "corroborated_proxy");
    assert_eq!(
        classification.classification_reason,
        "shared_backing_downstream_variation"
    );
}

#[test]
fn axis_values_collect_proxy_predicates_and_db_entities_deterministically() {
    let variant = ImplementationVariant {
        route: vec![
            route_segment(
                RouteSegmentKind::Service,
                "src/validators/origin_validator.rs",
                Some("validate_origin"),
                "validator",
                "symbol_neighbor",
            ),
            route_segment(
                RouteSegmentKind::Query,
                "src/queries/origin_query.sql",
                Some("resolve_origin"),
                "query",
                "query_anchor",
            ),
        ],
        constraints: vec![
            weak_runtime_guard("assert !key.is_empty()"),
            strong_constraint("create unique index uq_origin on origins(origin_key)"),
        ],
        ..variant_with("origin_service", Vec::new(), vec!["test_origin"])
    };

    assert_eq!(
        axis_values(&variant, "predicate_signatures"),
        vec!["assert !key.is_empty()".to_string()]
    );
    assert_eq!(
        axis_values(&variant, "db_entities_and_queries"),
        vec!["query|query_anchor|src/queries/origin_query.sql".to_string()]
    );
    assert_eq!(
        axis_values(&variant, "downstream_symbols"),
        vec!["resolve_origin|query_anchor|src/queries/origin_query.sql".to_string()]
    );
}

#[test]
fn summary_and_followups_surface_highest_risk_and_next_steps() {
    let variants = vec![
        variant_with(
            "left",
            vec![strong_constraint("uq_left")],
            Vec::<&str>::new(),
        ),
        variant_with(
            "right",
            vec![strong_constraint("uq_right")],
            vec!["test_right"],
        ),
    ];
    let divergence_axes = vec![
        DivergenceAxis {
            axis: "constraint_evidence".to_string(),
            values: vec![],
        },
        DivergenceAxis {
            axis: "db_entities_and_queries".to_string(),
            values: vec![],
        },
    ];
    let signals = build_divergence_signals(&variants, &divergence_axes);
    let overall = overall_severity(&signals);
    let summary = build_summary(&variants, &signals, &overall, false, false);
    let followups = recommended_followups(
        &divergence_axes,
        &[],
        true,
        &["no_test_evidence".to_string()],
        &["constraint_evidence".to_string()],
        &signals,
    );

    assert!(summary.contains("highest severity high_risk"));
    assert!(summary.contains("constraint_evidence"));
    assert!(
        followups
            .iter()
            .any(|item| item.contains("schema, migration, and model backing"))
    );
    assert!(
        followups
            .iter()
            .any(|item| item.contains("add tests covering each database-facing variant"))
    );
}

#[test]
fn proxy_only_signals_require_manual_review_and_cautious_followup() {
    let variants = vec![
        variant_with(
            "left",
            vec![strong_constraint("uq_origin")],
            vec!["test_left"],
        ),
        variant_with(
            "right",
            vec![strong_constraint("uq_origin")],
            vec!["test_right"],
        ),
    ];
    let divergence_axes = vec![DivergenceAxis {
        axis: "guards_and_validators".to_string(),
        values: vec![],
    }];
    let signals = build_divergence_signals(&variants, &divergence_axes);
    let overall = overall_severity(&signals);
    let summary = build_summary(&variants, &signals, &overall, true, true);
    let followups = recommended_followups(&divergence_axes, &[], false, &[], &[], &signals);

    assert!(manual_review_required(&signals));
    assert!(
        signals
            .iter()
            .all(|signal| signal.evidence_strength == "proxy_only")
    );
    assert!(summary.contains("do not treat as a bug without hard evidence"));
    assert!(
        followups
            .iter()
            .any(|item| item.contains("Do not treat this divergence as a bug"))
    );
}

#[test]
fn corroborated_expected_signal_suppresses_manual_review_requirement() {
    let variants = vec![
        variant_with(
            "left",
            vec![strong_constraint("uq_origin")],
            vec!["test_left"],
        ),
        variant_with(
            "right",
            vec![strong_constraint("uq_origin")],
            vec!["test_right"],
        ),
    ];
    let divergence_axes = vec![
        DivergenceAxis {
            axis: "guards_and_validators".to_string(),
            values: vec![],
        },
        DivergenceAxis {
            axis: "downstream_symbols".to_string(),
            values: vec![],
        },
    ];
    let signals = build_divergence_signals(&variants, &divergence_axes);
    let overall = overall_severity(&signals);
    let summary = build_summary(&variants, &signals, &overall, false, false);

    assert!(!manual_review_required(&signals));
    assert!(summary.contains("no material risk axes identified"));
    assert!(!summary.contains("manual review required"));
}

fn variant_with(
    id: &str,
    constraints: Vec<ConstraintEvidence>,
    related_tests: Vec<&str>,
) -> ImplementationVariant {
    ImplementationVariant {
        id: id.to_string(),
        entry_anchor: InvestigationAnchor {
            path: format!("src/{id}.rs"),
            language: "rust".to_string(),
            symbol: Some(id.to_string()),
            kind: Some("function".to_string()),
            line: None,
            column: None,
        },
        body_anchor: None,
        route: vec![RouteSegment {
            kind: RouteSegmentKind::Service,
            path: format!("src/{id}.rs"),
            language: "rust".to_string(),
            evidence: "fixture".to_string(),
            anchor_symbol: Some(id.to_string()),
            source_span: None,
            relation_kind: "declares".to_string(),
            source_kind: "fixture".to_string(),
            score: 1.0,
        }],
        constraints,
        related_tests: related_tests.into_iter().map(ToString::to_string).collect(),
        lexical_proximity: 1.0,
        semantic_proximity: 0.0,
        route_centrality: 1.0,
        symbol_overlap: 1.0,
        constraint_overlap: 1.0,
        test_adjacency: 1.0,
        semantic_state: SemanticState::NotApplicable,
        score_model: "heuristic_v2".to_string(),
        score_breakdown: VariantScoreBreakdown {
            lexical: 1.0,
            semantic: 0.0,
            route: 1.0,
            symbol: 1.0,
            constraint: 1.0,
            test: 1.0,
            penalties: 0.0,
            final_score: 1.0,
        },
        confidence: 1.0,
        gaps: Vec::new(),
    }
}

fn route_segment(
    kind: RouteSegmentKind,
    path: &str,
    anchor_symbol: Option<&str>,
    source_kind: &str,
    relation_kind: &str,
) -> RouteSegment {
    RouteSegment {
        kind,
        path: path.to_string(),
        language: "rust".to_string(),
        evidence: "fixture".to_string(),
        anchor_symbol: anchor_symbol.map(ToString::to_string),
        source_span: None,
        relation_kind: relation_kind.to_string(),
        source_kind: source_kind.to_string(),
        score: 1.0,
    }
}

fn strong_constraint(normalized_text: &str) -> ConstraintEvidence {
    ConstraintEvidence::new(
        "index_constraint",
        "index_declaration",
        "migrations/001.sql".to_string(),
        1,
        1,
        normalized_text.to_string(),
        "strong",
        "table".to_string(),
        None,
        1.0,
        normalized_text.to_string(),
    )
}

fn weak_runtime_guard(normalized_text: &str) -> ConstraintEvidence {
    ConstraintEvidence::new(
        "runtime_guard",
        "runtime_guard_code",
        "src/validators/origin_validator.rs".to_string(),
        1,
        1,
        normalized_text.to_string(),
        "weak",
        "service".to_string(),
        None,
        0.6,
        normalized_text.to_string(),
    )
}
