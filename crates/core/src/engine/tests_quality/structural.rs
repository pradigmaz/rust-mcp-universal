use crate::model::{IndexProfile, IndexingOptions, RuleViolationsOptions};

use super::{Engine, temp_dir, write_project_file};

#[test]
fn rule_violations_expose_structural_hotspots() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-structural");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{
            "version":3,
            "thresholds":{"max_fan_in_per_file":1,"max_fan_out_per_file":1},
            "structural":{
                "zones":[
                    {"id":"ui","paths":["src/ui/**"]},
                    {"id":"domain","paths":["src/domain/**"]},
                    {"id":"data","paths":["src/data/**"]}
                ],
                "allowed_directions":[
                    {"from":"ui","to":"domain"},
                    {"from":"domain","to":"data"}
                ],
                "forbidden_edges":[
                    {"from":"ui","to":"data","reason":"ui must not talk to data directly"}
                ]
            }
        }"#,
    )?;
    write_project_file(
        &root,
        "src/ui/view.ts",
        "import { run } from '../domain/use_case';\nimport { save } from '../data/repo';\nexport function view() { return run() + save(); }\n",
    )?;
    write_project_file(
        &root,
        "src/domain/use_case.ts",
        "import { save } from '../data/repo';\nexport function run() { return save(); }\n",
    )?;
    write_project_file(
        &root,
        "src/domain/isolated.ts",
        "export const lonely = 1;\n",
    )?;
    write_project_file(
        &root,
        "src/data/repo.ts",
        "import { view } from '../ui/view';\nexport function save() { return typeof view === 'function' ? 1 : 0; }\n",
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path_with_options(&IndexingOptions {
        profile: Some(IndexProfile::Mixed),
        changed_since: None,
        changed_since_commit: None,
        include_paths: Vec::new(),
        exclude_paths: Vec::new(),
        reindex: true,
    })?;

    let result = engine.rule_violations(&RuleViolationsOptions {
        rule_ids: vec![
            "max_fan_in_per_file".to_string(),
            "max_fan_out_per_file".to_string(),
            "module_cycle_member".to_string(),
            "cross_layer_dependency".to_string(),
            "orphan_module".to_string(),
        ],
        ..RuleViolationsOptions::default()
    })?;

    let ui_hit = result
        .hits
        .iter()
        .find(|hit| hit.path == "src/ui/view.ts")
        .expect("ui file should be present");
    assert!(
        ui_hit
            .metrics
            .iter()
            .any(|metric| metric.metric_id == "fan_out_count" && metric.metric_value == 2)
    );
    assert!(
        ui_hit
            .violations
            .iter()
            .any(|violation| violation.rule_id == "max_fan_out_per_file")
    );
    assert!(
        ui_hit
            .violations
            .iter()
            .any(|violation| violation.rule_id == "cross_layer_dependency")
    );
    assert!(
        ui_hit
            .violations
            .iter()
            .any(|violation| violation.rule_id == "module_cycle_member")
    );

    let isolated_hit = result
        .hits
        .iter()
        .find(|hit| hit.path == "src/domain/isolated.ts")
        .expect("isolated domain file should be present");
    assert!(
        isolated_hit
            .violations
            .iter()
            .any(|violation| violation.rule_id == "orphan_module")
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
