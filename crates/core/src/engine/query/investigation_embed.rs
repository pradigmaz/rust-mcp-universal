use std::collections::HashSet;

use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    ConceptSeedKind, ConstraintEvidenceResult, DivergenceReport, InvestigationConstraintSummary,
    InvestigationDivergenceSummary, InvestigationHints, InvestigationRouteSummary,
    InvestigationSummary, InvestigationTopVariant, RouteTraceResult,
};

const MAX_TOP_VARIANTS: usize = 3;
const MAX_NORMALIZED_KEYS: usize = 5;
const MAX_FOLLOWUPS: usize = 3;
const EMBEDDED_SURFACE_KIND: &str = "embedded_investigation_hints";
const DIVERGENCE_PREVIEW_SURFACE_KIND: &str = "divergence_preview";
const DIVERGENCE_AUTHORITATIVE_TOOL: &str = "divergence_report";

pub(super) fn build_investigation_summary(
    engine: &Engine,
    query: &str,
    limit: usize,
) -> Result<InvestigationSummary> {
    let concept_cluster = engine.concept_cluster(query, ConceptSeedKind::Query, limit)?;
    let route_trace = engine.route_trace(query, ConceptSeedKind::Query, limit)?;
    let constraint_evidence = engine.constraint_evidence(query, ConceptSeedKind::Query, limit)?;
    let divergence = if concept_cluster.variants.len() > 1 {
        Some(engine.divergence_report(query, ConceptSeedKind::Query, limit)?)
    } else {
        None
    };

    Ok(InvestigationSummary {
        surface_kind: EMBEDDED_SURFACE_KIND.to_string(),
        concept_cluster: crate::model::InvestigationConceptClusterSummary {
            variant_count: concept_cluster.cluster_summary.variant_count,
            top_variants: collect_top_variants(&concept_cluster.variants),
        },
        route_trace: summarize_route(&route_trace),
        constraint_evidence: summarize_constraints(&constraint_evidence),
        divergence: divergence.as_ref().map(summarize_divergence),
    })
}

pub(super) fn build_investigation_hints(
    engine: &Engine,
    query: &str,
    limit: usize,
) -> Result<InvestigationHints> {
    let concept_cluster = engine.concept_cluster(query, ConceptSeedKind::Query, limit)?;
    let route_trace = engine.route_trace(query, ConceptSeedKind::Query, limit)?;
    let constraint_evidence = engine.constraint_evidence(query, ConceptSeedKind::Query, limit)?;
    let followups = if concept_cluster.variants.len() > 1 {
        let divergence = engine.divergence_report(query, ConceptSeedKind::Query, limit)?;
        divergence
            .recommended_followups
            .into_iter()
            .take(MAX_FOLLOWUPS)
            .collect()
    } else {
        Vec::new()
    };

    Ok(InvestigationHints {
        top_variants: collect_top_variants(&concept_cluster.variants),
        route_summary: summarize_route(&route_trace),
        constraint_keys: summarize_constraints(&constraint_evidence).normalized_keys,
        followups,
    })
}

fn collect_top_variants(
    variants: &[crate::model::ImplementationVariant],
) -> Vec<InvestigationTopVariant> {
    variants
        .iter()
        .take(MAX_TOP_VARIANTS)
        .map(|variant| InvestigationTopVariant {
            path: variant.entry_anchor.path.clone(),
            symbol: variant.entry_anchor.symbol.clone(),
            confidence: variant.confidence,
        })
        .collect()
}

fn summarize_route(route_trace: &RouteTraceResult) -> InvestigationRouteSummary {
    InvestigationRouteSummary {
        best_route_segment_count: route_trace.best_route.segments.len(),
        alternate_route_count: route_trace.alternate_routes.len(),
        unresolved_gap_count: route_trace.unresolved_gaps.len(),
        segment_kinds: unique_preserve_order(
            route_trace
                .best_route
                .segments
                .iter()
                .map(|segment| route_segment_kind_name(segment.kind).to_string())
                .chain(
                    route_trace
                        .alternate_routes
                        .iter()
                        .flat_map(|route| route.segments.iter())
                        .map(|segment| route_segment_kind_name(segment.kind).to_string()),
                ),
        ),
    }
}

fn summarize_constraints(
    constraint_evidence: &ConstraintEvidenceResult,
) -> InvestigationConstraintSummary {
    let strong = constraint_evidence
        .items
        .iter()
        .filter(|item| item.strength == "strong")
        .count();
    let weak = constraint_evidence
        .items
        .iter()
        .filter(|item| item.strength == "weak")
        .count();

    InvestigationConstraintSummary {
        total: constraint_evidence.items.len(),
        strong,
        weak,
        constraint_kinds: unique_preserve_order(
            constraint_evidence
                .items
                .iter()
                .map(|item| item.constraint_kind.clone()),
        ),
        normalized_keys: unique_preserve_order(
            constraint_evidence
                .items
                .iter()
                .map(|item| item.normalized_key.clone()),
        )
        .into_iter()
        .take(MAX_NORMALIZED_KEYS)
        .collect(),
    }
}

fn summarize_divergence(report: &DivergenceReport) -> InvestigationDivergenceSummary {
    InvestigationDivergenceSummary {
        surface_kind: DIVERGENCE_PREVIEW_SURFACE_KIND.to_string(),
        authoritative_tool: DIVERGENCE_AUTHORITATIVE_TOOL.to_string(),
        preview_only: true,
        highest_severity: report.overall_severity.clone(),
        signal_count: report.divergence_signals.len(),
        recommended_followups: report
            .recommended_followups
            .iter()
            .take(MAX_FOLLOWUPS)
            .cloned()
            .collect(),
    }
}

fn route_segment_kind_name(kind: crate::model::RouteSegmentKind) -> &'static str {
    match kind {
        crate::model::RouteSegmentKind::Ui => "ui",
        crate::model::RouteSegmentKind::ApiClient => "api_client",
        crate::model::RouteSegmentKind::Endpoint => "endpoint",
        crate::model::RouteSegmentKind::Service => "service",
        crate::model::RouteSegmentKind::Crud => "crud",
        crate::model::RouteSegmentKind::Query => "query",
        crate::model::RouteSegmentKind::Test => "test",
        crate::model::RouteSegmentKind::Migration => "migration",
        crate::model::RouteSegmentKind::Unknown => "unknown",
    }
}

fn unique_preserve_order<I>(values: I) -> Vec<String>
where
    I: Iterator<Item = String>,
{
    let mut seen = HashSet::new();
    let mut ordered = Vec::new();
    for value in values {
        if seen.insert(value.clone()) {
            ordered.push(value);
        }
    }
    ordered
}
