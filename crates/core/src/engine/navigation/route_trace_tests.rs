use crate::engine::investigation::common::{classify_route_segment, classify_route_source_kind};
use crate::model::{RouteSegment, RouteSegmentKind};

use super::{collapse_route_segments, route_trace_capability};

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
