use std::collections::HashMap;

use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::model::SearchHit;
use crate::query_profile::derive_query_profile;
use crate::search_db::{extract_tokens, graph_boost};

pub(super) fn build_lexical_by_path(
    conn: &Connection,
    query: &str,
    lexical_hits: &[SearchHit],
    used_like_fallback: bool,
) -> Result<HashMap<String, (f32, f32)>> {
    let mut lexical_by_path = HashMap::with_capacity(lexical_hits.len());
    if used_like_fallback {
        for hit in lexical_hits {
            lexical_by_path.insert(hit.path.clone(), (hit.score.max(0.0), 0.0));
        }
        return Ok(lexical_by_path);
    }

    let tokens = extract_tokens(query);
    let query_profile = derive_query_profile(query);
    for hit in lexical_hits {
        let graph = graph_boost(conn, &hit.path, &tokens, query_profile)
            .with_context(|| format!("failed to compute graph score for `{}`", hit.path))?;
        let lexical = (hit.score - graph).max(0.0);
        lexical_by_path.insert(hit.path.clone(), (lexical, graph));
    }
    Ok(lexical_by_path)
}
