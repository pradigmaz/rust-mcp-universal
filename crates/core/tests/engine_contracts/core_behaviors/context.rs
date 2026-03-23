use super::*;
use rmu_core::{IndexProfile, IndexingOptions};

#[test]
fn empty_query_returns_no_hits() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;

    let hits = engine.search(&QueryOptions {
        query: "   ".to_string(),
        limit: 20,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?;
    assert!(hits.is_empty());

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn context_budget_stops_on_char_and_token_limits() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;
    let opts = QueryOptions {
        query: "alpha".to_string(),
        limit: 20,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    };

    let by_chars = engine.build_context_under_budget(&opts, 5, 10_000)?;
    assert!(by_chars.files.is_empty());
    assert!(by_chars.truncated);

    let by_tokens = engine.build_context_under_budget(&opts, 10_000, 1)?;
    assert!(by_tokens.files.is_empty());
    assert!(by_tokens.truncated);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn context_pack_design_mode_returns_docs_first() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-context-pack");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("docs"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn architecture_runtime() {}\n",
    )?;
    fs::write(
        project_dir.join("docs/design.md"),
        "Architecture overview and design decisions.\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path_with_options(&IndexingOptions {
        profile: Some(IndexProfile::DocsHeavy),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let pack = engine.build_context_pack(
        &QueryOptions {
            query: "architecture".to_string(),
            limit: 10,
            detailed: false,
            semantic: false,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        },
        rmu_core::ContextMode::Design,
        10_000,
        10_000,
    )?;
    assert_eq!(pack.mode.as_str(), "design");
    assert!(!pack.context.files.is_empty());
    assert_eq!(pack.context.files[0].path, "docs/design.md");

    cleanup_project(&project_dir);
    Ok(())
}
