use std::collections::HashMap;

use anyhow::Result;

#[path = "query/agent.rs"]
mod agent;
#[path = "query/brief.rs"]
mod brief;
#[path = "query/chunking.rs"]
mod chunking;
#[path = "query/fusion.rs"]
mod fusion;
#[path = "query/graph_stage.rs"]
mod graph_stage;
#[path = "query/intent.rs"]
mod intent;
#[path = "query/pipeline.rs"]
mod pipeline;
#[path = "query/semantic_candidates.rs"]
mod semantic_candidates;
#[path = "query/support.rs"]
mod support;
#[path = "query/vector_utils.rs"]
mod vector_utils;

use super::{Engine, context};
use crate::model::{
    ContextMode, ContextPackResult, ContextSelection, IndexTelemetry, QueryOptions, SearchHit,
};
use crate::report::{
    QueryReportBuildInput, ResultExplainEntry, RetrievalStageCounts, build_query_report,
};
use crate::vector_rank::SemanticRerankOutcome;
use chunking::best_chunks_for_hits;

#[derive(Debug)]
pub(super) struct SearchExecution {
    pub(super) hits: Vec<SearchHit>,
    pub(super) chunk_by_path: HashMap<String, context::ChunkExcerpt>,
    pub(super) semantic_outcome: SemanticRerankOutcome,
    pub(super) explain_entries: Vec<ResultExplainEntry>,
    pub(super) stage_counts: RetrievalStageCounts,
}

impl Engine {
    pub fn search(&self, options: &QueryOptions) -> Result<Vec<SearchHit>> {
        let execution = self.search_with_meta(options)?;
        Ok(execution.hits)
    }

    pub fn build_context_under_budget(
        &self,
        options: &QueryOptions,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<ContextSelection> {
        let execution = self.search_with_meta(options)?;
        self.context_for_hits_with_chunks(
            &options.query,
            &execution.hits,
            Some(&execution.chunk_by_path),
            options.context_mode,
            max_chars,
            max_tokens,
        )
    }

    pub fn build_context_pack(
        &self,
        options: &QueryOptions,
        mode: ContextMode,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<ContextPackResult> {
        let mut options = options.clone();
        options.context_mode = Some(mode);
        let context = self.build_context_under_budget(&options, max_chars, max_tokens)?;
        Ok(ContextPackResult { mode, context })
    }

    pub fn build_report(
        &self,
        options: &QueryOptions,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<crate::model::QueryReport> {
        let execution = self.search_with_meta(options)?;
        let context = self.context_for_hits_with_chunks(
            &options.query,
            &execution.hits,
            Some(&execution.chunk_by_path),
            options.context_mode,
            max_chars,
            max_tokens,
        )?;
        let (chunk_coverage, chunk_source) = derive_chunk_telemetry(&context);
        let status = self.index_status()?;
        build_query_report(
            &self.project_root,
            QueryReportBuildInput {
                shortlist: &execution.hits,
                context: &context,
                max_tokens,
                privacy_mode: options.privacy_mode,
                semantic_requested: options.semantic,
                semantic_outcome: execution.semantic_outcome,
                explain_entries: &execution.explain_entries,
                stage_counts: Some(execution.stage_counts),
                index_telemetry: IndexTelemetry {
                    last_index_lock_wait_ms: status.last_index_lock_wait_ms,
                    last_embedding_cache_hits: status.last_embedding_cache_hits,
                    last_embedding_cache_misses: status.last_embedding_cache_misses,
                    chunk_coverage,
                    chunk_source,
                },
            },
        )
    }

    pub(super) fn context_for_hits_with_chunks(
        &self,
        query: &str,
        hits: &[SearchHit],
        prefetched_chunks: Option<&HashMap<String, context::ChunkExcerpt>>,
        context_mode: Option<ContextMode>,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<ContextSelection> {
        let chunk_map = if let Some(prefetched) = prefetched_chunks {
            let mut filtered = HashMap::with_capacity(hits.len());
            for hit in hits {
                if let Some(chunk) = prefetched.get(&hit.path) {
                    filtered.insert(hit.path.clone(), chunk.clone());
                }
            }
            filtered
        } else {
            let conn = self.open_db()?;
            best_chunks_for_hits(&conn, query, hits)?
        };
        Ok(context::context_from_hits(
            hits,
            &chunk_map,
            context_mode,
            max_chars,
            max_tokens,
        ))
    }

    pub(super) fn search_with_meta(&self, options: &QueryOptions) -> Result<SearchExecution> {
        pipeline::search_with_meta(self, options)
    }
}

fn derive_chunk_telemetry(context: &ContextSelection) -> (f32, String) {
    if context.files.is_empty() {
        return (0.0, "none".to_string());
    }

    let chunk_coverage =
        (context.chunk_selected as f32 / context.files.len() as f32).clamp(0.0, 1.0);
    if context.chunk_selected == 0 {
        return (chunk_coverage, "none".to_string());
    }

    let mut by_source = HashMap::new();
    for item in &context.files {
        if item.chunk_source == "preview_fallback" {
            continue;
        }
        *by_source
            .entry(item.chunk_source.clone())
            .or_insert(0_usize) += 1;
    }

    let chunk_source = if by_source.is_empty() {
        "none".to_string()
    } else if by_source.len() == 1 {
        by_source
            .into_iter()
            .next()
            .map(|(source, _)| source)
            .unwrap_or_else(|| "none".to_string())
    } else {
        "mixed".to_string()
    };

    (chunk_coverage, chunk_source)
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
