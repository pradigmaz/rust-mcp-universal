use std::error::Error;
use std::fs;

use rmu_core::{
    Engine, IndexProfile, IndexingOptions, PrivacyMode, QueryOptions, SemanticFailMode,
};

use crate::common::{cleanup_project, temp_project_dir};

#[test]
fn scope_preview_ignores_common_cross_language_artifact_directories() -> Result<(), Box<dyn Error>>
{
    let project_dir = temp_project_dir("rmu-core-tests-ignore-regressions");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("node_modules/react"))?;
    fs::create_dir_all(project_dir.join(".venv/lib"))?;
    fs::create_dir_all(project_dir.join("venv/lib"))?;
    fs::create_dir_all(project_dir.join(".pytest_cache"))?;
    fs::create_dir_all(project_dir.join(".gradle"))?;
    fs::create_dir_all(project_dir.join(".terraform"))?;
    fs::create_dir_all(project_dir.join(".serverless"))?;
    fs::create_dir_all(project_dir.join(".aws-sam/build"))?;

    fs::write(
        project_dir.join("src/main.ts"),
        "export const anchor = 'live';\n",
    )?;
    fs::write(
        project_dir.join("node_modules/react/index.js"),
        "module.exports = {};\n",
    )?;
    fs::write(project_dir.join(".venv/lib/site.py"), "IGNORED = True\n")?;
    fs::write(project_dir.join("venv/lib/site.py"), "IGNORED = True\n")?;
    fs::write(project_dir.join(".pytest_cache/state"), "ignored\n")?;
    fs::write(project_dir.join(".gradle/cache.bin"), "ignored\n")?;
    fs::write(project_dir.join(".terraform/terraform.tfstate"), "{}\n")?;
    fs::write(project_dir.join(".serverless/output.json"), "{}\n")?;
    fs::write(
        project_dir.join(".aws-sam/build/template.yaml"),
        "ignored: true\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let preview = engine.scope_preview_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert_eq!(preview.candidate_paths, vec!["src/main.ts"]);
    for ignored in [
        "node_modules/react/index.js",
        ".venv/lib/site.py",
        "venv/lib/site.py",
        ".pytest_cache/state",
        ".gradle/cache.bin",
        ".terraform/terraform.tfstate",
        ".serverless/output.json",
        ".aws-sam/build/template.yaml",
    ] {
        assert!(
            preview.ignored_paths.contains(&ignored.to_string()),
            "expected ignored path {ignored}"
        );
    }

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn indexing_respects_project_gitignore_for_scoped_code_indexing() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-gitignore-regressions");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("ideas-ui"))?;
    fs::write(project_dir.join(".gitignore"), "ideas-ui/\n")?;
    fs::write(
        project_dir.join("src/main.ts"),
        "export const kept = 'live';\n",
    )?;
    fs::write(
        project_dir.join("ideas-ui/mock.ts"),
        "export const ignored_ui = 'skip';\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let preview = engine.scope_preview_with_options(&IndexingOptions {
        profile: Some(IndexProfile::Mixed),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;
    assert_eq!(preview.candidate_paths, vec!["src/main.ts"]);
    assert!(
        preview
            .ignored_paths
            .contains(&"ideas-ui/mock.ts".to_string())
    );

    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: Some(IndexProfile::Mixed),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;
    let status = engine.index_status()?;
    assert_eq!(status.files, 1);

    let hits = engine.search(&QueryOptions {
        query: "ignored_ui".to_string(),
        limit: 5,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
        agent_intent_mode: None,
    })?;
    assert!(hits.is_empty());

    cleanup_project(&project_dir);
    Ok(())
}
