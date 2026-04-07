#[path = "investigation/actionability.rs"]
mod actionability;
#[path = "investigation/body.rs"]
mod body;
#[path = "investigation/candidate_relevance.rs"]
mod candidate_relevance;
#[path = "investigation/cluster.rs"]
mod cluster;
#[path = "investigation/cluster_constraints.rs"]
mod cluster_constraints;
#[path = "investigation/cluster_policy.rs"]
mod cluster_policy;
#[path = "investigation/cluster_scoring.rs"]
mod cluster_scoring;
#[path = "investigation/cluster_selection.rs"]
mod cluster_selection;
#[path = "investigation/cluster_variants.rs"]
mod cluster_variants;
#[path = "investigation/common.rs"]
pub(crate) mod common;
#[path = "investigation/constraint_items.rs"]
mod constraint_items;
#[path = "investigation/constraint_relevance.rs"]
mod constraint_relevance;
#[path = "investigation/constraints.rs"]
mod constraints;
#[path = "investigation/contract_trace.rs"]
mod contract_trace;
#[path = "investigation/divergence.rs"]
mod divergence;
#[path = "investigation/generated_lineage.rs"]
mod generated_lineage;
#[path = "investigation/path_helpers.rs"]
pub(crate) mod path_helpers;
#[path = "investigation/route.rs"]
mod route;
#[path = "investigation/shared_snapshot.rs"]
mod shared_snapshot;

use anyhow::Result;

use super::Engine;
use crate::model::{
    ConceptClusterResult, ConceptSeedKind, ConstraintEvidenceResult, ContractTraceResult,
    DivergenceReport, RouteTraceResult, SymbolBodyResult,
};

pub(crate) use shared_snapshot::{
    SharedInvestigationSnapshot, shared_query_investigation_snapshot,
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

    pub fn contract_trace(
        &self,
        seed: &str,
        seed_kind: ConceptSeedKind,
        limit: usize,
    ) -> Result<ContractTraceResult> {
        contract_trace::contract_trace(self, seed, seed_kind, limit)
    }
}

#[cfg(test)]
#[path = "investigation/constraint_relevance_tests.rs"]
mod constraint_relevance_tests;
#[cfg(test)]
#[path = "investigation/scoring_tests.rs"]
mod scoring_tests;
#[cfg(test)]
#[path = "investigation/symbol_body_tests.rs"]
mod symbol_body_tests;
#[cfg(test)]
#[path = "investigation/tests.rs"]
mod tests;
