use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};

const MANAGED_BLOCK_START: &str = "# --- RMU managed ignore block: start ---";
const MANAGED_BLOCK_END: &str = "# --- RMU managed ignore block: end ---";
const MANAGED_PATTERNS: &[&str] = &[
    ".rmu/",
    ".codex/",
    ".qodo/",
    ".idea/",
    ".vscode/",
    ".DS_Store",
    "Thumbs.db",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GitignoreUpdate {
    pub created: bool,
    pub updated: bool,
}

pub struct ProjectIgnoreMatcher {
    project_root: PathBuf,
    gitignore: Gitignore,
}

pub fn ensure_root_gitignore(project_root: &Path) -> Result<GitignoreUpdate> {
    let gitignore_path = project_root.join(".gitignore");
    if !gitignore_path.exists() {
        fs::write(&gitignore_path, render_managed_block("\n"))
            .with_context(|| format!("failed to create {}", gitignore_path.display()))?;
        return Ok(GitignoreUpdate {
            created: true,
            updated: true,
        });
    }

    let existing = fs::read_to_string(&gitignore_path)
        .with_context(|| format!("failed to read {}", gitignore_path.display()))?;
    let (next, updated) = merge_managed_block(&existing);
    if updated {
        fs::write(&gitignore_path, next)
            .with_context(|| format!("failed to update {}", gitignore_path.display()))?;
    }

    Ok(GitignoreUpdate {
        created: false,
        updated,
    })
}

impl ProjectIgnoreMatcher {
    pub fn new(project_root: &Path) -> Result<Self> {
        let mut builder = GitignoreBuilder::new(project_root);
        let root_gitignore = project_root.join(".gitignore");
        if root_gitignore.is_file() {
            if let Some(err) = builder.add(&root_gitignore) {
                return Err(err)
                    .with_context(|| format!("failed to load {}", root_gitignore.display()));
            }
        }
        let gitignore = builder
            .build()
            .context("failed to build project .gitignore matcher")?;
        Ok(Self {
            project_root: project_root.to_path_buf(),
            gitignore,
        })
    }

    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        let candidate = if path.is_absolute() {
            path.strip_prefix(&self.project_root)
                .unwrap_or(path)
                .to_path_buf()
        } else {
            path.to_path_buf()
        };
        self.gitignore
            .matched_path_or_any_parents(&candidate, is_dir)
            .is_ignore()
    }
}

fn merge_managed_block(existing: &str) -> (String, bool) {
    let line_ending = detect_line_ending(existing);
    let block = render_managed_block(line_ending);
    if let Some((start, end)) = managed_block_bounds(existing) {
        let before = existing[..start].trim_end_matches(['\r', '\n']);
        let after = existing[end..].trim_start_matches(['\r', '\n']);
        let mut merged = String::new();
        if !before.is_empty() {
            merged.push_str(before);
            merged.push_str(line_ending);
            merged.push_str(line_ending);
        }
        merged.push_str(&block);
        if !after.is_empty() {
            merged.push_str(line_ending);
            merged.push_str(line_ending);
            merged.push_str(after);
            if !after.ends_with(['\r', '\n']) {
                merged.push_str(line_ending);
            }
        }
        let updated = merged != existing;
        return (merged, updated);
    }

    let trimmed = existing.trim_end_matches(['\r', '\n']);
    if trimmed.is_empty() {
        return (block.clone(), block != existing);
    }

    let merged = format!("{trimmed}{line_ending}{line_ending}{block}");
    (merged.clone(), merged != existing)
}

fn managed_block_bounds(existing: &str) -> Option<(usize, usize)> {
    let start = existing.find(MANAGED_BLOCK_START)?;
    let end_marker = existing[start..].find(MANAGED_BLOCK_END)?;
    let mut end = start + end_marker + MANAGED_BLOCK_END.len();
    if existing[end..].starts_with("\r\n") {
        end += 2;
    } else if existing[end..].starts_with('\n') {
        end += 1;
    }
    Some((start, end))
}

fn render_managed_block(line_ending: &str) -> String {
    let mut lines = Vec::with_capacity(MANAGED_PATTERNS.len() + 2);
    lines.push(MANAGED_BLOCK_START);
    lines.extend(MANAGED_PATTERNS.iter().copied());
    lines.push(MANAGED_BLOCK_END);
    format!("{}{}", lines.join(line_ending), line_ending)
}

fn detect_line_ending(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

#[cfg(test)]
mod tests {
    use super::{ensure_root_gitignore, merge_managed_block};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn creates_gitignore_with_managed_block_when_missing() {
        let root = temp_dir("gitignore-create");
        fs::create_dir_all(&root).expect("create temp root");

        let update = ensure_root_gitignore(&root).expect("create gitignore");
        assert!(update.created);
        assert!(update.updated);

        let gitignore = fs::read_to_string(root.join(".gitignore")).expect("read gitignore");
        assert!(gitignore.contains(".rmu/"));
        assert!(gitignore.contains(".codex/"));

        cleanup(&root);
    }

    #[test]
    fn appends_managed_block_without_removing_existing_rules() {
        let existing = "target/\n.env\n";
        let (merged, updated) = merge_managed_block(existing);
        assert!(updated);
        assert!(merged.starts_with("target/\n.env\n\n"));
        assert!(merged.contains(".rmu/"));
        assert!(merged.contains(".idea/"));
    }

    #[test]
    fn replaces_existing_managed_block_in_place() {
        let existing = concat!(
            "target/\n\n",
            "# --- RMU managed ignore block: start ---\n",
            ".rmu/\n",
            "# --- RMU managed ignore block: end ---\n\n",
            ".env\n",
        );
        let (merged, updated) = merge_managed_block(existing);
        assert!(updated);
        assert_eq!(
            merged
                .matches("# --- RMU managed ignore block: start ---")
                .count(),
            1
        );
        assert!(merged.contains(".codex/"));
        assert!(merged.ends_with(".env\n"));
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should advance")
            .as_nanos();
        std::env::temp_dir().join(format!("rmu-core-{prefix}-{unique}"))
    }

    fn cleanup(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }
}
