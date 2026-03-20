use anyhow::Result;
use rusqlite::params;

use super::super::types::RunStats;
use super::graph::GraphArtifacts;
use super::source::{SourceMetadata, SourceSnapshot};
use super::{chunks, vectors};
use crate::artifact_fingerprint::{CURRENT_ARTIFACT_FINGERPRINT_VERSION, sample_content_hash};
use crate::engine::storage;

pub(super) struct PersistFileInput<'a> {
    pub(super) rel_text: &'a str,
    pub(super) source: &'a SourceSnapshot,
    pub(super) metadata: &'a SourceMetadata,
    pub(super) indexed_at: &'a str,
    pub(super) semantic_model: &'a str,
    pub(super) semantic_dim: i64,
    pub(super) graph: &'a GraphArtifacts,
}

pub(super) fn persist_indexed_file(
    tx: &rusqlite::Transaction<'_>,
    input: PersistFileInput<'_>,
    stats: &mut RunStats,
) -> Result<()> {
    let PersistFileInput {
        rel_text,
        source,
        metadata,
        indexed_at,
        semantic_model,
        semantic_dim,
        graph,
    } = input;

    let artifact_fingerprint_version = CURRENT_ARTIFACT_FINGERPRINT_VERSION;
    let fts_sample_hash = sample_content_hash(&source.sample);
    storage::remove_path_index(tx, rel_text)?;
    tx.execute(
        "INSERT INTO files_fts(path, sample) VALUES (?1, ?2)",
        params![rel_text, &source.sample],
    )?;

    for symbol in &graph.graph.symbols {
        tx.execute(
            "INSERT INTO symbols(path, name, kind, language, line, column)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                rel_text,
                &symbol.name,
                &symbol.kind,
                &source.language,
                symbol
                    .line
                    .map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
                symbol
                    .column
                    .map(|value| i64::try_from(value).unwrap_or(i64::MAX))
            ],
        )?;
    }
    for dep in &graph.graph.deps {
        tx.execute(
            "INSERT INTO module_deps(path, dep, language) VALUES (?1, ?2, ?3)",
            params![rel_text, dep, &source.language],
        )?;
    }
    for reference in &graph.graph.refs {
        tx.execute(
            "INSERT INTO refs(path, symbol, language, line, column)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                rel_text,
                &reference.symbol,
                &source.language,
                reference
                    .line
                    .map(|value| i64::try_from(value).unwrap_or(i64::MAX)),
                reference
                    .column
                    .map(|value| i64::try_from(value).unwrap_or(i64::MAX))
            ],
        )?;
    }

    let chunk_artifacts = chunks::index_file_chunks(
        tx,
        rel_text,
        &source.full_text,
        semantic_model,
        semantic_dim,
        indexed_at,
        stats,
    )?;
    let vector_artifacts = vectors::index_semantic_vector(
        tx,
        rel_text,
        semantic_model,
        &source.sample,
        indexed_at,
        &chunk_artifacts.chunk_vectors,
    )?;
    tx.execute(
        "INSERT INTO files(
                path,
                sha256,
                size_bytes,
                language,
                sample,
                indexed_at_utc,
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
                graph_fingerprint_version
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        params![
            rel_text,
            &source.sha256,
            i64::try_from(metadata.size_bytes).unwrap_or(i64::MAX),
            &source.language,
            &source.sample,
            indexed_at,
            metadata.current_mtime_unix_ms,
            artifact_fingerprint_version,
            &fts_sample_hash,
            chunk_artifacts.chunk_manifest_count,
            &chunk_artifacts.chunk_manifest_hash,
            chunk_artifacts.chunk_embedding_count,
            &chunk_artifacts.chunk_embedding_hash,
            &vector_artifacts.semantic_vector_hash,
            vector_artifacts.ann_bucket_count,
            &vector_artifacts.ann_bucket_hash,
            graph.graph_symbol_count,
            graph.graph_ref_count,
            graph.graph_module_dep_count,
            &graph.graph_content_hash,
            graph.graph_fingerprint_version
        ],
    )?;

    Ok(())
}
