use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow, bail};

use super::pass::filters;
use super::types::{CommitSelector, RunSelector};
use crate::engine::{Engine, storage};
use crate::index_scope::IndexScope;
use crate::model::IndexingOptions;
use crate::utils::{ProjectIgnoreMatcher, is_probably_ignored, normalize_path};

pub(crate) fn resolve_run_selector(
    engine: &Engine,
    options: &IndexingOptions,
    scope: &IndexScope,
    existing_files: &HashMap<String, storage::ExistingFileState>,
    ignore_matcher: &ProjectIgnoreMatcher,
) -> Result<RunSelector> {
    if options.changed_since.is_some() && options.changed_since_commit.is_some() {
        bail!("changed_since and changed_since_commit are mutually exclusive");
    }
    if let Some(revision) = options.changed_since_commit.as_deref() {
        return Ok(RunSelector::Commit(resolve_commit_selector(
            engine,
            revision,
            scope,
            existing_files,
            ignore_matcher,
        )?));
    }
    if let Some(changed_since) = options.changed_since {
        return Ok(RunSelector::Timestamp {
            changed_since_unix_ms: filters::offset_datetime_to_unix_ms(changed_since),
        });
    }
    Ok(RunSelector::Full)
}

fn resolve_commit_selector(
    engine: &Engine,
    revision: &str,
    scope: &IndexScope,
    existing_files: &HashMap<String, storage::ExistingFileState>,
    ignore_matcher: &ProjectIgnoreMatcher,
) -> Result<CommitSelector> {
    let revision = revision.trim();
    if revision.is_empty() {
        bail!("changed_since_commit must not be empty");
    }

    let git_root = PathBuf::from(run_git(engine, ["rev-parse", "--show-toplevel"])?);
    let resolved_merge_base_commit = run_git(engine, ["merge-base", revision, "HEAD"])?;

    let mut candidate_paths = HashSet::new();
    let mut deleted_paths = HashSet::new();

    collect_name_status_paths(
        engine,
        &git_root,
        scope,
        &mut candidate_paths,
        &mut deleted_paths,
        ignore_matcher,
        [
            "diff",
            "--name-status",
            "--no-renames",
            &format!("{resolved_merge_base_commit}...HEAD"),
            "--",
        ],
    )?;
    collect_name_status_paths(
        engine,
        &git_root,
        scope,
        &mut candidate_paths,
        &mut deleted_paths,
        ignore_matcher,
        ["diff", "--name-status", "--no-renames", "HEAD", "--"],
    )?;

    let untracked = run_git(engine, ["ls-files", "--others", "--exclude-standard"])?;
    for raw in untracked.lines() {
        if let Some(rel_text) = project_relative_from_project_output(raw) {
            if path_allowed_for_selector(scope, &rel_text, ignore_matcher) {
                candidate_paths.insert(rel_text);
            }
        }
    }

    for (path, state) in existing_files {
        if path_allowed_for_selector(scope, path, ignore_matcher)
            && !filters::is_state_complete(state)
        {
            candidate_paths.insert(path.clone());
        }
    }

    Ok(CommitSelector {
        candidate_paths,
        deleted_paths,
        resolved_merge_base_commit,
    })
}

fn collect_name_status_paths<const N: usize>(
    engine: &Engine,
    git_root: &Path,
    scope: &IndexScope,
    candidate_paths: &mut HashSet<String>,
    deleted_paths: &mut HashSet<String>,
    ignore_matcher: &ProjectIgnoreMatcher,
    args: [&str; N],
) -> Result<()> {
    let output = run_git(engine, args)?;
    for line in output.lines() {
        let Some((status, raw_path)) = line.split_once('\t') else {
            continue;
        };
        let Some(rel_text) =
            project_relative_from_git_output(git_root, &engine.project_root, raw_path)
        else {
            continue;
        };
        if !path_allowed_for_selector(scope, &rel_text, ignore_matcher) {
            continue;
        }
        if status.starts_with('D') {
            deleted_paths.insert(rel_text);
        } else {
            candidate_paths.insert(rel_text);
        }
    }
    Ok(())
}

fn project_relative_from_git_output(
    git_root: &Path,
    project_root: &Path,
    raw_path: &str,
) -> Option<String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    let absolute = git_root.join(trimmed);
    let relative = absolute.strip_prefix(project_root).ok()?;
    Some(normalize_path(relative))
}

fn project_relative_from_project_output(raw_path: &str) -> Option<String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(normalize_path(Path::new(trimmed)))
}

fn path_allowed_for_selector(
    scope: &IndexScope,
    rel_text: &str,
    ignore_matcher: &ProjectIgnoreMatcher,
) -> bool {
    scope.allows(rel_text)
        && !is_probably_ignored(Path::new(rel_text))
        && !ignore_matcher.is_ignored(Path::new(rel_text), false)
}

fn run_git<const N: usize>(engine: &Engine, args: [&str; N]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(&engine.project_root)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git in `{}`", engine.project_root.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let args_text = args.join(" ");
        if stderr.is_empty() {
            return Err(anyhow!("git {args_text} failed without stderr output"));
        }
        return Err(anyhow!("git {args_text} failed: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
