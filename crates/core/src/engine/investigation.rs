#[path = "investigation/body.rs"]
mod body;
#[path = "investigation/cluster.rs"]
mod cluster;
#[path = "investigation/cluster_policy.rs"]
mod cluster_policy;
#[path = "investigation/cluster_scoring.rs"]
mod cluster_scoring;
#[path = "investigation/common.rs"]
pub(crate) mod common;
#[path = "investigation/constraints.rs"]
mod constraints;
#[path = "investigation/divergence.rs"]
mod divergence;
#[path = "investigation/route.rs"]
mod route;

use anyhow::Result;

use super::Engine;
use crate::model::{
    ConceptClusterResult, ConceptSeedKind, ConstraintEvidenceResult, DivergenceReport,
    RouteTraceResult, SymbolBodyResult,
};

impl Engine {
    pub fn symbol_body(
        &self,
        seed: &str,
        seed_kind: ConceptSeedKind,
        limit: usize,
    ) -> Result<SymbolBodyResult> {
        body::symbol_body(self, seed, seed_kind, limit)
    }

    pub fn route_trace(
        &self,
        seed: &str,
        seed_kind: ConceptSeedKind,
        limit: usize,
    ) -> Result<RouteTraceResult> {
        super::navigation::route_trace::route_trace(self, seed, seed_kind, limit)
    }

    pub fn constraint_evidence(
        &self,
        seed: &str,
        seed_kind: ConceptSeedKind,
        limit: usize,
    ) -> Result<ConstraintEvidenceResult> {
        cluster::constraint_evidence(self, seed, seed_kind, limit)
    }

    pub fn concept_cluster(
        &self,
        seed: &str,
        seed_kind: ConceptSeedKind,
        limit: usize,
    ) -> Result<ConceptClusterResult> {
        cluster::concept_cluster(self, seed, seed_kind, limit)
    }

    pub fn divergence_report(
        &self,
        seed: &str,
        seed_kind: ConceptSeedKind,
        limit: usize,
    ) -> Result<DivergenceReport> {
        divergence::divergence_report(self, seed, seed_kind, limit)
    }
}

#[cfg(test)]
#[path = "investigation/scoring_tests.rs"]
mod scoring_tests;
#[cfg(test)]
#[path = "investigation/tests.rs"]
mod tests;
