use std::collections::HashMap;
use std::time::Instant;

use crate::model::ContextSelection;

pub(in crate::engine) fn derive_chunk_telemetry(context: &ContextSelection) -> (f32, String) {
    if context.files.is_empty() {
        return (0.0, "none".to_string());
    }

    let chunk_coverage =
        (context.chunk_selected as f32 / context.files.len() as f32).clamp(0.0, 1.0);
    if context.chunk_selected == 0 {
        return (chunk_coverage, "none".to_string());
    }

    let mut by_source = HashMap::new();
    for item in &context.files {
        if item.chunk_source == "preview_fallback" {
            continue;
        }
        *by_source
            .entry(item.chunk_source.clone())
            .or_insert(0_usize) += 1;
    }

    let chunk_source = if by_source.is_empty() {
        "none".to_string()
    } else if by_source.len() == 1 {
        by_source
            .into_iter()
            .next()
            .map(|(source, _)| source)
            .unwrap_or_else(|| "none".to_string())
    } else {
        "mixed".to_string()
    };

    (chunk_coverage, chunk_source)
}

pub(in crate::engine) fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
