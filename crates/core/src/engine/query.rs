use std::collections::HashMap;

use anyhow::Result;

#[path = "query/agent.rs"]
mod agent;
#[path = "query/agent_bootstrap.rs"]
mod agent_bootstrap;
#[path = "query/agent_compat.rs"]
mod agent_compat;
#[path = "query/agent_query.rs"]
mod agent_query;
#[path = "query/agent_surface.rs"]
mod agent_surface;
#[cfg(test)]
#[path = "query/agent_tests.rs"]
mod agent_tests;
#[path = "query/brief.rs"]
mod brief;
#[path = "query/chunking.rs"]
mod chunking;
#[path = "query/context_selection.rs"]
mod context_selection;
#[path = "query/fusion.rs"]
mod fusion;
#[path = "query/graph_stage.rs"]
mod graph_stage;
#[path = "query/intent.rs"]
mod intent;
#[path = "query/investigation_embed.rs"]
mod investigation_embed;
#[path = "query/pipeline.rs"]
mod pipeline;
#[path = "query/semantic_candidates.rs"]
mod semantic_candidates;
#[path = "query/support.rs"]
mod support;
#[path = "query/support_fusion.rs"]
mod support_fusion;
#[path = "query/surfaces.rs"]
mod surfaces;
#[path = "query/vector_utils.rs"]
mod vector_utils;

use super::{Engine, context};
use crate::model::{AgentIntentMode, ModeResolutionSource, QueryOptions, SearchHit};
use crate::report::{ResultExplainEntry, RetrievalStageCounts};
use crate::vector_rank::SemanticRerankOutcome;

#[derive(Debug)]
pub(super) struct SearchExecution {
    pub(super) hits: Vec<SearchHit>,
    pub(super) chunk_by_path: HashMap<String, context::ChunkExcerpt>,
    pub(super) semantic_outcome: SemanticRerankOutcome,
    pub(super) resolved_mode: AgentIntentMode,
    pub(super) mode_source: ModeResolutionSource,
    pub(super) explain_entries: Vec<ResultExplainEntry>,
    pub(super) stage_counts: RetrievalStageCounts,
}

impl Engine {
    pub fn search(&self, options: &QueryOptions) -> Result<Vec<SearchHit>> {
        let execution = self.search_with_meta(options)?;
        Ok(execution.hits)
    }

    pub(crate) fn search_with_semantic_outcome(
        &self,
        options: &QueryOptions,
    ) -> Result<(Vec<SearchHit>, SemanticRerankOutcome)> {
        let execution = self.search_with_meta(options)?;
        Ok((execution.hits, execution.semantic_outcome))
    }

    pub(super) fn search_with_meta(&self, options: &QueryOptions) -> Result<SearchExecution> {
        pipeline::search_with_meta(self, options)
    }
}

#[cfg(test)]
mod tests {
    use super::semantic_candidates::{ann_accept_floor, ann_probe_limit};
    use super::support::db_limit_for;
    use super::vector_utils::trim_excerpt;

    #[test]
    fn db_limit_for_rejects_oversized_values() {
        if usize::BITS < 64 {
            return;
        }

        let err = db_limit_for(usize::MAX).expect_err("must reject oversized limit");
        assert!(err.to_string().contains("exceeds maximum supported value"));
    }

    #[test]
    fn db_limit_for_accepts_regular_values() {
        let limit = db_limit_for(20).expect("regular limit should be supported");
        assert_eq!(limit, 20);
    }

    #[test]
    fn trim_excerpt_normalizes_whitespace() {
        let trimmed = trim_excerpt("a\tb\nc", 20);
        assert_eq!(trimmed, "a b c");
    }

    #[test]
    fn ann_probe_limit_is_clamped() {
        assert_eq!(ann_probe_limit(1), 64);
        assert_eq!(ann_probe_limit(10), 120);
        assert_eq!(ann_probe_limit(10_000), 1_024);
    }

    #[test]
    fn ann_accept_floor_is_clamped() {
        assert_eq!(ann_accept_floor(1), 6);
        assert_eq!(ann_accept_floor(12), 12);
        assert_eq!(ann_accept_floor(10_000), 24);
    }
}
