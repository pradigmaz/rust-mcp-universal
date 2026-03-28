use std::collections::{BTreeMap, BTreeSet};

use crate::quality::duplication::artifact::{
    DuplicationArtifact, DuplicationCloneClass, DuplicationCloneMember,
    SuppressedDuplicationCloneClass,
};
use crate::quality::policy::QualityPolicy;

use super::DuplicationAnalysis;
use super::candidates::{DuplicationCandidate, collect_occurrences, load_candidates};
use super::classify::QualityCorpusClass;
use super::collapse::collapse_nested_clone_classes;
use super::facts::build_file_facts;
use super::matching::approximate_match_from_anchor;
use super::semantics::classify_signal_role;
use super::support::{
    LoadedCandidate, SegmentMember, build_member_span, class_id_for_signature, extend_match,
    member_key, ranges_overlap,
};

const MIN_DUPLICATION_TOKENS: usize = 32;
const APPROXIMATE_ANCHOR_TOKENS: usize = 8;
const MAX_HASH_OCCURRENCES: usize = 12;

#[derive(Debug, Clone)]
struct PendingCloneClass {
    language: String,
    corpus_class: QualityCorpusClass,
    normalized_token_count: usize,
    similarity_percent: i64,
    signature_tokens: Vec<String>,
    members: BTreeMap<String, SegmentMember>,
}

pub(crate) fn analyze(
    policy: &QualityPolicy,
    ruleset_id: &str,
    policy_digest: &str,
    candidates: &[DuplicationCandidate<'_>],
) -> DuplicationAnalysis {
    let files = load_candidates(candidates);
    let clone_classes = collapse_nested_clone_classes(build_clone_classes(&files));
    let (clone_classes, suppressed_clone_classes) =
        split_suppressed_clone_classes(policy, clone_classes);
    let file_facts = build_file_facts(&files, &clone_classes);
    DuplicationAnalysis {
        file_facts,
        artifact: DuplicationArtifact {
            version: 4,
            ruleset_id: ruleset_id.to_string(),
            policy_digest: policy_digest.to_string(),
            generated_at_utc: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| String::new()),
            clone_classes,
            suppressed_clone_classes,
        },
    }
}

fn build_clone_classes(files: &[LoadedCandidate]) -> Vec<DuplicationCloneClass> {
    let mut classes = BTreeMap::<String, PendingCloneClass>::new();
    let mut pair_seen = BTreeSet::<String>::new();
    for ((language, corpus_class, _), group) in collect_occurrences(files, MIN_DUPLICATION_TOKENS) {
        if group.len() < 2 || group.len() > MAX_HASH_OCCURRENCES {
            continue;
        }
        for left_idx in 0..group.len() {
            for right_idx in left_idx + 1..group.len() {
                let (left_file_idx, left_start) = group[left_idx];
                let (right_file_idx, right_start) = group[right_idx];
                let left_tokens = &files[left_file_idx].tokens;
                let right_tokens = &files[right_file_idx].tokens;
                let (left_start_idx, right_start_idx, exact_len) =
                    extend_match(left_tokens, left_start, right_tokens, right_start);
                if exact_len < MIN_DUPLICATION_TOKENS {
                    continue;
                }
                insert_clone_class(
                    files,
                    &mut classes,
                    &mut pair_seen,
                    &language,
                    corpus_class,
                    left_file_idx,
                    right_file_idx,
                    left_start_idx,
                    right_start_idx,
                    exact_len,
                    exact_len,
                    exact_len,
                    100,
                    left_tokens[left_start_idx..left_start_idx + exact_len]
                        .iter()
                        .map(|token| token.value.clone())
                        .collect(),
                );
            }
        }
    }
    for ((language, corpus_class, _), group) in
        collect_occurrences(files, APPROXIMATE_ANCHOR_TOKENS)
    {
        if group.len() < 2 || group.len() > MAX_HASH_OCCURRENCES {
            continue;
        }
        for left_idx in 0..group.len() {
            for right_idx in left_idx + 1..group.len() {
                let (left_file_idx, left_start) = group[left_idx];
                let (right_file_idx, right_start) = group[right_idx];
                let left_tokens = &files[left_file_idx].tokens;
                let right_tokens = &files[right_file_idx].tokens;
                let Some(near_miss) = approximate_match_from_anchor(
                    left_tokens,
                    left_start,
                    right_tokens,
                    right_start,
                    APPROXIMATE_ANCHOR_TOKENS,
                ) else {
                    continue;
                };
                if near_miss.common_tokens < MIN_DUPLICATION_TOKENS {
                    continue;
                }
                insert_clone_class(
                    files,
                    &mut classes,
                    &mut pair_seen,
                    &language,
                    corpus_class,
                    left_file_idx,
                    right_file_idx,
                    near_miss.left_start,
                    near_miss.right_start,
                    near_miss.left_len,
                    near_miss.right_len,
                    near_miss.common_tokens,
                    near_miss.similarity_percent,
                    near_miss.signature_tokens,
                );
            }
        }
    }

    classes
        .into_iter()
        .filter_map(|(clone_class_id, pending)| {
            (pending.members.len() >= 2).then(|| {
                let member_paths = pending
                    .members
                    .values()
                    .map(|member| member.path.clone())
                    .collect::<Vec<_>>();
                let signal = classify_signal_role(
                    &pending.language,
                    &member_paths,
                    &pending.signature_tokens,
                );
                let distinct_paths = pending
                    .members
                    .values()
                    .map(|member| member.path.as_str())
                    .collect::<BTreeSet<_>>();
                DuplicationCloneClass {
                    clone_class_id,
                    language: pending.language,
                    corpus_class: pending.corpus_class.as_str().to_string(),
                    normalized_token_count: pending.normalized_token_count,
                    similarity_percent: pending.similarity_percent,
                    signal_role: signal.role,
                    signal_reason: signal.reason,
                    same_file: distinct_paths.len() == 1,
                    cross_file: distinct_paths.len() > 1,
                    members: pending
                        .members
                        .into_values()
                        .map(|member| DuplicationCloneMember {
                            path: member.path,
                            start_line: member.start_line,
                            end_line: member.end_line,
                            token_count: member.end_idx.saturating_sub(member.start_idx),
                        })
                        .collect(),
                }
            })
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn insert_clone_class(
    files: &[LoadedCandidate],
    classes: &mut BTreeMap<String, PendingCloneClass>,
    pair_seen: &mut BTreeSet<String>,
    language: &str,
    corpus_class: QualityCorpusClass,
    left_file_idx: usize,
    right_file_idx: usize,
    match_left_start: usize,
    match_right_start: usize,
    left_len: usize,
    right_len: usize,
    normalized_token_count: usize,
    similarity_percent: i64,
    signature_tokens: Vec<String>,
) {
    if left_file_idx == right_file_idx
        && ranges_overlap(
            match_left_start,
            match_left_start + left_len,
            match_right_start,
            match_right_start + right_len,
        )
    {
        return;
    }
    let pair_key = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        files[left_file_idx].path,
        match_left_start,
        left_len,
        files[right_file_idx].path,
        match_right_start,
        right_len,
        language
    );
    if !pair_seen.insert(pair_key) {
        return;
    }

    let class_id = class_id_for_signature(language, corpus_class, &signature_tokens);
    let left_member = build_member_span(
        &files[left_file_idx].path,
        &files[left_file_idx].tokens,
        match_left_start,
        match_left_start + left_len,
    );
    let right_member = build_member_span(
        &files[right_file_idx].path,
        &files[right_file_idx].tokens,
        match_right_start,
        match_right_start + right_len,
    );
    let pending = classes
        .entry(class_id)
        .or_insert_with(|| PendingCloneClass {
            language: language.to_string(),
            corpus_class,
            normalized_token_count,
            similarity_percent,
            signature_tokens: signature_tokens.clone(),
            members: BTreeMap::new(),
        });
    if normalized_token_count > pending.normalized_token_count {
        pending.normalized_token_count = normalized_token_count;
        pending.signature_tokens = signature_tokens;
    }
    pending.similarity_percent = pending.similarity_percent.max(similarity_percent);
    pending
        .members
        .entry(member_key(&left_member))
        .or_insert(left_member);
    pending
        .members
        .entry(member_key(&right_member))
        .or_insert(right_member);
}

fn split_suppressed_clone_classes(
    policy: &QualityPolicy,
    clone_classes: Vec<DuplicationCloneClass>,
) -> (
    Vec<DuplicationCloneClass>,
    Vec<SuppressedDuplicationCloneClass>,
) {
    let mut active = Vec::new();
    let mut suppressed = Vec::new();
    for clone_class in clone_classes {
        let suppressions = policy.duplication_suppressions_for_class(&clone_class);
        if suppressions.is_empty() {
            active.push(clone_class);
        } else {
            suppressed.push(SuppressedDuplicationCloneClass {
                clone_class,
                suppressions,
            });
        }
    }
    (active, suppressed)
}
