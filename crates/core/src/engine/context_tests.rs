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
