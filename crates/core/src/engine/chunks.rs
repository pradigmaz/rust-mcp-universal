use crate::utils::hash_bytes;

const CHUNK_TARGET_CHARS: usize = 1200;

#[derive(Debug, Clone)]
pub(super) struct TextChunk {
    pub(super) chunk_hash: String,
    pub(super) chunk_idx: usize,
    pub(super) start_line: usize,
    pub(super) end_line: usize,
    pub(super) text: String,
}

pub(super) fn build_chunks_with_context(text: &str) -> Vec<TextChunk> {
    let lines = text.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        let hash = hash_bytes(text.as_bytes());
        return vec![TextChunk {
            chunk_hash: hash,
            chunk_idx: 0,
            start_line: 1,
            end_line: 1,
            text: text.to_string(),
        }];
    }

    let mut out = Vec::new();
    let mut start = 0_usize;
    let mut chunk_idx = 0_usize;

    while start < lines.len() {
        let mut end = start;
        let mut chars = 0_usize;
        while end < lines.len() {
            let next = lines[end];
            let projected = chars + next.chars().count() + 1;
            if end > start && projected > CHUNK_TARGET_CHARS {
                break;
            }
            chars = projected;
            end += 1;
        }
        if end == start {
            end += 1;
        }

        let leading = if start > 0 { lines[start - 1] } else { "" };
        let trailing = if end < lines.len() { lines[end] } else { "" };
        let body = lines[start..end].join("\n");
        let hash_input = format!("{leading}\n{body}\n{trailing}");

        out.push(TextChunk {
            chunk_hash: hash_bytes(hash_input.as_bytes()),
            chunk_idx,
            start_line: start + 1,
            end_line: end,
            text: body,
        });
        chunk_idx += 1;
        start = end;
    }

    out
}

pub(super) fn aggregate_chunk_vectors(chunks: &[Vec<f32>]) -> Option<Vec<f32>> {
    let first = chunks.first()?;
    if first.is_empty() {
        return None;
    }
    let dim = first.len();
    let mut aggregated = vec![0.0_f32; dim];
    for vector in chunks {
        if vector.len() != dim {
            continue;
        }
        for (slot, value) in aggregated.iter_mut().zip(vector) {
            *slot += *value;
        }
    }
    let n = chunks.len() as f32;
    if n > 0.0 {
        for value in &mut aggregated {
            *value /= n;
        }
    }
    normalize_dynamic(&mut aggregated);
    Some(aggregated)
}

fn normalize_dynamic(values: &mut [f32]) {
    let norm = values.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm <= 0.0 {
        return;
    }
    for value in values {
        *value /= norm;
    }
}
