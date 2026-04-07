use super::*;

#[test]
fn search_prefers_exact_handler_file_over_umbrella_dispatcher_for_handler_queries()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-search-handler-ranking");
    fs::create_dir_all(project_dir.join("src/rpc_tools/handlers"))?;
    fs::write(
        project_dir.join("src/rpc_tools/handlers.rs"),
        r#"
#[path = "handlers/symbol_lookup.rs"]
mod symbol_lookup;
#[path = "handlers/symbol_references.rs"]
mod symbol_references;
#[path = "handlers/related_files.rs"]
mod related_files;

pub fn symbol_lookup() {
    symbol_lookup::symbol_lookup();
}

pub fn symbol_references() {
    symbol_references::symbol_references();
}

pub fn related_files() {
    related_files::related_files();
}
"#,
    )?;
    fs::write(
        project_dir.join("src/rpc_tools/handlers/symbol_lookup.rs"),
        r#"
pub fn symbol_lookup() {
    let implementation = "direct symbol lookup implementation";
    println!("{implementation}");
}
"#,
    )?;
    fs::write(
        project_dir.join("src/rpc_tools/handlers/symbol_references.rs"),
        r#"
pub fn symbol_references() {
    let implementation = "direct symbol references implementation";
    println!("{implementation}");
}
"#,
    )?;
    fs::write(
        project_dir.join("src/rpc_tools/handlers/related_files.rs"),
        r#"
pub fn related_files() {
    let implementation = "direct related files implementation";
    println!("{implementation}");
}
"#,
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    for (query, expected_path) in [
        ("symbol_lookup", "src/rpc_tools/handlers/symbol_lookup.rs"),
        (
            "symbol_references",
            "src/rpc_tools/handlers/symbol_references.rs",
        ),
        ("related_files", "src/rpc_tools/handlers/related_files.rs"),
    ] {
        let hits = engine.search(&QueryOptions {
            query: query.to_string(),
            limit: 5,
            detailed: false,
            semantic: false,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
            agent_intent_mode: None,
        })?;
        assert_eq!(
            hits.first().map(|hit| hit.path.as_str()),
            Some(expected_path),
            "expected `{query}` to prefer exact handler file"
        );
    }

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn search_surfaces_graph_connected_file_when_anchor_only_exists_in_seed()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-search-graph-stage");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        r#"
mod worker;

pub fn anchor_entry() {
    let note = "graph_connected_anchor";
    worker::render_worker();
    println!("{note}");
}
"#,
    )?;
    fs::write(
        project_dir.join("src/worker.rs"),
        r#"
pub fn render_worker() {
    println!("worker implementation only");
}
"#,
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let hits = engine.search(&QueryOptions {
        query: "graph_connected_anchor".to_string(),
        limit: 5,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
        agent_intent_mode: None,
    })?;

    assert_eq!(
        hits.first().map(|hit| hit.path.as_str()),
        Some("src/main.rs")
    );
    assert!(
        hits.iter().any(|hit| hit.path == "src/worker.rs"),
        "expected graph-stage to surface connected worker file, got {:?}",
        hits.iter().map(|hit| hit.path.as_str()).collect::<Vec<_>>()
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn search_plain_tests_query_prefers_real_test_files_over_non_test_mentions()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-search-tests-surface");
    fs::create_dir_all(project_dir.join("src/world/generation/runtime"))?;
    fs::create_dir_all(project_dir.join("src/world/generation"))?;

    fs::write(
        project_dir.join("src/world/generation/runtime/GenerateChunk.test.ts"),
        r#"
import { describe, it, expect } from "vitest";

describe("GenerateChunk", () => {
    it("keeps runtime tests visible", () => {
        expect("tests").toBe("tests");
    });
});
"#,
    )?;
    fs::write(
        project_dir.join("src/world/generation/ChunkGenerator.test.ts"),
        r#"
import { describe, it, expect } from "vitest";

describe("ChunkGenerator", () => {
    it("keeps generator tests visible", () => {
        expect("tests").toBe("tests");
    });
});
"#,
    )?;
    fs::write(
        project_dir.join("src/world/generation/StructureGenerator.ts"),
        r#"
/**
 * Legacy structure generator used only by legacy decorators/tests.
 */
export class StructureGenerator {}
"#,
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let hits = engine.search(&QueryOptions {
        query: "tests".to_string(),
        limit: 5,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
        agent_intent_mode: None,
    })?;

    let top_paths = hits
        .iter()
        .take(3)
        .map(|hit| hit.path.as_str())
        .collect::<Vec<_>>();
    assert!(
        top_paths
            .iter()
            .take(2)
            .all(|path| path.ends_with(".test.ts") || path.ends_with(".test.tsx")),
        "plain tests query should put real test files ahead of non-test mentions: {top_paths:?}"
    );

    cleanup_project(&project_dir);
    Ok(())
}
