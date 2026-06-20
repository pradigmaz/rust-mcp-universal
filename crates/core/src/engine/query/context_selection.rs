use std::collections::HashMap;

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
