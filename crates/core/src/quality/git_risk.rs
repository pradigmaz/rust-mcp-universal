use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};

use crate::quality::{GitRiskFacts, GitRiskPolicy};
use crate::utils::normalize_path;

#[derive(Debug, Default)]
struct GitRiskAccumulator {
    commits: BTreeSet<String>,
    authors: BTreeSet<String>,
    cochange_neighbors: BTreeSet<String>,
    recent_churn_lines: i64,
    author_touch_counts: HashMap<String, i64>,
}

pub(crate) fn load_git_risk_facts(
    project_root: &Path,
    active_paths: &HashSet<String>,
    policy: &GitRiskPolicy,
) -> Result<HashMap<String, GitRiskFacts>> {
    let mut facts = active_paths
        .iter()
        .cloned()
        .map(|path| (path, GitRiskFacts::default()))
        .collect::<HashMap<_, _>>();
    if !policy.enabled || active_paths.is_empty() || !is_git_repo(project_root)? {
        return Ok(facts);
    }

    let log_output = run_git(
        project_root,
        &[
            "log",
            "--since",
            &format!("{} days ago", policy.recent_days),
            "--format=__RMU_COMMIT__\t%H\t%an",
            "--numstat",
            "--",
            ".",
        ],
    )?;

    let mut accumulators = active_paths
        .iter()
        .cloned()
        .map(|path| (path, GitRiskAccumulator::default()))
        .collect::<HashMap<_, _>>();
    let mut current_commit = None::<String>;
    let mut current_author = None::<String>;
    let mut commit_active_paths = Vec::<String>::new();
    let mut commit_all_paths = Vec::<String>::new();

    let flush_commit = |accumulators: &mut HashMap<String, GitRiskAccumulator>,
                        commit_active_paths: &mut Vec<String>,
                        commit_all_paths: &mut Vec<String>| {
        if commit_active_paths.is_empty() || commit_all_paths.is_empty() {
            commit_active_paths.clear();
            commit_all_paths.clear();
            return;
        }
        commit_all_paths.sort();
        commit_all_paths.dedup();
        commit_active_paths.sort();
        commit_active_paths.dedup();
        for active_path in commit_active_paths.iter() {
            if let Some(entry) = accumulators.get_mut(active_path) {
                for neighbor in commit_all_paths.iter() {
                    if neighbor != active_path {
                        entry.cochange_neighbors.insert(neighbor.clone());
                    }
                }
            }
        }
        commit_active_paths.clear();
        commit_all_paths.clear();
    };

    for raw_line in log_output.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(payload) = line.strip_prefix("__RMU_COMMIT__\t") {
            flush_commit(
                &mut accumulators,
                &mut commit_active_paths,
                &mut commit_all_paths,
            );
            let mut parts = payload.splitn(2, '\t');
            current_commit = parts.next().map(ToOwned::to_owned);
            current_author = parts.next().map(ToOwned::to_owned);
            continue;
        }
        let Some(commit_id) = current_commit.as_ref() else {
            continue;
        };
        let Some(author) = current_author.as_ref() else {
            continue;
        };
        let mut parts = line.splitn(3, '\t');
        let added = parts.next().unwrap_or_default();
        let deleted = parts.next().unwrap_or_default();
        let raw_path = parts.next().unwrap_or_default();
        if raw_path.is_empty() {
            continue;
        }
        let normalized = normalize_path(Path::new(raw_path));
        commit_all_paths.push(normalized.clone());
        if let Some(entry) = accumulators.get_mut(&normalized) {
            commit_active_paths.push(normalized);
            entry.commits.insert(commit_id.clone());
            entry.authors.insert(author.clone());
            *entry.author_touch_counts.entry(author.clone()).or_default() += 1;
            entry.recent_churn_lines = entry
                .recent_churn_lines
                .saturating_add(parse_numstat_value(added))
                .saturating_add(parse_numstat_value(deleted));
        }
    }
    flush_commit(
        &mut accumulators,
        &mut commit_active_paths,
        &mut commit_all_paths,
    );

    for (path, accumulator) in accumulators {
        let commit_count = i64::try_from(accumulator.commits.len()).unwrap_or(i64::MAX);
        let author_count = i64::try_from(accumulator.authors.len()).unwrap_or(i64::MAX);
        let primary_touch_count = accumulator
            .author_touch_counts
            .values()
            .copied()
            .max()
            .unwrap_or_default();
        let primary_author_share_bps = if commit_count >= policy.min_commits_for_ownership {
            primary_touch_count.saturating_mul(10_000) / commit_count.max(1)
        } else {
            0
        };
        facts.insert(
            path,
            GitRiskFacts {
                recent_commit_count: commit_count,
                recent_author_count: author_count,
                recent_churn_lines: accumulator.recent_churn_lines,
                primary_author_share_bps,
                cochange_neighbor_count: i64::try_from(accumulator.cochange_neighbors.len())
                    .unwrap_or(i64::MAX),
            },
        );
    }

    Ok(facts)
}

fn parse_numstat_value(raw: &str) -> i64 {
    raw.parse::<i64>().unwrap_or(0)
}

fn is_git_repo(project_root: &Path) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .with_context(|| format!("failed to run git in `{}`", project_root.display()))?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim() == "true");
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_ascii_lowercase();
    if stderr.contains("not a git repository") {
        return Ok(false);
    }
    Err(anyhow!(
        "git rev-parse --is-inside-work-tree failed: {}",
        stderr.trim()
    ))
}

fn run_git(project_root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(args)
        .output()
        .with_context(|| format!("failed to run git in `{}`", project_root.display()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let args_text = args.join(" ");
        if stderr.is_empty() {
            return Err(anyhow!("git {args_text} failed without stderr output"));
        }
        return Err(anyhow!("git {args_text} failed: {stderr}"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::load_git_risk_facts;
    use crate::quality::GitRiskPolicy;
    use std::collections::HashSet;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    fn git(project_root: &PathBuf, args: &[&str]) {
        let status = Command::new("git")
            .arg("-C")
            .arg(project_root)
            .args(args)
            .status()
            .expect("git command should run");
        assert!(status.success(), "git {:?} should succeed", args);
    }

    #[test]
    fn git_risk_collects_recent_counts_and_ownership() {
        let root = temp_dir("rmu-git-risk");
        fs::create_dir_all(root.join("src")).expect("create temp repo");
        git(&root, &["init"]);
        git(&root, &["config", "user.email", "test@example.com"]);
        git(&root, &["config", "user.name", "Test User"]);

        fs::write(root.join("src/lib.rs"), "pub fn alpha() -> i32 { 1 }\n").expect("write");
        git(&root, &["add", "."]);
        git(
            &root,
            &[
                "-c",
                "user.name=Alice",
                "-c",
                "user.email=alice@example.com",
                "commit",
                "-m",
                "first",
            ],
        );

        fs::write(
            root.join("src/lib.rs"),
            "pub fn alpha() -> i32 { 2 }\npub fn beta() -> i32 { 3 }\n",
        )
        .expect("rewrite");
        fs::write(root.join("src/other.rs"), "pub fn other() -> i32 { 4 }\n").expect("write");
        git(&root, &["add", "."]);
        git(
            &root,
            &[
                "-c",
                "user.name=Bob",
                "-c",
                "user.email=bob@example.com",
                "commit",
                "-m",
                "second",
            ],
        );

        let active_paths = HashSet::from(["src/lib.rs".to_string()]);
        let facts = load_git_risk_facts(
            &root,
            &active_paths,
            &GitRiskPolicy {
                min_commits_for_ownership: 2,
                ..GitRiskPolicy::default()
            },
        )
        .expect("git risk should load");
        let lib = facts.get("src/lib.rs").expect("lib facts");
        assert_eq!(lib.recent_commit_count, 2);
        assert_eq!(lib.recent_author_count, 2);
        assert!(lib.recent_churn_lines >= 3);
        assert_eq!(lib.primary_author_share_bps, 5_000);
        assert!(lib.cochange_neighbor_count >= 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn non_git_repositories_return_empty_facts() {
        let root = temp_dir("rmu-git-risk-no-repo");
        fs::create_dir_all(root.join("src")).expect("create temp dir");
        let active_paths = HashSet::from(["src/lib.rs".to_string()]);
        let facts = load_git_risk_facts(&root, &active_paths, &GitRiskPolicy::default())
            .expect("non git repo should not fail");
        assert_eq!(
            facts
                .get("src/lib.rs")
                .expect("lib facts")
                .recent_commit_count,
            0
        );
        let _ = fs::remove_dir_all(root);
    }
}
