use std::fs;

use anyhow::Result;

use crate::engine::Engine;
use crate::quality::{
    IndexedQualityMetrics, build_indexed_quality_facts, build_oversize_quality_facts,
};
use crate::utils::{INDEX_FILE_LIMIT, infer_language, normalized_path_to_fs_path};

#[derive(Debug)]
pub(super) struct QualityRefreshInput {
    pub(super) path: String,
    pub(super) language: String,
    pub(super) source_mtime_unix_ms: Option<i64>,
    pub(super) facts: crate::quality::QualityCandidateFacts,
    pub(super) indexed_metrics: IndexedQualityMetrics,
    pub(super) source_text: Option<String>,
}

pub(super) fn build_refresh_input(
    conn: &rusqlite::Connection,
    engine: &Engine,
    path: &str,
    structural: crate::quality::StructuralFacts,
    layering: crate::quality::LayeringFacts,
    git_risk: crate::quality::GitRiskFacts,
) -> Result<Option<QualityRefreshInput>> {
    let abs_path = engine.project_root.join(normalized_path_to_fs_path(path));
    let metadata = match fs::metadata(&abs_path) {
        Ok(metadata) => metadata,
        Err(_) => return Ok(None),
    };
    let source_mtime_unix_ms = metadata.modified().ok().map(system_time_to_unix_ms);
    let language = infer_language(&abs_path);
    let indexed_metrics = load_indexed_quality_metrics(conn, path)?;
    if metadata.len() > INDEX_FILE_LIMIT {
        let mut facts =
            build_oversize_quality_facts(path, &language, metadata.len(), source_mtime_unix_ms);
        facts.structural = structural;
        facts.layering = layering;
        facts.git_risk = git_risk;
        let source_text = fs::read(&abs_path).ok().and_then(|bytes| {
            (!bytes.contains(&0)).then(|| String::from_utf8_lossy(&bytes).to_string())
        });
        return Ok(Some(QualityRefreshInput {
            path: path.to_string(),
            language,
            source_mtime_unix_ms,
            facts,
            indexed_metrics,
            source_text,
        }));
    }
    let bytes = match fs::read(&abs_path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };
    if bytes.contains(&0) {
        return Ok(None);
    }
    let full_text = String::from_utf8_lossy(&bytes).to_string();
    let mut facts = build_indexed_quality_facts(
        path,
        &language,
        metadata.len(),
        source_mtime_unix_ms,
        &full_text,
    );
    facts.structural = structural;
    facts.layering = layering;
    facts.git_risk = git_risk;
    Ok(Some(QualityRefreshInput {
        path: path.to_string(),
        language,
        source_mtime_unix_ms,
        facts,
        indexed_metrics,
        source_text: Some(full_text),
    }))
}

fn load_indexed_quality_metrics(
    conn: &rusqlite::Connection,
    path: &str,
) -> Result<IndexedQualityMetrics> {
    Ok(conn
        .query_row(
            r#"
            SELECT graph_symbol_count, graph_ref_count, graph_module_dep_count, graph_edge_out_count
            FROM files
            WHERE path = ?1
            "#,
            [path],
            |row| {
                Ok(IndexedQualityMetrics {
                    symbol_count: row.get(0)?,
                    ref_count: row.get(1)?,
                    module_dep_count: row.get(2)?,
                    graph_edge_out_count: row.get(3)?,
                })
            },
        )
        .unwrap_or_default())
}

fn system_time_to_unix_ms(time: std::time::SystemTime) -> i64 {
    time.duration_since(std::time::UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}
