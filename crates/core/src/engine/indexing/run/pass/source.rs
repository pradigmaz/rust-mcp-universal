use std::fs;
use std::path::Path;

use anyhow::Result;
use time::OffsetDateTime;

use crate::utils::{SAMPLE_LIMIT, hash_bytes, infer_language};

pub(super) struct SourceMetadata {
    pub(super) size_bytes: u64,
    pub(super) current_mtime_unix_ms: Option<i64>,
}

pub(super) struct SourceSnapshot {
    pub(super) full_text: String,
    pub(super) sample: String,
    pub(super) sha256: String,
    pub(super) language: String,
    pub(super) is_binary: bool,
}

pub(super) fn read_source_metadata(path: &Path) -> std::io::Result<SourceMetadata> {
    let metadata = fs::metadata(path)?;
    Ok(SourceMetadata {
        size_bytes: metadata.len(),
        current_mtime_unix_ms: metadata
            .modified()
            .ok()
            .map(super::filters::system_time_to_unix_ms),
    })
}

pub(super) fn read_source_snapshot(path: &Path) -> std::io::Result<SourceSnapshot> {
    let bytes = fs::read(path)?;
    let is_binary = bytes.contains(&0);
    let full_text = String::from_utf8_lossy(&bytes).to_string();
    let sample = full_text.chars().take(SAMPLE_LIMIT).collect::<String>();
    Ok(SourceSnapshot {
        full_text,
        sample,
        sha256: hash_bytes(&bytes),
        language: infer_language(path),
        is_binary,
    })
}

pub(super) fn now_indexed_at() -> Result<String> {
    Ok(OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?)
}
