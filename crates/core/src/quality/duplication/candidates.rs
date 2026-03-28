use std::collections::BTreeMap;

use super::classify::{QualityCorpusClass, classify_corpus};
use super::support::{LoadedCandidate, window_hash};
use super::tokenize::normalize_tokens;

const MIN_DUPLICATION_TOKENS: usize = 32;

pub(crate) struct DuplicationCandidate<'a> {
    pub(crate) path: &'a str,
    pub(crate) language: &'a str,
    pub(crate) non_empty_lines: Option<i64>,
    pub(crate) source_text: Option<&'a str>,
}

pub(crate) fn load_candidates(candidates: &[DuplicationCandidate<'_>]) -> Vec<LoadedCandidate> {
    let mut files = Vec::new();
    for candidate in candidates {
        let corpus_class = classify_corpus(candidate.path, candidate.language);
        let Some(source_text) = candidate.source_text else {
            continue;
        };
        if !corpus_class.participates_in_duplication() {
            continue;
        }
        if !matches!(
            candidate.language,
            "rust"
                | "python"
                | "java"
                | "javascript"
                | "jsx"
                | "mjs"
                | "cjs"
                | "typescript"
                | "tsx"
        ) {
            continue;
        }
        let tokens = normalize_tokens(candidate.language, source_text);
        if tokens.len() < MIN_DUPLICATION_TOKENS {
            continue;
        }
        files.push(LoadedCandidate {
            path: candidate.path.to_string(),
            language: candidate.language.to_string(),
            corpus_class,
            tokens,
            non_empty_lines: candidate.non_empty_lines,
        });
    }
    files
}

pub(crate) fn collect_occurrences(
    files: &[LoadedCandidate],
    anchor_len: usize,
) -> BTreeMap<(String, QualityCorpusClass, String), Vec<(usize, usize)>> {
    let mut occurrences =
        BTreeMap::<(String, QualityCorpusClass, String), Vec<(usize, usize)>>::new();
    for (file_idx, file) in files.iter().enumerate() {
        for start in 0..=file.tokens.len().saturating_sub(anchor_len) {
            let hash = window_hash(&file.tokens, start, anchor_len);
            occurrences
                .entry((file.language.clone(), file.corpus_class, hash))
                .or_default()
                .push((file_idx, start));
        }
    }
    occurrences
}
