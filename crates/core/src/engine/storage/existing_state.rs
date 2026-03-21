use std::collections::HashMap;

use anyhow::Result;

use super::artifact_state::{
    load_actual_ann_bucket_state, load_actual_chunk_embedding_state,
    load_actual_chunk_manifest_state, load_actual_fts_state, load_actual_semantic_vector_state,
};
use super::graph_state::{load_actual_graph_edge_state, load_actual_graph_state};

#[path = "existing_state/completeness.rs"]
mod completeness;

pub(in crate::engine) use completeness::state_completeness_report;
#[cfg(test)]
pub(in crate::engine) use completeness::FileStateSection;

#[derive(Debug, Clone)]
pub(in crate::engine) struct ExistingFileState {
    pub(in crate::engine) sha256: String,
    pub(in crate::engine) source_mtime_unix_ms: Option<i64>,
    pub(in crate::engine) artifact_fingerprint_version: Option<i64>,
    pub(in crate::engine) fts_sample_hash: Option<String>,
    pub(in crate::engine) chunk_manifest_count: Option<i64>,
    pub(in crate::engine) chunk_manifest_hash: Option<String>,
    pub(in crate::engine) chunk_embedding_count: Option<i64>,
    pub(in crate::engine) chunk_embedding_hash: Option<String>,
    pub(in crate::engine) semantic_vector_hash: Option<String>,
    pub(in crate::engine) ann_bucket_count: Option<i64>,
    pub(in crate::engine) ann_bucket_hash: Option<String>,
    pub(in crate::engine) graph_symbol_count: Option<i64>,
    pub(in crate::engine) graph_ref_count: Option<i64>,
    pub(in crate::engine) graph_module_dep_count: Option<i64>,
    pub(in crate::engine) graph_content_hash: Option<String>,
    pub(in crate::engine) graph_fingerprint_version: Option<i64>,
    pub(in crate::engine) graph_edge_out_count: Option<i64>,
    pub(in crate::engine) graph_edge_in_count: Option<i64>,
    pub(in crate::engine) graph_edge_hash: Option<String>,
    pub(in crate::engine) graph_edge_fingerprint_version: Option<i64>,
    pub(in crate::engine) actual_fts_sample_hash: Option<String>,
    pub(in crate::engine) actual_chunk_manifest_count: i64,
    pub(in crate::engine) actual_chunk_manifest_hash: String,
    pub(in crate::engine) actual_chunk_embedding_count: i64,
    pub(in crate::engine) actual_chunk_embedding_hash: String,
    pub(in crate::engine) actual_semantic_vector_hash: Option<String>,
    pub(in crate::engine) actual_ann_bucket_count: i64,
    pub(in crate::engine) actual_ann_bucket_hash: String,
    pub(in crate::engine) actual_graph_symbol_count: i64,
    pub(in crate::engine) actual_graph_ref_count: i64,
    pub(in crate::engine) actual_graph_module_dep_count: i64,
    pub(in crate::engine) actual_graph_content_hash: String,
    pub(in crate::engine) actual_graph_edge_out_count: i64,
    pub(in crate::engine) actual_graph_edge_in_count: i64,
    pub(in crate::engine) actual_graph_edge_hash: String,
}

pub(in crate::engine) fn load_existing_file_state(
    tx: &rusqlite::Transaction<'_>,
    semantic_model: &str,
) -> Result<HashMap<String, ExistingFileState>> {
    let actual_fts_state = load_actual_fts_state(tx)?;
    let actual_chunk_manifest_state = load_actual_chunk_manifest_state(tx)?;
    let actual_chunk_embedding_state = load_actual_chunk_embedding_state(tx, semantic_model)?;
    let actual_semantic_vector_state = load_actual_semantic_vector_state(tx, semantic_model)?;
    let actual_ann_bucket_state = load_actual_ann_bucket_state(tx, semantic_model)?;
    let actual_graph_state = load_actual_graph_state(tx)?;
    let actual_graph_edge_state = load_actual_graph_edge_state(tx)?;
    let mut stmt = tx.prepare(
        r#"
        SELECT
            f.path,
            f.sha256,
            f.source_mtime_unix_ms,
            f.artifact_fingerprint_version,
            f.fts_sample_hash,
            f.chunk_manifest_count,
            f.chunk_manifest_hash,
            f.chunk_embedding_count,
            f.chunk_embedding_hash,
            f.semantic_vector_hash,
            f.ann_bucket_count,
            f.ann_bucket_hash,
            f.graph_symbol_count,
            f.graph_ref_count,
            f.graph_module_dep_count,
            f.graph_content_hash,
            f.graph_fingerprint_version,
            f.graph_edge_out_count,
            f.graph_edge_in_count,
            f.graph_edge_hash,
            f.graph_edge_fingerprint_version
        FROM files f
        "#,
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
                row.get::<_, Option<i64>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, Option<String>>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, Option<String>>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, Option<i64>>(10)?,
                row.get::<_, Option<String>>(11)?,
                row.get::<_, Option<i64>>(12)?,
                row.get::<_, Option<i64>>(13)?,
                row.get::<_, Option<i64>>(14)?,
                row.get::<_, Option<String>>(15)?,
                row.get::<_, Option<i64>>(16)?,
                row.get::<_, Option<i64>>(17)?,
                row.get::<_, Option<i64>>(18)?,
                row.get::<_, Option<String>>(19)?,
                row.get::<_, Option<i64>>(20)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut out = HashMap::with_capacity(rows.len());
    for (
        path,
        sha256,
        source_mtime_unix_ms,
        artifact_fingerprint_version,
        fts_sample_hash,
        chunk_manifest_count,
        chunk_manifest_hash,
        chunk_embedding_count,
        chunk_embedding_hash,
        semantic_vector_hash,
        ann_bucket_count,
        ann_bucket_hash,
        graph_symbol_count,
        graph_ref_count,
        graph_module_dep_count,
        graph_content_hash,
        graph_fingerprint_version,
        graph_edge_out_count,
        graph_edge_in_count,
        graph_edge_hash,
        graph_edge_fingerprint_version,
    ) in rows
    {
        let actual_fts_sample_hash = actual_fts_state.get(&path).cloned();
        let actual_chunk_manifest_state = actual_chunk_manifest_state
            .get(&path)
            .cloned()
            .unwrap_or_default();
        let actual_chunk_embedding_state = actual_chunk_embedding_state
            .get(&path)
            .cloned()
            .unwrap_or_default();
        let actual_semantic_vector_hash = actual_semantic_vector_state.get(&path).cloned();
        let actual_ann_bucket_state = actual_ann_bucket_state
            .get(&path)
            .cloned()
            .unwrap_or_default();
        let actual_graph_state = actual_graph_state.get(&path).cloned().unwrap_or_default();
        let actual_graph_edge_state = actual_graph_edge_state
            .get(&path)
            .cloned()
            .unwrap_or_default();
        out.insert(
            path,
            ExistingFileState {
                sha256,
                source_mtime_unix_ms,
                artifact_fingerprint_version,
                fts_sample_hash,
                chunk_manifest_count,
                chunk_manifest_hash,
                chunk_embedding_count,
                chunk_embedding_hash,
                semantic_vector_hash,
                ann_bucket_count,
                ann_bucket_hash,
                graph_symbol_count,
                graph_ref_count,
                graph_module_dep_count,
                graph_content_hash,
                graph_fingerprint_version,
                graph_edge_out_count,
                graph_edge_in_count,
                graph_edge_hash,
                graph_edge_fingerprint_version,
                actual_fts_sample_hash,
                actual_chunk_manifest_count: actual_chunk_manifest_state.count,
                actual_chunk_manifest_hash: actual_chunk_manifest_state.content_hash,
                actual_chunk_embedding_count: actual_chunk_embedding_state.count,
                actual_chunk_embedding_hash: actual_chunk_embedding_state.content_hash,
                actual_semantic_vector_hash,
                actual_ann_bucket_count: actual_ann_bucket_state.count,
                actual_ann_bucket_hash: actual_ann_bucket_state.content_hash,
                actual_graph_symbol_count: actual_graph_state.symbol_count,
                actual_graph_ref_count: actual_graph_state.ref_count,
                actual_graph_module_dep_count: actual_graph_state.module_dep_count,
                actual_graph_content_hash: actual_graph_state.content_hash,
                actual_graph_edge_out_count: actual_graph_edge_state.outgoing_count,
                actual_graph_edge_in_count: actual_graph_edge_state.incoming_count,
                actual_graph_edge_hash: actual_graph_edge_state.content_hash,
            },
        );
    }
    Ok(out)
}
