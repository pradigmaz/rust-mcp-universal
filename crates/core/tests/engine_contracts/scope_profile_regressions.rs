use std::error::Error;
use std::fs;

use rmu_core::{Engine, IndexProfile, IndexingOptions};

use crate::common::{cleanup_project, temp_project_dir};

#[test]
fn mixed_profile_scope_preview_excludes_planning_and_semgrep_artifacts()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-mixed-scope-regressions");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join(".codex-planning"))?;
    fs::write(
        project_dir.join("src/main.ts"),
        "export function chunkVisibility() { return 'ok'; }\n",
    )?;
    fs::write(
        project_dir.join(".codex-planning/task_plan.md"),
        "# task plan\nchunk visibility planning notes\n",
    )?;
    fs::write(project_dir.join("semgrep.json"), "{\"result\":\"noise\"}\n")?;
    fs::write(project_dir.join("semgrep.err"), "noise\n")?;

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
            .excluded_by_scope_paths
            .contains(&".codex-planning/task_plan.md".to_string())
    );
    assert!(
        preview
            .excluded_by_scope_paths
            .contains(&"semgrep.json".to_string())
    );
    assert!(
        preview
            .excluded_by_scope_paths
            .contains(&"semgrep.err".to_string())
    );

    cleanup_project(&project_dir);
    Ok(())
}
