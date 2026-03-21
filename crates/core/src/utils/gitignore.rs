use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};

mod install;
mod matcher;

pub use install::{GitignoreUpdate, ensure_root_gitignore, install_ignore_rules};
pub use matcher::ProjectIgnoreMatcher;

const MANAGED_BLOCK_START: &str = "# --- RMU managed ignore block: start ---";
const MANAGED_BLOCK_END: &str = "# --- RMU managed ignore block: end ---";
const MANAGED_PATTERNS_RESOURCE: &str = include_str!("../../resources/ignore_patterns.txt");

#[derive(Debug, Clone)]
struct GitRepoContext {
    repo_root: PathBuf,
    info_exclude_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ManagedFileUpdate {
    created: bool,
    updated: bool,
}

fn managed_patterns() -> Vec<&'static str> {
    MANAGED_PATTERNS_RESOURCE
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect()
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

fn render_managed_block(line_ending: &str) -> String {
    let patterns = managed_patterns();
    let mut lines = Vec::with_capacity(patterns.len() + 2);
    lines.push(MANAGED_BLOCK_START);
    lines.extend(patterns);
    lines.push(MANAGED_BLOCK_END);
    format!("{}{}", lines.join(line_ending), line_ending)
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

fn detect_line_ending(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

fn resolve_git_repo_context(project_root: &Path) -> Result<Option<GitRepoContext>> {
    for candidate_root in project_root.ancestors() {
        let marker = candidate_root.join(".git");
        if !marker.exists() {
            continue;
        }
        let Some(git_dir) = resolve_git_dir(&marker)? else {
            continue;
        };
        return Ok(Some(GitRepoContext {
            repo_root: candidate_root.to_path_buf(),
            info_exclude_path: git_dir.join("info").join("exclude"),
        }));
    }

    Ok(None)
}

fn resolve_git_dir(marker: &Path) -> Result<Option<PathBuf>> {
    if marker.is_dir() {
        return Ok(Some(marker.to_path_buf()));
    }
    if !marker.is_file() {
        return Ok(None);
    }

    let raw = fs::read_to_string(marker)
        .with_context(|| format!("failed to read git metadata from {}", marker.display()))?;
    for line in raw.lines() {
        let trimmed = line.trim();
        let Some(remainder) = trimmed.strip_prefix("gitdir:") else {
            continue;
        };
        let git_dir = PathBuf::from(remainder.trim());
        let resolved = if git_dir.is_absolute() {
            git_dir
        } else {
            marker
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(git_dir)
        };
        return Ok(Some(resolved));
    }

    Err(anyhow!(
        "failed to resolve git directory from {}",
        marker.display()
    ))
}
