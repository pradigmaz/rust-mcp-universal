use std::collections::HashMap;
use std::time::Instant;

use anyhow::Result;

use super::Engine;
use super::chunking::best_chunks_for_hits;
use crate::engine::context;
use crate::model::{ContextMode, ContextSelection, SearchHit};

impl Engine {
    pub(in crate::engine) fn context_for_hits_with_chunks(
        &self,
        query: &str,
        hits: &[SearchHit],
        prefetched_chunks: Option<&HashMap<String, context::ChunkExcerpt>>,
        context_mode: Option<ContextMode>,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<ContextSelection> {
        let chunk_map = if let Some(prefetched) = prefetched_chunks {
            let mut filtered = HashMap::with_capacity(hits.len());
            for hit in hits {
                if let Some(chunk) = prefetched.get(&hit.path) {
                    filtered.insert(hit.path.clone(), chunk.clone());
                }
            }
            filtered
        } else {
            let conn = self.open_db()?;
            best_chunks_for_hits(&conn, query, hits)?
        };
        Ok(context::context_from_hits(
            hits,
            &chunk_map,
            context_mode,
            max_chars,
            max_tokens,
        ))
    }
}

pub(in crate::engine) fn derive_chunk_telemetry(context: &ContextSelection) -> (f32, String) {
    if context.files.is_empty() {
        return (0.0, "none".to_string());
    }

    let chunk_coverage =
        (context.chunk_selected as f32 / context.files.len() as f32).clamp(0.0, 1.0);
    if context.chunk_selected == 0 {
        return (chunk_coverage, "none".to_string());
    }

    let mut by_source = HashMap::new();
    for item in &context.files {
        if item.chunk_source == "preview_fallback" {
            continue;
        }
        *by_source
            .entry(item.chunk_source.clone())
            .or_insert(0_usize) += 1;
    }

    let chunk_source = if by_source.is_empty() {
        "none".to_string()
    } else if by_source.len() == 1 {
        by_source
            .into_iter()
            .next()
            .map(|(source, _)| source)
            .unwrap_or_else(|| "none".to_string())
    } else {
        "mixed".to_string()
    };

    (chunk_coverage, chunk_source)
}

pub(in crate::engine) fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
