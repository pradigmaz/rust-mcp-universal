use std::path::Path;

use walkdir::DirEntry;

use super::super::types::PassResult;
use crate::utils::normalize_path;

pub(super) fn resolve_walk_entry(
    entry: Result<DirEntry, walkdir::Error>,
    project_root: &Path,
    pass_result: &mut PassResult,
) -> Option<DirEntry> {
    match entry {
        Ok(value) => Some(value),
        Err(err) => {
            let should_preserve_snapshot = err
                .io_error()
                .is_none_or(|io| io.kind() != std::io::ErrorKind::NotFound);
            if let Some(raw_path) = err.path() {
                if let Ok(relative) = raw_path.strip_prefix(project_root) {
                    let rel_text = normalize_path(relative);
                    if should_preserve_snapshot && !rel_text.is_empty() {
                        pass_result.failed_walk_prefixes.push(rel_text);
                    }
                }
            }
            None
        }
    }
}
