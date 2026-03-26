use crate::engine::investigation::common::{classify_route_segment, classify_route_source_kind};
use crate::model::{RouteSegment, RouteSegmentKind};

use super::route_trace_build::collapse_route_segments;
use super::{
    RankedRoute, compare_ranked_routes, prioritize_start_candidates, route_trace_capability,
};
use crate::model::RoutePath;

#[test]
fn classifier_maps_unknown_paths_to_unknown() {
    assert_eq!(
        classify_route_segment("src/domain/origin_model.rs"),
        RouteSegmentKind::Unknown
    );
    assert_eq!(
        classify_route_source_kind("src/domain/origin_model.rs"),
        "model"
    );
}

#[test]
fn classifier_maps_known_route_layers() {
    assert_eq!(
        classify_route_segment("web/ui/use_origin.ts"),
        RouteSegmentKind::Ui
    );
    assert_eq!(
        classify_route_segment("src/client/origin_client.ts"),
        RouteSegmentKind::ApiClient
    );
    assert_eq!(
        classify_route_segment("src/routes/origin.rs"),
        RouteSegmentKind::Endpoint
    );
    assert_eq!(
        classify_route_segment("src/services/origin_service.rs"),
        RouteSegmentKind::Service
    );
    assert_eq!(
        classify_route_segment("src/queries/origin_query.sql"),
        RouteSegmentKind::Query
    );
}

#[test]
fn collapse_route_segments_merges_adjacent_same_kind_hops() {
    let collapsed = collapse_route_segments(vec![
        segment(
            RouteSegmentKind::Service,
            "src/services/origin_service.rs",
            "self",
        ),
        segment(
            RouteSegmentKind::Service,
            "src/validators/origin_validator.rs",
            "ref_exact",
        ),
        segment(
            RouteSegmentKind::Query,
            "src/queries/origin_query.sql",
            "query",
        ),
    ]);

    assert_eq!(collapsed.len(), 2);
    assert_eq!(collapsed[0].kind, RouteSegmentKind::Service);
    assert!(
        collapsed[0]
            .evidence
            .contains("collapsed:src/validators/origin_validator.rs")
    );
    assert_eq!(collapsed[1].kind, RouteSegmentKind::Query);
}

#[test]
fn capability_marks_unresolved_routes_as_partial() {
    assert_eq!(route_trace_capability(true, true, &[]), "partial");
    assert_eq!(route_trace_capability(true, false, &[]), "supported");
    assert_eq!(
        route_trace_capability(false, false, &["python:docs/readme.md".to_string()]),
        "unsupported"
    );
}

#[test]
fn compare_ranked_routes_prefers_higher_weight_paths() {
    let stronger = RankedRoute {
        sequence: vec!["endpoint".to_string(), "crud".to_string()],
        relevance: 0.9,
        route: RoutePath {
            segments: vec![
                segment(RouteSegmentKind::Endpoint, "src/routes/origin.rs", "self"),
                segment(RouteSegmentKind::Crud, "src/crud/origin.py", "ref_exact"),
            ],
            total_hops: 1,
            total_weight: 3.0,
            collapsed_hops: 0,
            confidence: 0.9,
        },
    };
    let weaker = RankedRoute {
        sequence: vec!["endpoint".to_string(), "migration".to_string()],
        relevance: 0.3,
        route: RoutePath {
            segments: vec![
                segment(RouteSegmentKind::Endpoint, "src/routes/origin.rs", "self"),
                segment(
                    RouteSegmentKind::Migration,
                    "migrations/001_origin.sql",
                    "shared_dep",
                ),
            ],
            total_hops: 1,
            total_weight: 0.35,
            collapsed_hops: 0,
            confidence: 0.6,
        },
    };

    assert!(compare_ranked_routes(&stronger, &weaker).is_lt());
}

#[test]
fn query_start_prioritization_prefers_relevant_service_over_lower_value_ui() {
    use crate::engine::investigation::common::{CandidateFile, CandidateMatchKind};
    use crate::model::ConceptSeedKind;

    let ordered = prioritize_start_candidates(
        vec![
            CandidateFile {
                path: "frontend/src/app/admin/students/components/types.ts".to_string(),
                language: "typescript".to_string(),
                line: None,
                column: None,
                symbol: None,
                symbol_kind: None,
                source_kind: "search_candidate".to_string(),
                match_kind: CandidateMatchKind::QuerySearch,
                score: 0.04,
            },
            CandidateFile {
                path: "backend/app/services/attestation/deadline_validator.py".to_string(),
                language: "python".to_string(),
                line: None,
                column: None,
                symbol: None,
                symbol_kind: None,
                source_kind: "search_candidate".to_string(),
                match_kind: CandidateMatchKind::QuerySearch,
                score: 0.18,
            },
        ],
        ConceptSeedKind::Query,
    );

    assert_eq!(
        ordered.first().map(|candidate| candidate.path.as_str()),
        Some("backend/app/services/attestation/deadline_validator.py")
    );
}

fn segment(kind: RouteSegmentKind, path: &str, evidence: &str) -> RouteSegment {
    RouteSegment {
        kind,
        path: path.to_string(),
        language: "rust".to_string(),
        evidence: evidence.to_string(),
        anchor_symbol: None,
        source_span: None,
        relation_kind: "fixture".to_string(),
        source_kind: "fixture".to_string(),
        score: 1.0,
    }
}
