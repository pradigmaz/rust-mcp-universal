use std::time::Instant;

use anyhow::Result;

use crate::engine::Engine;
use crate::model::{
    ConceptClusterResult, ConceptSeedKind, ConstraintEvidenceResult, DivergenceReport,
    InvestigationPhaseTimings, RouteTraceResult,
};

use super::cluster::{concept_cluster_with_route_trace, constraint_evidence_from_cluster};
use super::divergence::divergence_report_from_cluster;

#[derive(Debug, Clone)]
pub(crate) struct SharedInvestigationSnapshot {
    pub(crate) concept_cluster: ConceptClusterResult,
    pub(crate) route_trace: RouteTraceResult,
    pub(crate) constraint_evidence: ConstraintEvidenceResult,
    pub(crate) divergence: Option<DivergenceReport>,
    pub(crate) timings: InvestigationPhaseTimings,
}

pub(crate) fn shared_query_investigation_snapshot(
    engine: &Engine,
    query: &str,
    limit: usize,
) -> Result<SharedInvestigationSnapshot> {
    let requested_limit = limit.max(1);
    let mut timings = InvestigationPhaseTimings::default();

    let phase_started = Instant::now();
    let route_trace = engine.route_trace(query, ConceptSeedKind::Query, requested_limit)?;
    timings.route_ms = elapsed_ms(phase_started);

    let phase_started = Instant::now();
    let concept_cluster = concept_cluster_with_route_trace(
        engine,
        query,
        ConceptSeedKind::Query,
        requested_limit,
        Some(&route_trace),
    )?;
    timings.cluster_ms = elapsed_ms(phase_started);

    let phase_started = Instant::now();
    let constraint_evidence = constraint_evidence_from_cluster(concept_cluster.clone());
    timings.constraints_ms = elapsed_ms(phase_started);

    let divergence = if concept_cluster.variants.len() > 1 {
        let phase_started = Instant::now();
        let report = divergence_report_from_cluster(concept_cluster.clone());
        timings.divergence_ms = elapsed_ms(phase_started);
        Some(report)
    } else {
        None
    };

    Ok(SharedInvestigationSnapshot {
        concept_cluster,
        route_trace,
        constraint_evidence,
        divergence,
        timings,
    })
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
