use std::path::Path;

use crate::utils::{decode_normalized_path, normalize_path, normalized_path_to_fs_path};

pub(crate) fn display_path(path: &str) -> String {
    decode_normalized_path(path).unwrap_or_else(|| path.to_string())
}

pub(crate) fn source_fs_path(path: &str) -> std::path::PathBuf {
    normalized_path_to_fs_path(path)
}

pub(crate) fn index_lookup_paths(path: &str) -> Vec<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut variants = Vec::new();
    push_unique(&mut variants, trimmed.to_string());

    if let Some(decoded) = decode_normalized_path(trimmed) {
        push_unique(&mut variants, normalize_path(Path::new(&decoded)));
        push_unique(&mut variants, decoded);
    } else {
        push_unique(&mut variants, normalize_path(Path::new(trimmed)));
    }

    variants
}

fn push_unique(items: &mut Vec<String>, value: String) {
    if !items.iter().any(|item| item == &value) {
        items.push(value);
    }
}
