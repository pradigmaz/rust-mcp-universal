use crate::embedding_backend::{EmbeddingBackend, active_backend, transformer_embedding};

use super::{LOCAL_BLEND_WEIGHT, TRANSFORMER_BLEND_WEIGHT, VECTOR_DIM};

pub(super) fn embed_for_index(text: &str) -> Vec<f32> {
    embed(text).to_vec()
}

pub(super) fn embed(text: &str) -> [f32; VECTOR_DIM] {
    let local = embed_local_dense(text);
    match active_backend() {
        EmbeddingBackend::LocalDense => local,
        EmbeddingBackend::Transformer(config) => match transformer_embedding(&config, text) {
            Ok(transformer) => mix_local_and_transformer(&local, &transformer),
            Err(_) => local,
        },
    }
}

fn embed_local_dense(text: &str) -> [f32; VECTOR_DIM] {
    let mut v = [0.0_f32; VECTOR_DIM];

    for token in tokenize_terms(text) {
        let idx = hash_token(&token) % VECTOR_DIM;
        v[idx] += 1.0;
    }

    // Character 3-grams improve locality on identifiers/symbol fragments.
    let normalized = text.to_lowercase();
    let chars = normalized.chars().collect::<Vec<_>>();
    if chars.len() >= 3 {
        for window in chars.windows(3) {
            let trigram = window.iter().collect::<String>();
            let idx = hash_token(&trigram) % VECTOR_DIM;
            v[idx] += 0.35;
        }
    }

    normalize(&mut v);
    v
}

pub(super) fn tokenize_terms(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .map(str::trim)
        .filter(|term| term.chars().count() >= 2)
        .map(|term| term.to_lowercase())
        .collect()
}

fn mix_local_and_transformer(
    local_dense: &[f32; VECTOR_DIM],
    transformer_embedding: &[f32],
) -> [f32; VECTOR_DIM] {
    if transformer_embedding.is_empty() {
        return *local_dense;
    }

    let transformer_projected = project_transformer_to_local_dim(transformer_embedding);
    let mut mixed = [0.0_f32; VECTOR_DIM];
    for i in 0..VECTOR_DIM {
        mixed[i] = (local_dense[i] * LOCAL_BLEND_WEIGHT)
            + (transformer_projected[i] * TRANSFORMER_BLEND_WEIGHT);
    }
    normalize(&mut mixed);
    mixed
}

fn project_transformer_to_local_dim(input: &[f32]) -> [f32; VECTOR_DIM] {
    let mut projected = [0.0_f32; VECTOR_DIM];
    for (index, value) in input.iter().enumerate() {
        if !value.is_finite() {
            continue;
        }
        let slot = projection_slot(index);
        projected[slot] += *value;
    }
    normalize(&mut projected);
    projected
}

fn projection_slot(index: usize) -> usize {
    let mut x = (index as u64).wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x as usize) % VECTOR_DIM
}

fn hash_token(token: &str) -> usize {
    let mut h: usize = 2166136261;
    for b in token.as_bytes() {
        h ^= usize::from(*b);
        h = h.wrapping_mul(16777619);
    }
    h
}

fn normalize(v: &mut [f32; VECTOR_DIM]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}
