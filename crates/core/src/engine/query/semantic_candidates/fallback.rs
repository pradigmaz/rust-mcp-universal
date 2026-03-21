use anyhow::Result;
use rusqlite::Connection;

use super::telemetry::RetrievalTelemetry;
use super::{RawSemanticCandidateRow, SemanticFileCandidateBatch, score_semantic_candidates};

pub(super) fn semantic_candidates_from_full_scan(
    conn: &Connection,
    query_vec: &[f32],
    model_name: &str,
    candidate_limit: usize,
    strict_corruption: bool,
) -> Result<SemanticFileCandidateBatch> {
    let mut stmt = conn.prepare(
        r#"
        SELECT sv.path, sv.vector_json, f.sample, f.size_bytes, f.language
        FROM semantic_vectors sv
        JOIN files f ON f.path = sv.path
        WHERE sv.model = ?1
        "#,
    )?;
    let rows = stmt
        .query_map([model_name], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<RawSemanticCandidateRow>>>()?;
    score_semantic_candidates(
        rows,
        query_vec,
        candidate_limit,
        RetrievalTelemetry::fallback(),
        strict_corruption,
    )
}
