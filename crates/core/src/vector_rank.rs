use anyhow::Result;

use crate::embedding_backend::semantic_model_name as backend_model_name;

mod ann;
mod embedding;

const VECTOR_DIM: usize = 192;
const LOCAL_BLEND_WEIGHT: f32 = 0.35;
const TRANSFORMER_BLEND_WEIGHT: f32 = 0.65;
const ANN_BUCKET_FAMILIES: usize = 4;
const ANN_BITS_PER_FAMILY: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticRerankOutcome {
    NotApplied,
    Failed,
    ShortCircuitedLexical,
    AppliedRrfFallback,
    AppliedRrfIndexed,
    AppliedRrfMixed,
}

pub fn semantic_model_name() -> String {
    backend_model_name()
}

pub const fn vector_dim() -> usize {
    VECTOR_DIM
}

pub fn ann_bucket_keys(vector: &[f32]) -> Vec<(i64, String)> {
    ann::ann_bucket_keys_impl(vector)
}

pub fn embed_for_index(text: &str) -> Vec<f32> {
    embedding::embed_for_index(text)
}

pub fn vector_to_json(vector: &[f32]) -> Result<String> {
    Ok(serde_json::to_string(vector)?)
}

#[cfg(test)]
mod tests;
