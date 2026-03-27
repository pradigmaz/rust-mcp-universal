use super::bootstrap_broad_shared::*;
use super::*;

fn search_paths(engine: &Engine, query: &str) -> Result<Vec<rmu_core::SearchHit>, Box<dyn Error>> {
    Ok(engine.search(&QueryOptions {
        query: query.to_string(),
        limit: 6,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?)
}

#[test]
fn search_generic_mod_query_balances_module_roots_and_foundational_hits()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-search-mod-balancing");
    write_bootstrap_broad_fixture(&project_dir)?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let hits = search_paths(
        &engine,
        "mod entrypoint mixins runtime hooks config network",
    )?;
    let top_paths = hits
        .iter()
        .take(6)
        .map(|hit| hit.path.as_str())
        .collect::<Vec<_>>();

    assert!(
        top_paths
            .iter()
            .any(|path| path.starts_with("mods/alpha_mod/")),
        "generic mod search should keep one module root from alpha visible: {top_paths:?}"
    );
    assert!(
        top_paths
            .iter()
            .any(|path| path.starts_with("mods/beta_mod/")),
        "generic mod search should keep one module root from beta visible: {top_paths:?}"
    );
    assert!(
        top_paths.iter().any(|path| {
            path.ends_with("BetaClient.java")
                || path.ends_with("BetaClientMixin.java")
                || path.ends_with("BetaMixinConfig.java")
                || path.ends_with("ModuleVision.java")
                || path.ends_with("AlphaMod.java")
        }),
        "generic mod search should surface at least one foundational module file: {top_paths:?}"
    );

    cleanup_project(&project_dir);
    Ok(())
}
