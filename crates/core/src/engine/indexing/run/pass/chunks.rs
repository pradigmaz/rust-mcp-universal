use anyhow::Result;
use rusqlite::params;

use super::super::types::RunStats;
use super::stats;
use crate::artifact_fingerprint::ArtifactFingerprintBuilder;
use crate::engine::{chunks as engine_chunks, storage};
use crate::vector_rank::{embed_for_index, vector_to_json};

pub(super) struct ChunkIndexArtifacts {
    pub(super) chunk_vectors: Vec<Vec<f32>>,
    pub(super) chunk_manifest_count: i64,
    pub(super) chunk_manifest_hash: String,
    pub(super) chunk_embedding_count: i64,
    pub(super) chunk_embedding_hash: String,
}

pub(super) fn index_file_chunks(
    tx: &rusqlite::Transaction<'_>,
    rel_text: &str,
    full_text: &str,
    semantic_model: &str,
    semantic_dim: i64,
    indexed_at: &str,
    stats_state: &mut RunStats,
) -> Result<ChunkIndexArtifacts> {
    let mut chunk_vectors = Vec::new();
    let mut chunk_manifest_count = 0_i64;
    let mut chunk_manifest_fingerprint = ArtifactFingerprintBuilder::default();
    let mut chunk_embedding_count = 0_i64;
    let mut chunk_embedding_fingerprint = ArtifactFingerprintBuilder::default();
    for chunk in engine_chunks::build_chunks_with_context(full_text) {
        let chunk_idx = i64::try_from(chunk.chunk_idx).unwrap_or(i64::MAX);
        let start_line = i64::try_from(chunk.start_line).unwrap_or(i64::MAX);
        let end_line = i64::try_from(chunk.end_line).unwrap_or(i64::MAX);
        tx.execute(
            "INSERT INTO file_chunks(path, chunk_hash, chunk_idx, start_line, end_line, excerpt)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                rel_text,
                &chunk.chunk_hash,
                chunk_idx,
                start_line,
                end_line,
                &chunk.text
            ],
        )?;
        chunk_manifest_count += 1;
        chunk_manifest_fingerprint.add_chunk_manifest_entry(
            &chunk.chunk_hash,
            chunk_idx,
            Some(start_line),
            Some(end_line),
            &chunk.text,
        );

        let cache_entry = storage::load_cached_chunk_embedding(
            tx,
            &chunk.chunk_hash,
            semantic_model,
            usize::try_from(semantic_dim).unwrap_or(0),
        )?;
        let vector = if let Some(vector) = cache_entry {
            stats::mark_embedding_cache_hit(stats_state);
            vector
        } else {
            stats::mark_embedding_cache_miss(stats_state);
            let computed = embed_for_index(&chunk.text);
            let json = vector_to_json(&computed)?;
            tx.execute(
                "INSERT INTO chunk_embeddings(chunk_hash, model, dim, vector_json, created_at_utc)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(chunk_hash, model) DO UPDATE
                     SET dim = excluded.dim, vector_json = excluded.vector_json, created_at_utc = excluded.created_at_utc",
                params![&chunk.chunk_hash, semantic_model, semantic_dim, &json, indexed_at],
            )?;
            computed
        };
        let vector_json = vector_to_json(&vector)?;
        chunk_embedding_count += 1;
        chunk_embedding_fingerprint.add_chunk_embedding_entry(
            &chunk.chunk_hash,
            chunk_idx,
            semantic_dim,
            &vector_json,
        );
        chunk_vectors.push(vector);
    }
    Ok(ChunkIndexArtifacts {
        chunk_vectors,
        chunk_manifest_count,
        chunk_manifest_hash: chunk_manifest_fingerprint.finish(),
        chunk_embedding_count,
        chunk_embedding_hash: chunk_embedding_fingerprint.finish(),
    })
}
