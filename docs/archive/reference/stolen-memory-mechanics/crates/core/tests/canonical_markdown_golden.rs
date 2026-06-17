use std::path::{Path, PathBuf};

use obsidian_memory_core::{
    AddObservationInput, CreateNodeInput, Engine, LinkNodesInput, StorageMode, UpdateNodeInput,
};

fn scenario_root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("canonical-golden")
        .join(name)
}

fn prepare_root(name: &str) -> PathBuf {
    let root = scenario_root(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create scenario root");
    root
}

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("canonical_markdown")
}

fn normalize_note(content: &str) -> String {
    content
        .replace("\r\n", "\n")
        .lines()
        .map(|line| {
            if line.starts_with("created_at: ") {
                "created_at: <timestamp>".to_string()
            } else if line.starts_with("updated_at: ") {
                "updated_at: <timestamp>".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn assert_note_matches_fixture(root: &Path, relative: &str, fixture: &str) {
    let actual =
        std::fs::read_to_string(root.join("memory").join(relative)).expect("read generated note");
    let expected =
        std::fs::read_to_string(fixture_dir().join(fixture)).expect("read golden fixture");
    assert_eq!(
        normalize_note(&actual).trim_end().to_string(),
        expected.replace("\r\n", "\n").trim_end().to_string()
    );
}

fn create_project(engine: &Engine, title: &str, summary: &str) {
    let created = engine
        .create_node(CreateNodeInput {
            node_type: "Project".to_string(),
            title: title.to_string(),
            slug: Some("_index".to_string()),
            status: Some("active".to_string()),
            summary: Some(summary.to_string()),
            tags: vec!["memory".to_string()],
            aliases: vec!["MCP Home".to_string()],
        })
        .expect("create project");
    assert_eq!(created.node.slug, "_index");
}

#[test]
fn freshly_created_project_note_matches_golden_fixture() {
    let root = prepare_root("fresh-project");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

    create_project(&engine, "Workspace", "Canonical summary.");

    assert_note_matches_fixture(&root, "Workspace.md", "project-create.md");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn add_observation_updates_full_note_shape() {
    let root = prepare_root("project-observation");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

    create_project(&engine, "Workspace", "Canonical summary.");
    let observation = engine
        .add_observation(AddObservationInput {
            node: "_index".to_string(),
            content: "Smoke observation".to_string(),
        })
        .expect("add observation");
    assert!(observation.added);

    assert_note_matches_fixture(&root, "Workspace.md", "project-add-observation.md");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn link_and_unlink_update_canonical_markdown_golden() {
    let root = prepare_root("project-relations");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

    create_project(&engine, "Workspace", "Canonical summary.");
    engine
        .create_node(CreateNodeInput {
            node_type: "Module".to_string(),
            title: "Memory Module".to_string(),
            slug: Some("memory-module".to_string()),
            status: Some("active".to_string()),
            summary: Some("Module summary.".to_string()),
            tags: Vec::new(),
            aliases: Vec::new(),
        })
        .expect("create module");

    let linked = engine
        .link_nodes(LinkNodesInput {
            source: "_index".to_string(),
            target: "memory-module".to_string(),
            relation_kind: "depends_on".to_string(),
        })
        .expect("link nodes");
    assert!(linked.changed);
    assert_note_matches_fixture(&root, "Workspace.md", "project-link.md");

    let unlinked = engine
        .unlink_nodes(LinkNodesInput {
            source: "_index".to_string(),
            target: "memory-module".to_string(),
            relation_kind: "depends_on".to_string(),
        })
        .expect("unlink nodes");
    assert!(unlinked.changed);
    assert_note_matches_fixture(&root, "Workspace.md", "project-unlink.md");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn update_node_metadata_matches_golden_fixture() {
    let root = prepare_root("project-update");
    let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

    create_project(&engine, "Workspace", "Canonical summary.");
    let updated = engine
        .update_node(UpdateNodeInput {
            node: "_index".to_string(),
            title: Some("Workspace Control Plane".to_string()),
            status: Some("accepted".to_string()),
            summary: Some("Updated canonical summary.".to_string()),
            tags: Some(vec!["memory".to_string(), "control-plane".to_string()]),
            aliases: Some(vec!["MCP Home".to_string(), "Workspace Hub".to_string()]),
        })
        .expect("update node");
    assert!(updated.changed);

    assert_note_matches_fixture(
        &root,
        "Workspace Control Plane (Accepted).md",
        "project-update.md",
    );
    let _ = std::fs::remove_dir_all(root);
}
