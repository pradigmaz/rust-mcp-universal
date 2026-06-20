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
