use std::collections::HashSet;

use crate::model::SearchHit;

use super::super::semantic_candidates::SemanticFileCandidate;

pub(super) fn sort_hits_desc(hits: &mut [SearchHit]) {
    hits.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.path.cmp(&b.path))
    });
}

pub(super) fn build_pre_chunk_hits(
    lexical_hits: &[SearchHit],
    semantic_file_pool: &[SemanticFileCandidate],
    semantic_file_weight: f32,
) -> Vec<SearchHit> {
    let mut pre_chunk_hits = lexical_hits.to_vec();
    let mut pre_chunk_paths = pre_chunk_hits
        .iter()
        .map(|hit| hit.path.clone())
        .collect::<HashSet<_>>();
    for candidate in semantic_file_pool {
        if pre_chunk_paths.contains(&candidate.path) {
            continue;
        }
        pre_chunk_paths.insert(candidate.path.clone());
        pre_chunk_hits.push(SearchHit {
            path: candidate.path.clone(),
            preview: candidate.preview.clone(),
            score: candidate.semantic_score.max(0.0) * semantic_file_weight.max(0.1),
            size_bytes: candidate.size_bytes,
            language: candidate.language.clone(),
        });
    }
    sort_hits_desc(&mut pre_chunk_hits);
    pre_chunk_hits
}

pub(super) fn compute_chunk_seed_limit(
    requested_limit: usize,
    semantic_chunk_weight: f32,
) -> usize {
    let chunk_seed_extra = if semantic_chunk_weight >= 0.33 {
        (requested_limit / 4).max(2)
    } else {
        (requested_limit / 5).max(1)
    };
    let upper_bound = requested_limit.max(32);
    requested_limit
        .saturating_add(chunk_seed_extra)
        .min(upper_bound)
}

#[cfg(test)]
mod tests {
    use super::compute_chunk_seed_limit;

    #[test]
    fn chunk_seed_limit_caps_small_requested_to_32() {
        assert_eq!(compute_chunk_seed_limit(30, 0.5), 32);
    }

    #[test]
    fn chunk_seed_limit_never_panics_for_large_requested() {
        assert_eq!(compute_chunk_seed_limit(40, 0.5), 40);
        assert_eq!(compute_chunk_seed_limit(120, 0.1), 120);
    }
}
