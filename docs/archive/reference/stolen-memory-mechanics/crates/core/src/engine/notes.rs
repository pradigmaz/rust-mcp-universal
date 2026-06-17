use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

pub(super) use super::canonical::ParsedNote;
use super::canonical::{parse_canonical_note, root_markdown_files, scan_canonical_locations};

pub(super) fn scan_markdown_files(memory_root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for location in scan_canonical_locations(memory_root) {
        if location == memory_root {
            for file in root_markdown_files(memory_root)? {
                if parse_canonical_note(memory_root, &file, None).is_ok() {
                    files.push(file);
                }
            }
            continue;
        }
        if location.is_file() {
            files.push(location);
            continue;
        }
        if !location.is_dir() {
            continue;
        }
        for entry in WalkDir::new(location) {
            let entry = entry?;
            if entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .is_some_and(|value| value.eq_ignore_ascii_case("md"))
            {
                files.push(entry.into_path());
            }
        }
    }
    files.sort();
    Ok(files)
}

pub(super) fn parse_note(memory_root: &Path, file_path: &Path) -> Result<ParsedNote> {
    parse_canonical_note(memory_root, file_path, None)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{parse_note, scan_markdown_files};

    #[test]
    fn scan_markdown_files_ignores_non_canonical_markdown() {
        let root = tempfile_dir("canonical-scan");
        std::fs::create_dir_all(Path::new(&root).join("decisions")).expect("create dir");
        std::fs::write(
            Path::new(&root).join("README.md"),
            "# Ignored\n\nNot canonical.",
        )
        .expect("write readme");
        std::fs::write(
            Path::new(&root).join("decisions").join("auth.md"),
            "---\nid: decision-auth\ntype: Decision\ntitle: Auth\nstatus: active\nproject: canonical-scan\ncreated_at: 1\nupdated_at: 1\n---\n\n# Auth\n\n## Summary\nShort summary.\n\n## Observations\n\n## Relations\n\n## References\n",
        )
        .expect("write note");

        let files = scan_markdown_files(Path::new(&root)).expect("scan markdown");
        assert_eq!(files.len(), 1);
        let parsed = parse_note(Path::new(&root), &files[0]).expect("parse canonical note");
        assert_eq!(parsed.slug, "auth");

        let _ = std::fs::remove_dir_all(root);
    }

    fn tempfile_dir(prefix: &str) -> std::path::PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{suffix}"))
    }
}
