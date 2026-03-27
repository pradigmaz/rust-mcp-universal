use super::bootstrap_broad_shared::*;
use super::*;

#[test]
fn agent_bootstrap_broad_query_spans_multiple_layers_and_emits_followups()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-agent-bootstrap-broad");
    write_bootstrap_broad_fixture(&project_dir)?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let payload = engine.agent_bootstrap_with_auto_index(
        Some("frontend backend auth entrypoints tests architecture"),
        5,
        true,
        12_000,
        3_000,
        true,
    )?;

    let bundle = payload.query_bundle.expect("query bundle");
    let top_paths = bundle
        .hits
        .iter()
        .take(4)
        .map(|hit| hit.path.as_str())
        .collect::<Vec<_>>();

    assert!(top_paths.iter().any(|path| path.starts_with("backend/")));
    assert!(top_paths.iter().any(|path| path.starts_with("frontend/")));
    assert!(
        !top_paths
            .iter()
            .any(|path| path == &DOC_PATH || path == &AI_ARTIFACT_PATH),
        "support/docs artifacts should stay below code-bearing layers: {top_paths:?}"
    );
    assert!(
        !bundle.followups.is_empty(),
        "broad bootstrap query should emit next-step guidance"
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn agent_bootstrap_auth_tests_query_keeps_test_surface_visible() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-agent-bootstrap-auth-tests");
    write_bootstrap_broad_fixture(&project_dir)?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let payload = engine.agent_bootstrap_with_auto_index(
        Some("auth boundary tests nearby backend frontend"),
        6,
        true,
        12_000,
        3_000,
        true,
    )?;

    let bundle = payload.query_bundle.expect("query bundle");
    let top_paths = bundle
        .hits
        .iter()
        .take(5)
        .map(|hit| hit.path.as_str())
        .collect::<Vec<_>>();

    assert!(
        top_paths.iter().any(|path| path.starts_with("tests/")),
        "auth/tests query should keep test surface visible: {top_paths:?}"
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn agent_bootstrap_broad_service_query_keeps_api_and_domain_service_layers_visible()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-agent-bootstrap-service-broad");
    write_bootstrap_broad_fixture(&project_dir)?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let payload = engine.agent_bootstrap_with_auto_index(
        Some("orchestration domain rules api service layer"),
        6,
        true,
        12_000,
        3_000,
        true,
    )?;

    let bundle = payload.query_bundle.expect("query bundle");
    let top_paths = bundle
        .hits
        .iter()
        .take(5)
        .map(|hit| hit.path.as_str())
        .collect::<Vec<_>>();

    assert!(top_paths.iter().any(|path| path.starts_with("api/")));
    assert!(top_paths.iter().any(|path| {
        path.starts_with("domain/")
            || path.starts_with("services/")
            || path.starts_with("orchestration/")
    }));

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn agent_bootstrap_natural_language_query_uses_data_driven_diversification()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-agent-bootstrap-broad-nl");
    write_bootstrap_broad_fixture(&project_dir)?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let payload = engine.agent_bootstrap_with_auto_index(
        Some("How does the login flow move through the app and which code paths and tests matter most?"),
        5,
        true,
        12_000,
        3_000,
        true,
    )?;

    let bundle = payload.query_bundle.expect("query bundle");
    let top_paths = bundle
        .hits
        .iter()
        .take(4)
        .map(|hit| hit.path.as_str())
        .collect::<Vec<_>>();

    assert!(top_paths.iter().any(|path| path.starts_with("backend/")));
    assert!(
        top_paths
            .iter()
            .any(|path| path.starts_with("frontend/") || path.starts_with("tests/"))
    );
    assert!(
        !bundle.followups.is_empty(),
        "natural-language broad query should emit next-step guidance"
    );

    cleanup_project(&project_dir);
    Ok(())
}
