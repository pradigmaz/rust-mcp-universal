use crate::model::RetrievalStage;
use crate::vector_rank::SemanticRerankOutcome;

use super::RetrievalStageCounts;
use super::helpers::semantic_stage_name;

pub(super) fn build_retrieval_pipeline(
    shortlist_len: usize,
    chunk_candidates: usize,
    context_len: usize,
    semantic_requested: bool,
    semantic_outcome: SemanticRerankOutcome,
    stage_counts: Option<RetrievalStageCounts>,
) -> Vec<RetrievalStage> {
    let counts = stage_counts.unwrap_or(RetrievalStageCounts {
        lexical_candidates: shortlist_len,
        semantic_file_candidates: 0,
        semantic_chunk_candidates: 0,
        semantic_candidates: 0,
        fused_candidates: shortlist_len,
        graph_candidates: 0,
        shortlist_candidates: shortlist_len,
    });

    let mut pipeline = vec![
        RetrievalStage {
            stage: "lexical_fts_or_like".to_string(),
            candidates: counts.lexical_candidates,
            kept: counts.lexical_candidates,
        },
        RetrievalStage {
            stage: "graph_signal_boost(symbols/refs/deps)".to_string(),
            candidates: counts.lexical_candidates,
            kept: counts.lexical_candidates,
        },
    ];

    if semantic_requested {
        pipeline.push(RetrievalStage {
            stage: "semantic_file_pool(local_dense_index)".to_string(),
            candidates: counts.semantic_file_candidates,
            kept: counts.semantic_file_candidates,
        });
        pipeline.push(RetrievalStage {
            stage: "semantic_chunk_pool(file_chunks)".to_string(),
            candidates: counts.semantic_chunk_candidates,
            kept: counts.semantic_chunk_candidates,
        });
        pipeline.push(RetrievalStage {
            stage: "semantic_candidate_pool(local_dense_index)".to_string(),
            candidates: counts.semantic_candidates,
            kept: counts.semantic_candidates,
        });
        pipeline.push(RetrievalStage {
            stage: "candidate_fusion(lexical+semantic_union)".to_string(),
            candidates: counts
                .lexical_candidates
                .saturating_add(counts.semantic_candidates),
            kept: counts.fused_candidates,
        });
    }

    if let Some(stage) = semantic_stage_name(semantic_requested, semantic_outcome) {
        pipeline.push(RetrievalStage {
            stage,
            candidates: counts.fused_candidates,
            kept: counts.fused_candidates,
        });
    }

    pipeline.push(RetrievalStage {
        stage: "graph_neighbor_pool(file_graph_edges)".to_string(),
        candidates: counts.fused_candidates,
        kept: counts.graph_candidates,
    });

    pipeline.push(RetrievalStage {
        stage: "candidate_refusion(lexical+semantic+graph)".to_string(),
        candidates: counts
            .fused_candidates
            .saturating_add(counts.graph_candidates),
        kept: counts.shortlist_candidates,
    });

    if chunk_candidates > 0 {
        pipeline.push(RetrievalStage {
            stage: "semantic_chunk_candidate_pool(file_chunks)".to_string(),
            candidates: counts.shortlist_candidates,
            kept: chunk_candidates,
        });
    }

    pipeline.push(RetrievalStage {
        stage: "budget_pack(prioritize_chunk_sources)".to_string(),
        candidates: chunk_candidates.max(counts.shortlist_candidates),
        kept: context_len,
    });
    pipeline
}
