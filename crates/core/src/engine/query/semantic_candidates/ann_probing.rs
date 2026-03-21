use anyhow::Result;
use rusqlite::{Connection, params};

use crate::vector_rank::ann_bucket_keys;

use super::telemetry::RetrievalTelemetry;
use super::{
    RawSemanticCandidateRow, SemanticFileCandidateBatch, ann_accept_floor,
    ann_probe_limit_with_factor, score_semantic_candidates,
};

pub(super) fn semantic_candidates_from_ann(
    conn: &Connection,
    query_vec: &[f32],
    model_name: &str,
    candidate_limit: usize,
    probe_factor: f32,
    strict_corruption: bool,
) -> Result<Option<SemanticFileCandidateBatch>> {
    let keys = ann_bucket_keys(query_vec);
    if keys.len() < 4 {
        return Ok(None);
    }

    let probe_limit = ann_probe_limit_with_factor(candidate_limit, probe_factor);
    let mut stmt = conn.prepare(
        r#"
        SELECT
            sv.path,
            sv.vector_json,
            f.sample,
            f.size_bytes,
            f.language,
            COUNT(1) AS bucket_hits
        FROM semantic_ann_buckets ab
        JOIN semantic_vectors sv
            ON sv.path = ab.path
           AND sv.model = ab.model
        JOIN files f ON f.path = sv.path
        WHERE ab.model = ?1
          AND (
                (ab.bucket_family = ?2 AND ab.bucket_key = ?3)
             OR (ab.bucket_family = ?4 AND ab.bucket_key = ?5)
             OR (ab.bucket_family = ?6 AND ab.bucket_key = ?7)
             OR (ab.bucket_family = ?8 AND ab.bucket_key = ?9)
          )
        GROUP BY sv.path, sv.vector_json, f.sample, f.size_bytes, f.language
        ORDER BY bucket_hits DESC, sv.path ASC
        LIMIT ?10
        "#,
    )?;
    let rows = stmt
        .query_map(
            params![
                model_name,
                keys[0].0,
                &keys[0].1,
                keys[1].0,
                &keys[1].1,
                keys[2].0,
                &keys[2].1,
                keys[3].0,
                &keys[3].1,
                i64::try_from(probe_limit).unwrap_or(i64::MAX)
            ],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rows.is_empty() {
        return Ok(None);
    }

    let rows = rows
        .into_iter()
        .map(
            |(path, vector_json, sample, size_bytes, language, _bucket_hits)| {
                (path, vector_json, sample, size_bytes, language)
            },
        )
        .collect::<Vec<RawSemanticCandidateRow>>();
    let mut batch = score_semantic_candidates(
        rows,
        query_vec,
        candidate_limit,
        RetrievalTelemetry::ann(),
        strict_corruption,
    )?;
    let ann_floor = ann_accept_floor(candidate_limit);
    if batch.candidates.len() < ann_floor {
        return Ok(None);
    }
    batch.candidates.truncate(candidate_limit);
    Ok(Some(batch))
}
