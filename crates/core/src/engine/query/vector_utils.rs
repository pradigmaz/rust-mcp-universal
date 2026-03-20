use anyhow::{Context, Result, anyhow};

use crate::text_utils;

pub(super) fn parse_vector_with_dim(raw: &str, expected_dim: usize) -> Result<Vec<f32>> {
    let vector =
        serde_json::from_str::<Vec<f32>>(raw).with_context(|| "failed to decode vector json")?;
    if vector.len() != expected_dim {
        return Err(anyhow!(
            "vector dimension mismatch: expected {expected_dim}, got {}",
            vector.len()
        ));
    }
    if let Some((idx, value)) = vector
        .iter()
        .enumerate()
        .find(|(_, value)| !value.is_finite())
    {
        return Err(anyhow!(
            "vector contains non-finite value at index {idx}: {value}"
        ));
    }
    Ok(vector)
}

pub(super) fn cosine_similarity(query_vec: &[f32], candidate_vec: &[f32]) -> f32 {
    let mut sum = 0.0_f32;
    for (lhs, rhs) in query_vec.iter().zip(candidate_vec.iter()) {
        sum += lhs * rhs;
    }
    sum
}

pub(super) fn is_zero_vector(vector: &[f32]) -> bool {
    vector.iter().all(|value| *value == 0.0)
}

pub(super) fn trim_excerpt(text: &str, max_chars: usize) -> String {
    text_utils::trim_compact_text(text, max_chars)
}

pub(super) fn i64_to_usize(value: i64) -> usize {
    text_utils::i64_to_usize_non_negative(value)
}
