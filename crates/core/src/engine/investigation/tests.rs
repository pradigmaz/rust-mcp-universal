use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::engine::Engine;
use crate::engine::investigation::cluster_policy::canonical_entry_candidate;
use crate::engine::investigation::common::{CandidateFile, CandidateMatchKind};
use crate::model::{
    ConceptSeedKind, RouteSegment, RouteSegmentKind, SymbolBodyAmbiguityStatus,
    SymbolBodyResolutionKind,
};

fn temp_project_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

#[test]
fn symbol_body_extracts_rust_function_body() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-rust-body");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn interesting_symbol() {\n    println!(\"ok\");\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("interesting_symbol", ConceptSeedKind::Symbol, 3)?;
    assert_eq!(result.capability_status, "supported");
    assert_eq!(result.ambiguity_status, SymbolBodyAmbiguityStatus::None);
    assert!(
        result
            .items
            .iter()
            .any(|item| item.signature.contains("interesting_symbol"))
    );
    assert!(
        result
            .items
            .iter()
            .all(|item| { item.resolution_kind == SymbolBodyResolutionKind::ExactSymbolSpan })
    );

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_path_seed_uses_chunk_excerpt_anchor_for_typescript_files() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-typescript-chunk");
    fs::create_dir_all(project_dir.join("web"))?;
    fs::write(
        project_dir.join("web/origin_client.ts"),
        "export function resolveOriginClient(key: string) {\n  return `/api/origin/${key}`;\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("web/origin_client.ts", ConceptSeedKind::Path, 3)?;

    assert_eq!(result.capability_status, "supported");
    assert_eq!(result.ambiguity_status, SymbolBodyAmbiguityStatus::None);
    assert!(result.items.iter().any(|item| {
        item.anchor.path == "web/origin_client.ts"
            && item.resolution_kind == SymbolBodyResolutionKind::ChunkExcerptAnchor
    }));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_path_seed_handles_bracket_paths() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-bracket-path");
    fs::create_dir_all(project_dir.join("web/[code]/components"))?;
    fs::write(
        project_dir.join("web/[code]/components/page.tsx"),
        "export function ReportStudentTable() {\n  return <div>ok</div>;\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("web/[code]/components/page.tsx", ConceptSeedKind::Path, 3)?;

    assert_eq!(result.capability_status, "supported");
    assert!(result.items.iter().any(|item| {
        item.anchor.path == "web/[code]/components/page.tsx"
            && item.body.contains("ReportStudentTable")
    }));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_path_line_seed_uses_nearest_indexed_lines_for_sql() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-sql-nearest");
    fs::create_dir_all(project_dir.join("queries"))?;
    fs::write(
        project_dir.join("queries/origin_query.sql"),
        "-- resolve origin\nSELECT id\nFROM origins\nWHERE origin_key = $1;\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("queries/origin_query.sql:3", ConceptSeedKind::PathLine, 3)?;

    assert_eq!(result.capability_status, "supported");
    assert!(result.items.iter().any(|item| {
        item.anchor.path == "queries/origin_query.sql"
            && item.resolution_kind == SymbolBodyResolutionKind::NearestIndexedLines
    }));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_marks_multiple_exact_matches() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-multiple-exact");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/a.rs"),
        "pub fn duplicate_symbol() {\n    println!(\"a\");\n}\n",
    )?;
    fs::write(
        project_dir.join("src/b.rs"),
        "pub fn duplicate_symbol() {\n    println!(\"b\");\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("duplicate_symbol", ConceptSeedKind::Symbol, 5)?;

    assert_eq!(
        result.ambiguity_status,
        SymbolBodyAmbiguityStatus::MultipleExact
    );
    assert_eq!(result.items.len(), 2);
    assert!(
        result
            .items
            .iter()
            .all(|item| { item.resolution_kind == SymbolBodyResolutionKind::ExactSymbolSpan })
    );

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_marks_partial_only_when_exact_match_is_absent() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-partial-only");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn resolve_origin() {\n    println!(\"ok\");\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("resolve", ConceptSeedKind::Symbol, 5)?;

    assert_eq!(result.capability_status, "supported");
    assert_eq!(
        result.ambiguity_status,
        SymbolBodyAmbiguityStatus::PartialOnly
    );
    assert!(result.items.iter().any(|item| {
        item.anchor.symbol.as_deref() == Some("resolve_origin")
            && item.resolution_kind == SymbolBodyResolutionKind::ExactSymbolSpan
    }));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn symbol_body_reports_unsupported_sources_for_unsupported_languages() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-unsupported");
    fs::create_dir_all(project_dir.join("docs"))?;
    fs::write(
        project_dir.join("docs/guide.md"),
        "# Investigation\n\nThis file is not a supported source.\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.symbol_body("docs/guide.md", ConceptSeedKind::Path, 3)?;

    assert_eq!(result.capability_status, "unsupported");
    assert!(result.items.is_empty());
    assert_eq!(result.ambiguity_status, SymbolBodyAmbiguityStatus::None);
    assert_eq!(result.unsupported_sources, vec!["markdown:docs/guide.md"]);

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn canonical_entry_prefers_backend_segments_over_ui_shells() {
    let candidate = CandidateFile {
        path: "frontend/src/app/admin/journal/components/AttendanceCell.tsx".to_string(),
        language: "typescript".to_string(),
        line: None,
        column: None,
        symbol: None,
        symbol_kind: None,
        source_kind: "search_candidate".to_string(),
        match_kind: CandidateMatchKind::QuerySearch,
        score: 0.82,
    };
    let route = vec![
        RouteSegment {
            kind: RouteSegmentKind::Ui,
            path: "frontend/src/app/admin/journal/components/AttendanceCell.tsx".to_string(),
            language: "typescript".to_string(),
            evidence: "ui render path".to_string(),
            anchor_symbol: Some("AttendanceCell".to_string()),
            source_span: None,
            relation_kind: "contains".to_string(),
            source_kind: "route_trace".to_string(),
            score: 0.7,
        },
        RouteSegment {
            kind: RouteSegmentKind::Endpoint,
            path: "backend/app/api/v1/endpoints/admin_attendance.py".to_string(),
            language: "python".to_string(),
            evidence: "api handler".to_string(),
            anchor_symbol: Some("router".to_string()),
            source_span: None,
            relation_kind: "calls".to_string(),
            source_kind: "route_trace".to_string(),
            score: 0.92,
        },
        RouteSegment {
            kind: RouteSegmentKind::Crud,
            path: "backend/app/crud/attendance/queries.py".to_string(),
            language: "python".to_string(),
            evidence: "query layer".to_string(),
            anchor_symbol: Some("get_attendance".to_string()),
            source_span: None,
            relation_kind: "calls".to_string(),
            source_kind: "route_trace".to_string(),
            score: 0.88,
        },
    ];

    let canonical = canonical_entry_candidate(&candidate, &route);

    assert_eq!(
        canonical.path,
        "backend/app/api/v1/endpoints/admin_attendance.py"
    );
    assert_eq!(canonical.symbol.as_deref(), Some("router"));
}

#[test]
fn concept_cluster_collects_constraints_or_tests_from_python_paths() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-python-cluster");
    fs::create_dir_all(project_dir.join("app/services"))?;
    fs::create_dir_all(project_dir.join("app/models"))?;
    fs::create_dir_all(project_dir.join("tests"))?;
    fs::write(
        project_dir.join("app/services/lab_service.py"),
        "def resolve_lab():\n    return True\n",
    )?;
    fs::write(
        project_dir.join("app/models/lab.py"),
        "UniqueConstraint('subject_id', 'number', name='uq_lab_subject_number')\n",
    )?;
    fs::write(
        project_dir.join("tests/test_lab_service.py"),
        "def test_resolve_lab():\n    assert True\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.concept_cluster("resolve_lab", ConceptSeedKind::Query, 5)?;
    assert!(!result.variants.is_empty());
    assert!(
        result.variants.iter().any(|variant| {
            !variant.constraints.is_empty() || !variant.related_tests.is_empty()
        })
    );
    assert!(
        result
            .cluster_summary
            .expansion_sources
            .iter()
            .any(|source| source == "retrieval_shortlist")
    );
    assert!(
        result
            .cluster_summary
            .expansion_sources
            .iter()
            .any(|source| source == "route_trace_anchors")
    );
    assert_eq!(
        result.cluster_summary.cutoff_policy.as_deref(),
        Some(
            "expand<=limit*3; score+dedup full pool; query seeds promote execution paths within top_4 when score gap<=0.05; return top_5"
        )
    );
    assert_eq!(
        result.cluster_summary.dedup_policy.as_deref(),
        Some(
            "candidate(path,symbol,line); variant(entry_anchor.path)->confidence,constraint,route,lexical,path"
        )
    );
    let expansion_policy = result
        .cluster_summary
        .expansion_policy
        .as_ref()
        .expect("expansion_policy should be serialized");
    assert_eq!(
        expansion_policy.initial_sources,
        vec!["retrieval_shortlist", "symbol_neighbors"]
    );
    assert_eq!(
        expansion_policy.enrichment_sources,
        vec!["semantic_retrieval", "route_trace_anchors", "related_files"]
    );
    assert_eq!(
        expansion_policy.feedback_sources,
        vec!["tests", "constraint_evidence"]
    );
    assert!(expansion_policy.route_trace_reused);
    assert_eq!(expansion_policy.candidate_pool_limit_multiplier, 3);
    assert_eq!(expansion_policy.dedup_unit, "entry_anchor.path");
    assert_eq!(
        expansion_policy.tie_break_order,
        vec![
            "final_confidence",
            "constraint_overlap",
            "route_centrality",
            "lexical_proximity",
            "path"
        ]
    );
    let unique_entry_paths = result
        .variants
        .iter()
        .map(|variant| variant.entry_anchor.path.clone())
        .collect::<HashSet<_>>();
    assert_eq!(unique_entry_paths.len(), result.variants.len());
    assert!(result.variants.iter().all(|variant| {
        (0.0..=1.0).contains(&variant.lexical_proximity)
            && (0.0..=1.0).contains(&variant.semantic_proximity)
            && (0.0..=1.0).contains(&variant.route_centrality)
            && (0.0..=1.0).contains(&variant.symbol_overlap)
            && (0.0..=1.0).contains(&variant.constraint_overlap)
            && (0.0..=1.0).contains(&variant.test_adjacency)
            && !variant.score_model.is_empty()
            && variant.score_breakdown.final_score >= 0.0
    }));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn constraint_evidence_prioritizes_strong_schema_signals() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-constraint-priority");
    fs::create_dir_all(project_dir.join("backend/app/models"))?;
    fs::create_dir_all(project_dir.join("frontend"))?;
    fs::write(
        project_dir.join("backend/app/models/attendance.py"),
        "UniqueConstraint('student_id', 'lesson_id', name='uq_attendance_student_lesson')\n",
    )?;
    fs::write(
        project_dir.join("frontend/attendance.ts"),
        "export function validateAttendance(value: string) {\n  return value.length > 0;\n}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.constraint_evidence("attendance", ConceptSeedKind::Query, 5)?;

    assert!(!result.items.is_empty());
    assert_eq!(result.items[0].strength, "strong");
    assert_eq!(result.items[0].path, "backend/app/models/attendance.py");

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn route_trace_returns_typed_best_route_and_gap_markers() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-route-trace");
    fs::create_dir_all(project_dir.join("app/services"))?;
    fs::create_dir_all(project_dir.join("app/models"))?;
    fs::create_dir_all(project_dir.join("tests"))?;
    fs::write(
        project_dir.join("app/services/lab_service.py"),
        "def resolve_lab():\n    return True\n",
    )?;
    fs::write(
        project_dir.join("app/models/lab.py"),
        "UniqueConstraint('subject_id', 'number', name='uq_lab_subject_number')\n",
    )?;
    fs::write(
        project_dir.join("tests/test_lab_service.py"),
        "def test_resolve_lab():\n    assert True\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let route = engine.route_trace("app/services/lab_service.py", ConceptSeedKind::Path, 5)?;

    assert!(!route.best_route.segments.is_empty());
    assert_eq!(route.best_route.segments[0].kind, RouteSegmentKind::Service);
    assert!(route.alternate_routes.is_empty());
    assert_eq!(route.capability_status, "partial");
    assert!(!route.unresolved_gaps.is_empty());

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn divergence_report_emits_axes_for_multiple_candidates() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-divergence");
    fs::create_dir_all(project_dir.join("src/services"))?;
    fs::write(
        project_dir.join("src/services/origin_service.rs"),
        "pub fn origin_resolution() { helper_query(); }\nfn helper_query() {}\n",
    )?;
    fs::write(
        project_dir.join("src/services/origin_validator.rs"),
        "pub fn origin_resolution_validator() {}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let report = engine.divergence_report("origin_resolution", ConceptSeedKind::Query, 5)?;
    assert!(!report.variants.is_empty());
    assert!(!report.consensus_axes.is_empty() || !report.divergence_axes.is_empty());
    assert!(!report.divergence_signals.is_empty());
    assert!(report.shared_evidence.iter().all(|item| !item.is_empty()));

    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}
