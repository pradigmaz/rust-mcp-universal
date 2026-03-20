use std::collections::HashMap;

use crate::model::{ContextFile, ContextMode, ContextSelection, SearchHit};
use crate::utils::estimate_tokens_for_text;

const PREVIEW_EXCERPT_MAX_CHARS: usize = 140;

#[derive(Debug, Clone)]
pub(super) struct ChunkExcerpt {
    pub(super) excerpt: String,
    pub(super) chunk_idx: usize,
    pub(super) start_line: usize,
    pub(super) end_line: usize,
    pub(super) score: f32,
    pub(super) source: String,
}

#[derive(Debug)]
struct ContextCandidate {
    path: String,
    excerpt: String,
    score: f32,
    chunk_idx: usize,
    start_line: usize,
    end_line: usize,
    chunk_source: String,
    chunk_score: f32,
    has_chunk: bool,
}

pub(super) fn context_from_hits(
    hits: &[SearchHit],
    chunk_by_path: &HashMap<String, ChunkExcerpt>,
    context_mode: Option<ContextMode>,
    max_chars: usize,
    max_tokens: usize,
) -> ContextSelection {
    let mut files = Vec::new();
    let mut total_chars = 0_usize;
    let mut total_tokens = 0_usize;
    let mut truncated = false;
    let mut chunk_selected = 0_usize;

    let mut candidates = Vec::with_capacity(hits.len());
    for hit in hits {
        if let Some(chunk) = chunk_by_path.get(&hit.path) {
            candidates.push(ContextCandidate {
                path: hit.path.clone(),
                excerpt: chunk.excerpt.clone(),
                score: hit.score,
                chunk_idx: chunk.chunk_idx,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                chunk_source: chunk.source.clone(),
                chunk_score: chunk.score,
                has_chunk: true,
            });
            continue;
        }

        candidates.push(ContextCandidate {
            path: hit.path.clone(),
            excerpt: compact_preview_excerpt(&hit.preview, PREVIEW_EXCERPT_MAX_CHARS),
            score: hit.score,
            chunk_idx: 0,
            start_line: 0,
            end_line: 0,
            chunk_source: "preview_fallback".to_string(),
            chunk_score: 0.0,
            has_chunk: false,
        });
    }

    candidates.sort_by(|a, b| compare_context_candidates(a, b, context_mode));

    for candidate in candidates {
        let next_len = candidate.excerpt.chars().count();
        let next_tokens = estimate_tokens_for_text(&candidate.excerpt);
        let next_chars = total_chars + next_len;
        let candidate_tokens = total_tokens + next_tokens;
        if next_chars > max_chars || candidate_tokens > max_tokens {
            truncated = true;
            continue;
        }

        files.push(ContextFile {
            path: candidate.path,
            excerpt: candidate.excerpt,
            score: candidate.score,
            chunk_idx: candidate.chunk_idx,
            start_line: candidate.start_line,
            end_line: candidate.end_line,
            chunk_source: candidate.chunk_source,
        });
        if candidate.has_chunk {
            chunk_selected += 1;
        }
        total_chars = next_chars;
        total_tokens = candidate_tokens;
    }

    ContextSelection {
        files,
        total_chars,
        estimated_tokens: total_tokens,
        truncated,
        chunk_candidates: chunk_by_path.len(),
        chunk_selected,
    }
}

fn compare_context_candidates(
    left: &ContextCandidate,
    right: &ContextCandidate,
    context_mode: Option<ContextMode>,
) -> std::cmp::Ordering {
    let left_path = left.path.replace('\\', "/");
    let right_path = right.path.replace('\\', "/");
    let left_is_test = is_test_path(&left_path);
    let right_is_test = is_test_path(&right_path);
    let left_is_design = is_design_path(&left_path);
    let right_is_design = is_design_path(&right_path);

    match context_mode.unwrap_or(ContextMode::Code) {
        ContextMode::Code => right
            .has_chunk
            .cmp(&left.has_chunk)
            .then_with(|| left_is_design.cmp(&right_is_design))
            .then_with(|| left_is_test.cmp(&right_is_test))
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| right.chunk_score.total_cmp(&left.chunk_score))
            .then_with(|| left.path.cmp(&right.path)),
        ContextMode::Design => right_is_design
            .cmp(&left_is_design)
            .then_with(|| right.has_chunk.cmp(&left.has_chunk))
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| right.chunk_score.total_cmp(&left.chunk_score))
            .then_with(|| left.path.cmp(&right.path)),
        ContextMode::Bugfix => right
            .has_chunk
            .cmp(&left.has_chunk)
            .then_with(|| right_is_test.cmp(&left_is_test))
            .then_with(|| left_is_design.cmp(&right_is_design))
            .then_with(|| right.score.total_cmp(&left.score))
            .then_with(|| right.chunk_score.total_cmp(&left.chunk_score))
            .then_with(|| left.path.cmp(&right.path)),
    }
}

fn is_test_path(path: &str) -> bool {
    path.starts_with("tests/")
        || path.contains("/tests/")
        || path.contains("/test/")
        || path.contains("_tests/")
        || path.ends_with("_test.rs")
        || path.ends_with("_tests.rs")
}

fn is_design_path(path: &str) -> bool {
    path.ends_with(".md")
        || path.ends_with(".mdx")
        || path.ends_with(".rst")
        || path.ends_with(".txt")
        || path.starts_with("docs/")
        || path.contains("/docs/")
        || path.starts_with("schemas/")
        || path.contains("/schemas/")
        || path == "Cargo.toml"
        || path == "Cargo.lock"
        || path.ends_with(".toml")
        || path.ends_with(".json")
}

fn compact_preview_excerpt(preview: &str, max_chars: usize) -> String {
    let compact = preview.replace(['\n', '\r', '\t'], " ");
    let trimmed = compact.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }

    let mut out = String::with_capacity(max_chars + 3);
    for ch in trimmed.chars().take(max_chars) {
        out.push(ch);
    }
    out.push_str("...");
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::model::{ContextMode, SearchHit};

    use super::{ChunkExcerpt, compact_preview_excerpt, context_from_hits};

    fn hit(path: &str, score: f32, preview: &str) -> SearchHit {
        SearchHit {
            path: path.to_string(),
            preview: preview.to_string(),
            score,
            size_bytes: 0,
            language: "rust".to_string(),
        }
    }

    #[test]
    fn budget_pack_prioritizes_chunk_sources() {
        let hits = vec![
            hit("src/no_chunk.rs", 0.99, "preview without semantic chunk"),
            hit("src/with_chunk.rs", 0.50, "generic preview"),
        ];
        let mut chunk_map = HashMap::new();
        chunk_map.insert(
            "src/with_chunk.rs".to_string(),
            ChunkExcerpt {
                excerpt: "needle semantic chunk".to_string(),
                chunk_idx: 1,
                start_line: 10,
                end_line: 20,
                score: 0.91,
                source: "chunk_embedding_index".to_string(),
            },
        );

        let context = context_from_hits(&hits, &chunk_map, Some(ContextMode::Code), 40, 100);
        assert_eq!(context.files.len(), 1);
        assert_eq!(context.files[0].path, "src/with_chunk.rs");
        assert_eq!(context.files[0].chunk_source, "chunk_embedding_index");
        assert_eq!(context.chunk_candidates, 1);
        assert_eq!(context.chunk_selected, 1);
    }

    #[test]
    fn chunk_telemetry_counts_selected_chunks() {
        let hits = vec![
            hit("src/a.rs", 0.8, "preview a"),
            hit("src/b.rs", 0.7, "preview b"),
        ];
        let mut chunk_map = HashMap::new();
        chunk_map.insert(
            "src/a.rs".to_string(),
            ChunkExcerpt {
                excerpt: "chunk a".to_string(),
                chunk_idx: 0,
                start_line: 1,
                end_line: 3,
                score: 0.88,
                source: "chunk_embedding_index".to_string(),
            },
        );

        let context = context_from_hits(&hits, &chunk_map, Some(ContextMode::Code), 10_000, 10_000);
        assert_eq!(context.files.len(), 2);
        assert_eq!(context.chunk_candidates, 1);
        assert_eq!(context.chunk_selected, 1);
        assert!(
            context
                .files
                .iter()
                .any(|item| item.chunk_source == "chunk_embedding_index")
        );
        assert!(
            context
                .files
                .iter()
                .any(|item| item.chunk_source == "preview_fallback")
        );
    }

    #[test]
    fn budget_pack_preserves_hit_order_within_chunked_candidates() {
        let hits = vec![
            hit("src/high_rank.rs", 0.90, "preview high"),
            hit("src/high_chunk.rs", 0.40, "preview chunk"),
        ];
        let mut chunk_map = HashMap::new();
        chunk_map.insert(
            "src/high_rank.rs".to_string(),
            ChunkExcerpt {
                excerpt: "relevant implementation chunk".to_string(),
                chunk_idx: 0,
                start_line: 1,
                end_line: 5,
                score: 0.41,
                source: "chunk_embedding_index".to_string(),
            },
        );
        chunk_map.insert(
            "src/high_chunk.rs".to_string(),
            ChunkExcerpt {
                excerpt: "strong chunk score but lower final rank".to_string(),
                chunk_idx: 1,
                start_line: 10,
                end_line: 14,
                score: 0.95,
                source: "chunk_embedding_index".to_string(),
            },
        );

        let context = context_from_hits(&hits, &chunk_map, Some(ContextMode::Code), 10_000, 10_000);
        assert_eq!(context.files.len(), 2);
        assert_eq!(context.files[0].path, "src/high_rank.rs");
        assert_eq!(context.files[1].path, "src/high_chunk.rs");
    }

    #[test]
    fn preview_excerpt_is_compacted_for_budgeting() {
        let preview = "x".repeat(500);
        let compact = compact_preview_excerpt(&preview, 40);
        assert!(compact.len() <= 43);
        assert!(compact.ends_with("..."));
    }

    #[test]
    fn design_mode_prioritizes_docs_context() {
        let hits = vec![
            hit("src/lib.rs", 0.91, "impl detail"),
            hit("docs/design.md", 0.82, "architecture overview"),
        ];

        let context = context_from_hits(
            &hits,
            &HashMap::new(),
            Some(ContextMode::Design),
            10_000,
            10_000,
        );
        assert_eq!(context.files[0].path, "docs/design.md");
    }

    #[test]
    fn bugfix_mode_prioritizes_test_context() {
        let hits = vec![
            hit("src/lib.rs", 0.84, "production code"),
            hit("tests/regression.rs", 0.75, "failing regression"),
        ];

        let context = context_from_hits(
            &hits,
            &HashMap::new(),
            Some(ContextMode::Bugfix),
            10_000,
            10_000,
        );
        assert_eq!(context.files[0].path, "tests/regression.rs");
    }
}
