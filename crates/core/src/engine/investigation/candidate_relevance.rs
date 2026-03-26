use std::collections::HashSet;

use super::common::CandidateFile;

pub(super) fn retain_query_relevant_candidates(
    seed: &str,
    candidates: Vec<CandidateFile>,
    limit: usize,
) -> Vec<CandidateFile> {
    let mut scored = candidates
        .into_iter()
        .map(|candidate| {
            let overlap = candidate_query_overlap(seed, &candidate);
            let keep_without_overlap = match candidate.source_kind.as_str() {
                "search_candidate" => candidate.score >= 0.08,
                "symbol_lookup" => true,
                "semantic_search_candidate" => candidate.score >= 0.35,
                _ => false,
            };
            (candidate, overlap, keep_without_overlap)
        })
        .collect::<Vec<_>>();

    let has_positive_overlap = scored.iter().any(|(_, overlap, _)| *overlap > 0.0);
    if has_positive_overlap {
        scored.retain(|(candidate, overlap, keep_without_overlap)| {
            *overlap >= minimum_required_overlap(candidate) || *keep_without_overlap
        });
    }

    scored.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| right.0.score.total_cmp(&left.0.score))
            .then_with(|| source_rank(&left.0).cmp(&source_rank(&right.0)))
            .then_with(|| left.0.path.cmp(&right.0.path))
    });

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for (candidate, _, _) in scored {
        if seen.insert((
            candidate.path.clone(),
            candidate.symbol.clone(),
            candidate.line.unwrap_or(0),
        )) {
            out.push(candidate);
        }
        if out.len() >= limit.max(1) {
            break;
        }
    }
    out
}

pub(super) fn candidate_query_overlap(seed: &str, candidate: &CandidateFile) -> f32 {
    let haystack = format!(
        "{} {} {}",
        candidate.path,
        candidate.symbol.as_deref().unwrap_or_default(),
        candidate.source_kind
    );
    query_text_overlap(seed, &haystack)
}

pub(super) fn query_text_overlap(seed: &str, text: &str) -> f32 {
    token_overlap(seed, text)
}

fn minimum_required_overlap(candidate: &CandidateFile) -> f32 {
    match candidate.source_kind.as_str() {
        "route_trace_anchor"
        | "related_file_expansion"
        | "test_expansion"
        | "constraint_evidence_candidate" => 0.15,
        _ => 0.0,
    }
}

fn source_rank(candidate: &CandidateFile) -> usize {
    match candidate.source_kind.as_str() {
        "symbol_lookup" => 0,
        "search_candidate" => 1,
        "semantic_search_candidate" => 2,
        "route_trace_anchor" => 3,
        "related_file_expansion" => 4,
        "constraint_evidence_candidate" => 5,
        "test_expansion" => 6,
        _ => 7,
    }
}

fn token_overlap(left: &str, right: &str) -> f32 {
    let left_tokens = tokenize(left);
    let right_tokens = tokenize(right);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return 0.0;
    }
    let overlap = left_tokens
        .iter()
        .filter(|token| right_tokens.contains(*token))
        .count();
    (overlap as f32 / left_tokens.len().max(right_tokens.len()) as f32).clamp(0.0, 1.0)
}

fn tokenize(value: &str) -> HashSet<String> {
    value
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::investigation::common::CandidateMatchKind;

    fn candidate(path: &str, source_kind: &str, score: f32) -> CandidateFile {
        CandidateFile {
            path: path.to_string(),
            language: "python".to_string(),
            line: None,
            column: None,
            symbol: None,
            symbol_kind: None,
            source_kind: source_kind.to_string(),
            match_kind: CandidateMatchKind::QuerySearch,
            score,
        }
    }

    #[test]
    fn query_relevance_prefers_overlap_and_drops_zero_overlap_expansion_noise() {
        let retained = retain_query_relevant_candidates(
            "origin deadline validator",
            vec![
                candidate(
                    "backend/app/services/attestation/deadline_validator.py",
                    "search_candidate",
                    0.18,
                ),
                candidate(
                    "frontend/src/components/dashboard/types.ts",
                    "route_trace_anchor",
                    0.9,
                ),
                candidate(
                    "frontend/src/lib/api/types/student.ts",
                    "related_file_expansion",
                    0.8,
                ),
            ],
            10,
        );

        assert_eq!(retained.len(), 1);
        assert_eq!(
            retained[0].path,
            "backend/app/services/attestation/deadline_validator.py"
        );
    }

    #[test]
    fn query_relevance_requires_stronger_overlap_for_expansion_candidates() {
        let retained = retain_query_relevant_candidates(
            "origin lesson attestation deadline validator",
            vec![
                candidate(
                    "backend/app/services/attestation/deadline_validator.py",
                    "search_candidate",
                    0.18,
                ),
                candidate(
                    "backend/app/services/attestation/service.py",
                    "route_trace_anchor",
                    0.9,
                ),
            ],
            10,
        );

        assert_eq!(retained.len(), 1);
        assert_eq!(
            retained[0].path,
            "backend/app/services/attestation/deadline_validator.py"
        );
    }
}
