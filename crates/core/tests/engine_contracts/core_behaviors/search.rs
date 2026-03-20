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
