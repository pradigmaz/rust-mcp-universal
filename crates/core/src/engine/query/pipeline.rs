use std::collections::HashMap;

use anyhow::Result;

#[path = "pipeline/explain.rs"]
mod explain;
#[path = "pipeline/lexical.rs"]
mod lexical;
#[path = "pipeline/shortlist.rs"]
mod shortlist;

use explain::{build_explain_entries, resolve_semantic_outcome};
use lexical::build_lexical_by_path;
use shortlist::{build_pre_chunk_hits, compute_chunk_seed_limit, sort_hits_desc};

use super::chunking::chunk_pool_for_hits;
use super::fusion::{FusionInputs, fuse_candidate_pools};
use super::graph_stage::graph_candidate_pool;
use super::intent::SearchIntent;
use super::semantic_candidates::semantic_file_candidates;
use super::support::{
    db_limit_for, derive_fusion_profile, is_low_signal_query, seed_fusion_profile,
    semantic_outcome_code,
};
use super::{Engine, SearchExecution};
use crate::model::{QueryOptions, SemanticFailMode};
use crate::report::RetrievalStageCounts;
use crate::search_db::{search_fts, search_like};
use crate::vector_rank::embed_for_index;

pub(super) fn search_with_meta(engine: &Engine, options: &QueryOptions) -> Result<SearchExecution> {
    let conn = engine.open_db()?;
    let search_intent = SearchIntent::from_query(&options.query);
    let requested_limit = options.limit.max(1);
    let lexical_candidate_limit = search_intent.lexical_candidate_limit(requested_limit);
    let lexical_db_limit = db_limit_for(lexical_candidate_limit)?;

    let mut lexical_hits = search_fts(&conn, &options.query, lexical_db_limit)?;
    let mut used_like_fallback = false;
    if lexical_hits.is_empty() {
        lexical_hits = search_like(&conn, &options.query, lexical_db_limit)?;
        used_like_fallback = true;
    }
    sort_hits_desc(&mut lexical_hits);
    let lexical_candidates = lexical_hits.len();

    let lexical_by_path =
        build_lexical_by_path(&conn, &options.query, &lexical_hits, used_like_fallback)?;

    let profile = derive_fusion_profile(&options.query, options.context_mode);
    let low_signal_semantic = options.semantic && is_low_signal_query(&options.query);
    let semantic_enabled = options.semantic && !low_signal_semantic;
    let candidate_limit =
        search_intent.pre_rerank_candidate_limit(requested_limit, semantic_enabled);
    let query_vec = if semantic_enabled {
        Some(embed_for_index(&options.query))
    } else {
        None
    };

    let mut semantic_file_pool = Vec::new();
    let mut semantic_stage_failed = false;
    if semantic_enabled {
        let semantic_file_result = semantic_file_candidates(
            &conn,
            query_vec.as_deref().unwrap_or(&[]),
            candidate_limit,
            profile.probe_factor,
            options.semantic_fail_mode,
        );
        match semantic_file_result {
            Ok(batch) => {
                semantic_stage_failed = batch.corrupted_rows > 0 && batch.candidates.is_empty();
                semantic_file_pool = batch.candidates;
            }
            Err(err) => {
                if options.semantic_fail_mode == SemanticFailMode::FailClosed {
                    return Err(err);
                }
                semantic_stage_failed = true;
            }
        }
    }
    let semantic_file_candidates = semantic_file_pool.len();
    let semantic_file_indexed = semantic_file_pool
        .iter()
        .any(|candidate| !candidate.semantic_fallback);
    let semantic_file_fallback = semantic_file_pool
        .iter()
        .any(|candidate| candidate.semantic_fallback);

    let mut pre_chunk_hits = build_pre_chunk_hits(
        &lexical_hits,
        &semantic_file_pool,
        profile.semantic_file_weight,
    );
    let chunk_seed_limit = compute_chunk_seed_limit(requested_limit, profile.semantic_chunk_weight);
    pre_chunk_hits.truncate(chunk_seed_limit);

    let (semantic_chunk_pool, chunk_by_path) = if semantic_enabled {
        let chunk_result = chunk_pool_for_hits(
            &conn,
            &options.query,
            query_vec.as_deref(),
            &pre_chunk_hits,
            chunk_seed_limit.max(requested_limit),
        );
        match chunk_result {
            Ok(chunk_payload) => chunk_payload,
            Err(err) => {
                if options.semantic_fail_mode == SemanticFailMode::FailClosed {
                    return Err(err);
                }
                semantic_stage_failed = true;
                (Vec::new(), HashMap::new())
            }
        }
    } else {
        (Vec::new(), HashMap::new())
    };
    let semantic_chunk_candidates = semantic_chunk_pool.len();
    let semantic_chunk_indexed = semantic_chunk_pool
        .iter()
        .any(|candidate| candidate.semantic_indexed);
    let semantic_chunk_fallback = semantic_chunk_pool
        .iter()
        .any(|candidate| candidate.semantic_fallback);
    let semantic_candidates = semantic_file_candidates.saturating_add(semantic_chunk_candidates);
    let semantic_indexed = semantic_file_indexed || semantic_chunk_indexed;
    let semantic_fallback = semantic_file_fallback || semantic_chunk_fallback;

    let seed_profile = seed_fusion_profile(profile);
    let mut pre_graph_hits = if semantic_enabled {
        let fused = fuse_candidate_pools(FusionInputs {
            query: &options.query,
            lexical_pool: &lexical_hits,
            file_pool: &semantic_file_pool,
            chunk_pool: &semantic_chunk_pool,
            graph_pool: &[],
            profile: seed_profile,
            context_mode: options.context_mode,
            candidate_limit,
        });
        fused.hits
    } else {
        lexical_hits.clone()
    };
    if !semantic_enabled {
        search_intent.apply_to_hits(&mut pre_graph_hits, options.context_mode);
    }
    sort_hits_desc(&mut pre_graph_hits);
    pre_graph_hits.truncate(candidate_limit);
    let fused_candidates = pre_graph_hits.len();

    let graph_pool = graph_candidate_pool(&conn, &pre_graph_hits)?;
    let graph_candidates = graph_pool.len();

    let mut fused_explain = HashMap::new();
    let mut hits = if semantic_enabled || !graph_pool.is_empty() {
        let fused = fuse_candidate_pools(FusionInputs {
            query: &options.query,
            lexical_pool: &lexical_hits,
            file_pool: &semantic_file_pool,
            chunk_pool: &semantic_chunk_pool,
            graph_pool: &graph_pool,
            profile,
            context_mode: options.context_mode,
            candidate_limit,
        });
        fused_explain = fused.explain_by_path;
        fused.hits
    } else {
        lexical_hits.clone()
    };
    if !semantic_enabled && graph_pool.is_empty() {
        search_intent.apply_to_hits(&mut hits, options.context_mode);
    }

    let lexical_rank_by_path = lexical_hits
        .iter()
        .enumerate()
        .map(|(idx, hit)| (hit.path.clone(), idx + 1))
        .collect::<HashMap<_, _>>();

    let semantic_outcome = resolve_semantic_outcome(
        semantic_stage_failed,
        options.semantic,
        low_signal_semantic,
        semantic_enabled,
        semantic_indexed,
        semantic_fallback,
    );

    sort_hits_desc(&mut hits);
    hits.truncate(requested_limit);
    let shortlist_candidates = hits.len();

    let semantic_outcome_label =
        semantic_outcome_code(options.semantic, semantic_outcome).to_string();
    let explain_entries = build_explain_entries(
        &hits,
        &lexical_by_path,
        &lexical_rank_by_path,
        &fused_explain,
        &semantic_outcome_label,
    );

    Ok(SearchExecution {
        hits,
        chunk_by_path,
        semantic_outcome,
        explain_entries,
        stage_counts: RetrievalStageCounts {
            lexical_candidates,
            semantic_file_candidates,
            semantic_chunk_candidates,
            semantic_candidates,
            fused_candidates,
            graph_candidates,
            shortlist_candidates,
        },
    })
}
