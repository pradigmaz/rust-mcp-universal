use super::bootstrap_broad_shared::*;
use super::*;

fn run_mod_query(semantic: bool) -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-agent-bootstrap-mod-roots");
    write_bootstrap_broad_fixture(&project_dir)?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let payload = engine.agent_bootstrap_with_auto_index(
        Some("mod entrypoint mixins runtime hooks config network"),
        6,
        semantic,
        12_000,
        3_000,
        true,
    )?;

    let bundle = payload.query_bundle.expect("query bundle");
    let top_paths = bundle
        .hits
        .iter()
        .take(6)
        .map(|hit| hit.path.as_str())
        .collect::<Vec<_>>();

    assert!(
        top_paths
            .iter()
            .any(|path| path.starts_with("mods/alpha_mod/"))
    );
    assert!(
        top_paths
            .iter()
            .any(|path| path.starts_with("mods/beta_mod/"))
    );
    assert!(
        top_paths.iter().any(|path| {
            path.ends_with("BetaClient.java")
                || path.ends_with("BetaClientMixin.java")
                || path.ends_with("BetaMixinConfig.java")
                || path.ends_with("ModuleVision.java")
                || path.ends_with("AlphaMod.java")
        }),
        "generic mod query should surface at least one foundational module file, not only runtime/network handlers: {top_paths:?}"
    );
    assert!(
        top_paths
            .iter()
            .take(4)
            .filter(|path| path.starts_with("mods/beta_mod/"))
            .count()
            >= 1,
        "generic mod query should keep an alternate module root inside the top window: {top_paths:?}"
    );
    assert!(
        bundle
            .followups
            .iter()
            .any(|item| item.contains("module roots")),
        "generic mod query should emit module-oriented next steps: {:?}",
        bundle.followups
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn agent_bootstrap_generic_mod_query_spans_multiple_module_roots() -> Result<(), Box<dyn Error>> {
    run_mod_query(true)
}

#[test]
fn agent_bootstrap_generic_mod_query_rebalances_lexical_results() -> Result<(), Box<dyn Error>> {
    run_mod_query(false)
}
