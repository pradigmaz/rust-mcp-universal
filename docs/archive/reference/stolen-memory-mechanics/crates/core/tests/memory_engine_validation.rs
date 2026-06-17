use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use obsidian_memory_core::{
    AddObservationInput, CreateNodeInput, Engine, LinkNodesInput, StorageMode, UpdateNodeInput,
    as_state_failure,
};

fn temp_root(prefix: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("obsidian-memory-validation-{prefix}-{suffix}"));
    std::fs::create_dir_all(&root).expect("create temp root");
    root
}

fn write_note(root: &Path, relative: &str, title: &str, node_type: &str, body: &str) {
    let path = root.join("memory").join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent");
    }
    let normalized_type = node_type
        .trim()
        .replace(['-', ' '], "_")
        .to_ascii_lowercase();
    let slug = if relative == "_index.md" {
        "_index".to_string()
    } else {
        Path::new(relative)
            .file_stem()
            .and_then(|value| value.to_str())
            .expect("slug")
            .to_string()
    };
    let content = format!(
        "---\nid: {normalized_type}-{slug}\nslug: {slug}\ntype: {node_type}\ntitle: {title}\nstatus: active\nproject: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# {title}\n\n## Summary\n{body}\n\n## Observations\n\n## Relations\n\n## References\n"
    );
    std::fs::write(path, content).expect("write note");
}

fn create_node(engine: &Engine, node_type: &str, title: &str, slug: &str, summary: &str) {
    engine
        .create_node(CreateNodeInput {
            node_type: node_type.to_string(),
            title: title.to_string(),
            slug: Some(slug.to_string()),
            status: Some("active".to_string()),
            summary: Some(summary.to_string()),
            tags: Vec::new(),
            aliases: Vec::new(),
        })
        .expect("create node");
}

#[test]
fn parser_rebuild_reports_malformed_notes_and_keeps_valid_ones() {
    let root = temp_root("parser");
    write_note(
        &root,
        "_index.md",
        "Workspace",
        "Project",
        "Project summary.",
    );
    write_note(
        &root,
        "decisions/auth.md",
        "Auth Decision",
        "Decision",
        "Auth summary.",
    );
    std::fs::create_dir_all(root.join("memory").join("risks")).expect("risks dir");
    std::fs::write(
        root.join("memory").join("risks/nested.md"),
        "---\nid: risk-nested\ntype: Risk\ntitle: Nested Risk\nstatus: active\nproject:\n  name: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# Nested Risk\n\n## Summary\nBroken.\n\n## Observations\n\n## Relations\n\n## References\n",
    )
    .expect("write nested note");
    std::fs::create_dir_all(root.join("memory").join("modules")).expect("modules dir");
    std::fs::write(
        root.join("memory").join("modules/missing-section.md"),
        "---\nid: module-missing-section\ntype: Module\ntitle: Missing Section\nstatus: active\nproject: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# Missing Section\n\n## Summary\nBroken.\n\n## Observations\n\n## References\n",
    )
    .expect("write missing section note");

    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
    let rebuilt = engine.rebuild_index().expect("rebuild");

    assert_eq!(rebuilt.indexed_files, 2);
    assert_eq!(rebuilt.errors.len(), 2);
    assert!(rebuilt.errors.iter().any(|err| err.contains("nested.md")));
    assert!(
        rebuilt
            .errors
            .iter()
            .any(|err| err.contains("missing-section.md"))
    );

    let hits = engine.search_memory("Auth", 10).expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].slug, "auth");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn rebuild_is_idempotent_and_tracks_updates_and_deletes() {
    let root = temp_root("rebuild");
    write_note(
        &root,
        "_index.md",
        "Workspace",
        "Project",
        "Project summary.",
    );
    write_note(
        &root,
        "decisions/auth.md",
        "Auth Decision",
        "Decision",
        "Auth summary.",
    );

    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
    let first = engine.rebuild_index().expect("first rebuild");
    let second = engine.rebuild_index().expect("second rebuild");
    assert_eq!(first.counts.notes, second.counts.notes);
    assert_eq!(first.counts.relations, second.counts.relations);
    assert!(second.errors.is_empty());

    write_note(
        &root,
        "decisions/auth.md",
        "Auth Decision",
        "Decision",
        "Updated summary.",
    );
    engine.rebuild_index().expect("rebuild after update");
    let opened = engine
        .open_nodes(&["auth".to_string()])
        .expect("open after update");
    assert_eq!(opened[0].summary, "Updated summary.");

    std::fs::remove_file(root.join("memory").join("decisions/auth.md")).expect("remove note");
    let after_delete = engine.rebuild_index().expect("rebuild after delete");
    assert_eq!(after_delete.counts.notes, 1);
    assert!(
        engine
            .open_nodes(&["auth".to_string()])
            .expect("open")
            .is_empty()
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn read_paths_and_obsidian_smoke_cover_generated_markdown() {
    let root = temp_root("smoke");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
    create_node(
        &engine,
        "Project",
        "Workspace",
        "_index",
        "Project summary.",
    );
    create_node(
        &engine,
        "Module",
        "Auth Module",
        "auth-module",
        "Auth module summary.",
    );
    engine
        .update_node(UpdateNodeInput {
            node: "auth-module".to_string(),
            title: None,
            status: None,
            summary: None,
            tags: None,
            aliases: Some(vec!["Authentication".to_string()]),
        })
        .expect("add module alias");
    create_node(
        &engine,
        "Risk",
        "Auth Risk",
        "auth-risk",
        "Auth risk summary.",
    );

    let observation = engine
        .add_observation(AddObservationInput {
            node: "Auth Risk".to_string(),
            content: "Watch token rotation".to_string(),
        })
        .expect("add observation");
    assert!(observation.added);

    let relation = engine
        .link_nodes(LinkNodesInput {
            source: "Auth Risk".to_string(),
            target: "Authentication".to_string(),
            relation_kind: "affects".to_string(),
        })
        .expect("link nodes");
    assert!(relation.changed);

    let hits = engine.search_memory("Auth", 10).expect("search");
    assert!(hits.iter().any(|hit| hit.slug == "auth-module"));
    let slug_hits = engine
        .search_memory("auth-module", 10)
        .expect("hyphen slug search");
    assert!(slug_hits.iter().any(|hit| hit.slug == "auth-module"));

    let opened = engine
        .open_nodes(&["Auth Risk".to_string(), "Authentication".to_string()])
        .expect("open by title and alias");
    assert_eq!(opened.len(), 2);
    let risk = opened
        .iter()
        .find(|node| node.slug == "auth-risk")
        .expect("risk opened by title");
    assert_eq!(risk.observations, vec!["Watch token rotation".to_string()]);
    assert!(risk.relations.iter().any(
        |relation| relation.relation_kind == "affects" && relation.target_slug == "auth-module"
    ));
    assert!(opened.iter().any(|node| node.slug == "auth-module"
        && node.aliases.iter().any(|alias| alias == "Authentication")));

    let graph = engine
        .read_graph(&["Auth Risk".to_string()])
        .expect("read graph by title");
    assert!(graph.nodes.iter().any(|node| node.slug == "auth-module"));
    assert!(
        graph
            .relations
            .iter()
            .any(|relation| relation.relation_kind == "affects"
                && relation.source_slug == "auth-risk"
                && relation.target_slug == "auth-module")
    );

    let rendered = std::fs::read_to_string(root.join("memory").join("risks/Auth Risk.md"))
        .expect("read risk file");
    let expected_project = format!(
        "project: {}\n",
        root.file_name()
            .and_then(|value| value.to_str())
            .expect("project name")
    );
    assert!(rendered.starts_with("---\n"));
    assert!(rendered.contains("type: Risk\n"));
    assert!(rendered.contains(&expected_project));
    assert!(rendered.contains("- Watch token rotation\n"));
    assert!(rendered.contains("- affects [[auth-module|Auth Module]]\n"));
    assert!(rendered.contains("- documents [[_index|Workspace]]\n"));
    assert!(!rendered.contains("project:\n  "));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn missing_and_stale_index_fail_with_recovery_codes() {
    let root = temp_root("recovery");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

    let missing = engine
        .ensure_index_ready(false)
        .expect_err("missing index should fail");
    assert_eq!(
        as_state_failure(&missing).expect("state failure").code,
        "E_REBUILD_REQUIRED"
    );

    write_note(
        &root,
        "_index.md",
        "Workspace",
        "Project",
        "Project summary.",
    );
    engine.rebuild_index().expect("rebuild");
    write_note(
        &root,
        "_index.md",
        "Workspace",
        "Project",
        "Updated summary.",
    );

    let stale = engine
        .ensure_index_ready(false)
        .expect_err("stale index should fail");
    assert_eq!(
        as_state_failure(&stale).expect("state failure").code,
        "E_STALE_INDEX"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn duplicate_policy_blocks_same_type_title_but_not_cross_type_matches() {
    let root = temp_root("duplicate-policy");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

    create_node(
        &engine,
        "Task",
        "Shared Name",
        "shared-task",
        "Task summary.",
    );
    create_node(
        &engine,
        "Module",
        "Shared Name",
        "shared-module",
        "Module summary.",
    );

    let duplicate = engine.create_node(CreateNodeInput {
        node_type: "Task".to_string(),
        title: "Shared Name".to_string(),
        slug: Some("shared-task-v2".to_string()),
        status: Some("active".to_string()),
        summary: Some("Replacement".to_string()),
        tags: Vec::new(),
        aliases: Vec::new(),
    });
    let failure = duplicate.expect_err("duplicate task");
    assert_eq!(failure.code, "E_DUPLICATE_CANDIDATE");
    assert_eq!(
        failure.details["candidates"]
            .as_array()
            .expect("candidates")
            .len(),
        1
    );
    assert_eq!(failure.details["candidates"][0]["slug"], "shared-task");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn read_resolver_reports_ambiguous_title_or_alias_with_stable_code() {
    let root = temp_root("read-ambiguous");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

    create_node(
        &engine,
        "Task",
        "Shared Name",
        "shared-task",
        "Task summary.",
    );
    create_node(
        &engine,
        "Module",
        "Shared Name",
        "shared-module",
        "Module summary.",
    );

    let err = engine
        .open_nodes(&["Shared Name".to_string()])
        .expect_err("ambiguous read target");
    let failure = as_state_failure(&err).expect("state failure");
    assert_eq!(failure.code, "E_AMBIGUOUS_TARGET");
    assert_eq!(failure.details["node"], "Shared Name");
    assert_eq!(failure.details["column"], "title_or_alias");

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn create_node_ids_are_slug_based_and_collision_safe() {
    let root = temp_root("slug-ids");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

    let first = engine
        .create_node(CreateNodeInput {
            node_type: "Task".to_string(),
            title: "First Task".to_string(),
            slug: Some("first-task".to_string()),
            status: Some("active".to_string()),
            summary: Some("First summary.".to_string()),
            tags: Vec::new(),
            aliases: Vec::new(),
        })
        .expect("first task");
    let second = engine
        .create_node(CreateNodeInput {
            node_type: "Task".to_string(),
            title: "Second Task".to_string(),
            slug: Some("second-task".to_string()),
            status: Some("active".to_string()),
            summary: Some("Second summary.".to_string()),
            tags: Vec::new(),
            aliases: Vec::new(),
        })
        .expect("second task");

    assert_eq!(first.node.id, "task-first-task");
    assert_eq!(second.node.id, "task-second-task");

    let _ = std::fs::remove_dir_all(root);
}
