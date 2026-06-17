use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::engine::Engine;
use crate::model::StorageMode;
use crate::model::{AddObservationInput, CreateNodeInput, LinkNodesInput};

fn temp_root(prefix: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("obsidian-memory-{prefix}-{suffix}"));
    std::fs::create_dir_all(&root).expect("create temp root");
    root
}

fn create_node(
    engine: &Engine,
    node_type: &str,
    title: &str,
    slug: &str,
    status: &str,
    summary: &str,
) {
    engine
        .create_node(CreateNodeInput {
            node_type: node_type.to_string(),
            title: title.to_string(),
            slug: Some(slug.to_string()),
            status: Some(status.to_string()),
            summary: Some(summary.to_string()),
            tags: Vec::new(),
            aliases: Vec::new(),
        })
        .expect("create node");
}

#[test]
fn project_brief_prefers_project_summary_and_filters_closed_risks() {
    let root = temp_root("brief");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
    create_node(
        &engine,
        "Project",
        "Workspace",
        "_index",
        "active",
        "Canonical project summary",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Decision",
        "Auth Decision",
        "auth-decision",
        "active",
        "Decision summary",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Decision",
        "Superseded Decision",
        "superseded-decision",
        "superseded",
        "Historical decision",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Risk",
        "Open Risk",
        "open-risk",
        "active",
        "Open risk summary",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Constraint",
        "Unknown Constraint",
        "unknown-constraint",
        "watching",
        "Unknown risk state",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Risk",
        "Closed Risk",
        "closed-risk",
        "resolved",
        "Should not appear",
    );

    let brief = engine.project_brief().expect("project brief");

    assert_eq!(brief.summary, "Canonical project summary");
    assert_eq!(brief.top_decisions[0].slug, "auth-decision");
    assert!(
        brief
            .top_decisions
            .iter()
            .all(|node| node.slug != "superseded-decision")
    );
    assert!(
        brief
            .top_risks
            .iter()
            .all(|node| node.slug != "closed-risk")
    );
    assert!(
        brief
            .top_risks
            .iter()
            .any(|node| node.normalized_status.map(|value| value.as_str()) == Some("unknown"))
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn recent_changes_follow_file_mtime_desc() {
    let root = temp_root("recent");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
    create_node(
        &engine,
        "Task",
        "Older Task",
        "older-task",
        "active",
        "Older summary",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "ProgressEntry",
        "Latest Progress",
        "latest-progress",
        "active",
        "Latest summary",
    );

    let changes = engine.recent_changes(5).expect("recent changes");

    assert_eq!(changes[0].slug, "latest-progress");
    assert_eq!(changes[0].change_hint, "Latest summary");
    assert!(changes.iter().all(|node| node.node_type != "section_hub"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn decision_log_uses_fts_and_like_fallback() {
    let root = temp_root("decision-log");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
    create_node(
        &engine,
        "Decision",
        "Auth Rollout",
        "auth-rollout",
        "active",
        "Handle auth(issue) carefully",
    );
    engine
        .add_observation(AddObservationInput {
            node: "auth-rollout".to_string(),
            content: "Rollout window approved".to_string(),
        })
        .expect("add observation");
    create_node(
        &engine,
        "Decision",
        "Legacy Auth Rollout",
        "legacy-auth-rollout",
        "accepted",
        "Historical auth decision",
    );

    let by_fts = engine
        .decision_log(Some("rollout"), 5)
        .expect("fts decision log");
    let by_like = engine
        .decision_log(Some("auth("), 5)
        .expect("like fallback decision log");

    assert_eq!(by_fts[0].slug, "auth-rollout");
    assert_eq!(by_like[0].slug, "auth-rollout");
    assert!(
        by_fts
            .iter()
            .all(|entry| entry.slug != "legacy-auth-rollout")
    );
    assert!(
        by_like
            .iter()
            .all(|entry| entry.slug != "legacy-auth-rollout")
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn risk_hotspots_prioritize_blockers_and_preserve_unknown_status() {
    let root = temp_root("hotspots");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
    create_node(
        &engine,
        "Task",
        "Blocked Task",
        "blocked-task",
        "active",
        "Task summary",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Risk",
        "Blocking Risk",
        "blocking-risk",
        "watching",
        "Blocks work",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Constraint",
        "Affecting Constraint",
        "affecting-constraint",
        "active",
        "Affects work",
    );
    engine
        .link_nodes(LinkNodesInput {
            source: "blocking-risk".to_string(),
            target: "blocked-task".to_string(),
            relation_kind: "blocks".to_string(),
        })
        .expect("link blocks");
    engine
        .link_nodes(LinkNodesInput {
            source: "affecting-constraint".to_string(),
            target: "blocked-task".to_string(),
            relation_kind: "affects".to_string(),
        })
        .expect("link affects");

    let hotspots = engine.risk_hotspots(5).expect("risk hotspots");

    assert_eq!(hotspots.risks[0].slug, "blocking-risk");
    assert_eq!(hotspots.risks[0].blocks, vec!["blocked-task".to_string()]);
    assert_eq!(hotspots.risks[0].normalized_status.as_str(), "unknown");
    assert_eq!(
        hotspots.constraints[0].affects,
        vec!["blocked-task".to_string()]
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn context_pack_is_deterministic_and_marks_truncation() {
    let root = temp_root("context-pack");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
    create_node(
        &engine,
        "Project",
        "Workspace",
        "_index",
        "active",
        "Context brief",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Module",
        "Auth Module",
        "auth-module",
        "active",
        "Auth module summary",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Decision",
        "Auth Decision",
        "auth-decision",
        "active",
        "Auth decision summary",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Decision",
        "Auth Decision Old",
        "auth-decision-old",
        "superseded",
        "Historical auth decision summary",
    );
    std::thread::sleep(Duration::from_millis(5));
    create_node(
        &engine,
        "Risk",
        "Auth Risk",
        "auth-risk",
        "active",
        "Auth risk summary",
    );
    engine
        .link_nodes(LinkNodesInput {
            source: "auth-risk".to_string(),
            target: "auth-module".to_string(),
            relation_kind: "affects".to_string(),
        })
        .expect("link risk");

    let first = engine
        .context_pack("auth", 8, 10_000, 2_500)
        .expect("first pack");
    let second = engine
        .context_pack("auth", 8, 10_000, 2_500)
        .expect("second pack");
    let truncated = engine
        .context_pack("auth", 8, 450, 120)
        .expect("truncated pack");

    assert_eq!(first.seed, "auth");
    assert_eq!(first.included_nodes, second.included_nodes);
    assert!(!first.included_nodes.is_empty());
    assert!(
        first
            .included_nodes
            .iter()
            .all(|node| node.slug != "auth-decision-old")
    );
    assert!(truncated.budget.truncated);
    assert!(truncated.budget.used_chars <= truncated.budget.max_chars);

    let _ = std::fs::remove_dir_all(root);
}
