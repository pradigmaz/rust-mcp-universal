#[path = "fusion/accumulate.rs"]
mod accumulate;
#[path = "fusion/anchors.rs"]
mod anchors;
#[path = "fusion/score.rs"]
mod score;
#[path = "fusion/types.rs"]
mod types;

pub(super) use types::{FusedExplainMeta, FusionInputs, FusionResult};

use super::intent::SearchIntent;
use accumulate::build_candidate_states;
use anchors::{lexical_anchor_paths, retain_lexical_anchors};
use score::score_candidates;

pub(super) fn fuse_candidate_pools(input: FusionInputs<'_>) -> FusionResult {
    let lexical_anchor_paths = lexical_anchor_paths(input.query, input.lexical_pool);
    let search_intent = SearchIntent::from_query(input.query);
    let mut scored = score_candidates(
        build_candidate_states(
            input.lexical_pool,
            input.file_pool,
            input.chunk_pool,
            input.graph_pool,
        ),
        input.profile,
        input.context_mode,
        &search_intent,
        &lexical_anchor_paths,
    );
    scored.sort_by(|a, b| {
        b.0.score
            .total_cmp(&a.0.score)
            .then_with(|| a.0.path.cmp(&b.0.path))
    });
    retain_lexical_anchors(
        &mut scored,
        &lexical_anchor_paths,
        input.candidate_limit.max(1),
    );
    scored.truncate(input.candidate_limit.max(1));

    let mut hits = Vec::with_capacity(scored.len());
    let mut explain_by_path = std::collections::HashMap::with_capacity(scored.len());
    for (idx, (hit, mut meta)) in scored.into_iter().enumerate() {
        meta.rank_after = idx + 1;
        explain_by_path.insert(hit.path.clone(), meta);
        hits.push(hit);
    }

    FusionResult {
        hits,
        explain_by_path,
    }
}
