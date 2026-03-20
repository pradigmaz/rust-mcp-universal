use anyhow::Result;
use rusqlite::params;

use crate::artifact_fingerprint::{ArtifactFingerprintBuilder, semantic_vector_content_hash};
use crate::engine::chunks as engine_chunks;
use crate::vector_rank::{ann_bucket_keys, embed_for_index, vector_to_json};

pub(super) struct SemanticVectorArtifacts {
    pub(super) semantic_vector_hash: String,
    pub(super) ann_bucket_count: i64,
    pub(super) ann_bucket_hash: String,
}

pub(super) fn index_semantic_vector(
    tx: &rusqlite::Transaction<'_>,
    rel_text: &str,
    semantic_model: &str,
    sample: &str,
    indexed_at: &str,
    chunk_vectors: &[Vec<f32>],
) -> Result<SemanticVectorArtifacts> {
    let vector = engine_chunks::aggregate_chunk_vectors(chunk_vectors)
        .unwrap_or_else(|| embed_for_index(sample));
    let dim = i64::try_from(vector.len()).unwrap_or(i64::MAX);
    let vector_json = vector_to_json(&vector)?;
    tx.execute(
        "INSERT INTO semantic_vectors(path, model, dim, vector_json, indexed_at_utc)
             VALUES (?1, ?2, ?3, ?4, ?5)",
        params![rel_text, semantic_model, dim, &vector_json, indexed_at],
    )?;
    let mut ann_bucket_count = 0_i64;
    let mut ann_bucket_fingerprint = ArtifactFingerprintBuilder::default();
    for (bucket_family, bucket_key) in ann_bucket_keys(&vector) {
        tx.execute(
            "INSERT INTO semantic_ann_buckets(path, model, bucket_family, bucket_key)
                 VALUES (?1, ?2, ?3, ?4)
                 ON CONFLICT(path, model, bucket_family) DO UPDATE
                 SET bucket_key = excluded.bucket_key",
            params![rel_text, semantic_model, bucket_family, &bucket_key],
        )?;
        ann_bucket_count += 1;
        ann_bucket_fingerprint.add_ann_bucket(bucket_family, &bucket_key);
    }
    Ok(SemanticVectorArtifacts {
        semantic_vector_hash: semantic_vector_content_hash(dim, &vector_json),
        ann_bucket_count,
        ann_bucket_hash: ann_bucket_fingerprint.finish(),
    })
}
