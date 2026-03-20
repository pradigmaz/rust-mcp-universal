use std::{cmp::Ordering, collections::HashMap, collections::HashSet, path::Path};

use crate::model::{QueryQrel, SearchHit};
use crate::utils::normalize_path;

pub(super) fn recall_at_k(hits: &[SearchHit], qrels: &[QueryQrel], k: usize) -> f32 {
    let relevant = qrels
        .iter()
        .filter(|qrel| qrel.relevance > 0.0)
        .filter_map(|qrel| canonical_benchmark_path(&qrel.path))
        .collect::<HashSet<_>>();
    if relevant.is_empty() {
        return 0.0;
    }

    let retrieved_relevant = hits
        .iter()
        .take(k)
        .filter_map(|hit| canonical_benchmark_path(&hit.path))
        .filter(|path| relevant.contains(path))
        .count();
    retrieved_relevant as f32 / relevant.len() as f32
}

pub(super) fn mrr_at_k(hits: &[SearchHit], qrels: &[QueryQrel], k: usize) -> f32 {
    let relevant = qrels
        .iter()
        .filter(|qrel| qrel.relevance > 0.0)
        .filter_map(|qrel| canonical_benchmark_path(&qrel.path))
        .collect::<HashSet<_>>();
    if relevant.is_empty() {
        return 0.0;
    }

    for (idx, hit) in hits.iter().take(k).enumerate() {
        if canonical_benchmark_path(&hit.path)
            .as_ref()
            .is_some_and(|path| relevant.contains(path))
        {
            return 1.0 / (idx + 1) as f32;
        }
    }
    0.0
}

pub(super) fn ndcg_at_k(hits: &[SearchHit], qrels: &[QueryQrel], k: usize) -> f32 {
    let mut rel_by_path = HashMap::new();
    for qrel in qrels.iter().filter(|qrel| qrel.relevance > 0.0) {
        let Some(path) = canonical_benchmark_path(&qrel.path) else {
            continue;
        };
        rel_by_path
            .entry(path)
            .and_modify(|value| *value = f32::max(*value, qrel.relevance))
            .or_insert(qrel.relevance);
    }
    if rel_by_path.is_empty() {
        return 0.0;
    }

    let mut dcg = 0.0_f32;
    for (rank_idx, hit) in hits.iter().take(k).enumerate() {
        let relevance = canonical_benchmark_path(&hit.path)
            .and_then(|path| rel_by_path.remove(&path))
            .unwrap_or(0.0);
        if relevance <= 0.0 {
            continue;
        }
        let gain = (2.0_f32.powf(relevance) - 1.0) / ((rank_idx + 2) as f32).log2();
        dcg += gain;
    }

    let mut ideal = qrels
        .iter()
        .filter(|qrel| qrel.relevance > 0.0)
        .map(|qrel| qrel.relevance)
        .collect::<Vec<_>>();
    ideal.sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));

    let mut idcg = 0.0_f32;
    for (rank_idx, relevance) in ideal.into_iter().take(k).enumerate() {
        idcg += (2.0_f32.powf(relevance) - 1.0) / ((rank_idx + 2) as f32).log2();
    }

    if idcg > 0.0 { dcg / idcg } else { 0.0 }
}

pub(super) fn percentile(values: &[f32], percentile: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    let p = percentile.clamp(0.0, 100.0) / 100.0;
    let idx = ((sorted.len().saturating_sub(1)) as f32 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

pub(super) fn canonical_benchmark_path(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Benchmark datasets can be shared across OSes; treat '\' as path separator.
    let mut portable = trimmed.replace('\\', "/");
    while portable.starts_with("./") {
        portable = portable[2..].to_string();
    }
    let normalized = normalize_path(Path::new(&portable));
    #[cfg(windows)]
    {
        Some(normalized.to_lowercase())
    }
    #[cfg(not(windows))]
    {
        Some(normalized)
    }
}

#[cfg(test)]
mod tests {
    use super::{mrr_at_k, ndcg_at_k, percentile, recall_at_k};
    use crate::model::{QueryQrel, SearchHit};

    fn hit(path: &str) -> SearchHit {
        SearchHit {
            path: path.to_string(),
            preview: String::new(),
            score: 1.0,
            size_bytes: 0,
            language: "rust".to_string(),
        }
    }

    #[test]
    fn metrics_match_expected_binary_relevance() {
        let hits = vec![hit("a.rs"), hit("b.rs"), hit("c.rs")];
        let qrels = vec![
            QueryQrel {
                path: "b.rs".to_string(),
                relevance: 1.0,
            },
            QueryQrel {
                path: "x.rs".to_string(),
                relevance: 1.0,
            },
        ];

        let recall = recall_at_k(&hits, &qrels, 2);
        let mrr = mrr_at_k(&hits, &qrels, 3);
        let ndcg = ndcg_at_k(&hits, &qrels, 3);

        assert!((recall - 0.5).abs() < 1e-6);
        assert!((mrr - 0.5).abs() < 1e-6);
        assert!(ndcg > 0.0);
    }

    #[test]
    fn percentile_rounds_to_nearest_rank_index() {
        let values = vec![1.0, 2.0, 3.0, 100.0];
        assert_eq!(percentile(&values, 50.0), 3.0);
        assert_eq!(percentile(&values, 95.0), 100.0);
    }

    #[test]
    fn metrics_treat_backslashes_and_dot_prefix_as_same_path() {
        let hits = vec![hit("src/lib.rs")];
        let qrels = vec![QueryQrel {
            path: ".\\src\\lib.rs".to_string(),
            relevance: 1.0,
        }];

        assert!((recall_at_k(&hits, &qrels, 1) - 1.0).abs() < 1e-6);
        assert!((mrr_at_k(&hits, &qrels, 1) - 1.0).abs() < 1e-6);
        assert!(ndcg_at_k(&hits, &qrels, 1) > 0.99);
    }
}
