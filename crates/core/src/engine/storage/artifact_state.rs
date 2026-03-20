use std::collections::HashMap;

use anyhow::Result;

use crate::artifact_fingerprint::{
    ArtifactFingerprintBuilder, empty_artifact_content_hash, sample_content_hash,
    semantic_vector_content_hash,
};

#[derive(Debug, Clone)]
pub(super) struct ActualCountedArtifactState {
    pub(super) count: i64,
    pub(super) content_hash: String,
}

impl Default for ActualCountedArtifactState {
    fn default() -> Self {
        Self {
            count: 0,
            content_hash: empty_artifact_content_hash(),
        }
    }
}

#[derive(Debug, Default)]
struct ActualCountedArtifactStateBuilder {
    count: i64,
    fingerprint: ArtifactFingerprintBuilder,
}

pub(super) fn load_actual_fts_state(
    tx: &rusqlite::Transaction<'_>,
) -> Result<HashMap<String, String>> {
    let mut stmt = tx.prepare("SELECT path, sample FROM files_fts ORDER BY path ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut out = HashMap::new();
    for row in rows {
        let (path, sample) = row?;
        out.insert(path, sample_content_hash(&sample));
    }
    Ok(out)
}

pub(super) fn load_actual_chunk_manifest_state(
    tx: &rusqlite::Transaction<'_>,
) -> Result<HashMap<String, ActualCountedArtifactState>> {
    let mut by_path = HashMap::<String, ActualCountedArtifactStateBuilder>::new();
    let mut stmt = tx.prepare(
        "SELECT path, chunk_hash, chunk_idx, start_line, end_line, excerpt
         FROM file_chunks
         ORDER BY path ASC, chunk_idx ASC, chunk_hash ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, Option<i64>>(3)?,
            row.get::<_, Option<i64>>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;
    for row in rows {
        let (path, chunk_hash, chunk_idx, start_line, end_line, excerpt) = row?;
        let state = by_path.entry(path).or_default();
        state.count += 1;
        state.fingerprint.add_chunk_manifest_entry(
            &chunk_hash,
            chunk_idx,
            start_line,
            end_line,
            &excerpt,
        );
    }
    finalize_counted_artifact_state(by_path)
}

pub(super) fn load_actual_chunk_embedding_state(
    tx: &rusqlite::Transaction<'_>,
    semantic_model: &str,
) -> Result<HashMap<String, ActualCountedArtifactState>> {
    let mut by_path = HashMap::<String, ActualCountedArtifactStateBuilder>::new();
    let mut stmt = tx.prepare(
        "SELECT fc.path, fc.chunk_hash, fc.chunk_idx, ce.dim, ce.vector_json
         FROM file_chunks fc
         JOIN chunk_embeddings ce
           ON ce.chunk_hash = fc.chunk_hash
          AND ce.model = ?1
         ORDER BY fc.path ASC, fc.chunk_idx ASC, fc.chunk_hash ASC",
    )?;
    let rows = stmt.query_map([semantic_model], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;
    for row in rows {
        let (path, chunk_hash, chunk_idx, dim, vector_json) = row?;
        let state = by_path.entry(path).or_default();
        state.count += 1;
        state
            .fingerprint
            .add_chunk_embedding_entry(&chunk_hash, chunk_idx, dim, &vector_json);
    }
    finalize_counted_artifact_state(by_path)
}

pub(super) fn load_actual_semantic_vector_state(
    tx: &rusqlite::Transaction<'_>,
    semantic_model: &str,
) -> Result<HashMap<String, String>> {
    let mut stmt = tx.prepare(
        "SELECT path, dim, vector_json
         FROM semantic_vectors
         WHERE model = ?1
         ORDER BY path ASC",
    )?;
    let rows = stmt.query_map([semantic_model], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut out = HashMap::new();
    for row in rows {
        let (path, dim, vector_json) = row?;
        out.insert(path, semantic_vector_content_hash(dim, &vector_json));
    }
    Ok(out)
}

pub(super) fn load_actual_ann_bucket_state(
    tx: &rusqlite::Transaction<'_>,
    semantic_model: &str,
) -> Result<HashMap<String, ActualCountedArtifactState>> {
    let mut by_path = HashMap::<String, ActualCountedArtifactStateBuilder>::new();
    let mut stmt = tx.prepare(
        "SELECT path, bucket_family, bucket_key
         FROM semantic_ann_buckets
         WHERE model = ?1
         ORDER BY path ASC, bucket_family ASC, bucket_key ASC",
    )?;
    let rows = stmt.query_map([semantic_model], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (path, bucket_family, bucket_key) = row?;
        let state = by_path.entry(path).or_default();
        state.count += 1;
        state.fingerprint.add_ann_bucket(bucket_family, &bucket_key);
    }
    finalize_counted_artifact_state(by_path)
}

fn finalize_counted_artifact_state(
    by_path: HashMap<String, ActualCountedArtifactStateBuilder>,
) -> Result<HashMap<String, ActualCountedArtifactState>> {
    let mut out = HashMap::with_capacity(by_path.len());
    for (path, state) in by_path {
        out.insert(
            path,
            ActualCountedArtifactState {
                count: state.count,
                content_hash: state.fingerprint.finish(),
            },
        );
    }
    Ok(out)
}
