use anyhow::Result;
use rusqlite::Connection;

use super::meta_store::{count_files, read_meta_raw, read_meta_u32_lossy};
use super::{IndexCompatibilityDecision, expected_index_meta};
use super::{
    META_ANN_VERSION, META_EMBEDDING_DIM, META_EMBEDDING_MODEL_ID, META_INDEX_FORMAT_VERSION,
};

pub(crate) fn evaluate_index_compatibility(
    conn: &Connection,
) -> Result<IndexCompatibilityDecision> {
    if count_files(conn)? == 0 {
        return Ok(IndexCompatibilityDecision::Compatible);
    }

    let expected = expected_index_meta();
    let mut reasons = Vec::new();

    let index_format_version = read_meta_u32_lossy(conn, META_INDEX_FORMAT_VERSION, &mut reasons)?;
    let embedding_model_id = read_meta_raw(conn, META_EMBEDDING_MODEL_ID)?;
    let embedding_dim = read_meta_u32_lossy(conn, META_EMBEDDING_DIM, &mut reasons)?;
    let ann_version = read_meta_u32_lossy(conn, META_ANN_VERSION, &mut reasons)?;

    match index_format_version {
        Some(actual) if actual == expected.index_format_version => {}
        Some(actual) => reasons.push(format!(
            "index_format_version mismatch: db={actual}, expected={}",
            expected.index_format_version
        )),
        None => reasons.push("missing index_format_version metadata".to_string()),
    }

    match embedding_model_id {
        Some(actual) if actual == expected.embedding_model_id => {}
        Some(actual) => reasons.push(format!(
            "embedding_model_id mismatch: db={actual}, expected={}",
            expected.embedding_model_id
        )),
        None => reasons.push("missing embedding_model_id metadata".to_string()),
    }

    match embedding_dim {
        Some(actual) if actual == expected.embedding_dim => {}
        Some(actual) => reasons.push(format!(
            "embedding_dim mismatch: db={actual}, expected={}",
            expected.embedding_dim
        )),
        None => reasons.push("missing embedding_dim metadata".to_string()),
    }

    match ann_version {
        Some(actual) if actual == expected.ann_version => {}
        Some(actual) => reasons.push(format!(
            "ann_version mismatch: db={actual}, expected={}",
            expected.ann_version
        )),
        None => reasons.push("missing ann_version metadata".to_string()),
    }

    if reasons.is_empty() {
        Ok(IndexCompatibilityDecision::Compatible)
    } else {
        Ok(IndexCompatibilityDecision::ReindexRequired {
            reason: reasons.join("; "),
        })
    }
}
