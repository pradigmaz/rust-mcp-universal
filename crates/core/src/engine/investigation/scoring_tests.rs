use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::engine::Engine;
use crate::model::{ConceptSeedKind, SemanticState};

#[test]
fn concept_cluster_marks_low_signal_query_as_disabled_low_signal() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-low-signal");
    fs::create_dir_all(project_dir.join("app"))?;
    fs::write(
        project_dir.join("app/id_service.py"),
        "def id(value):\n    return value\n",
    )?;

    let engine = Engine::new(&project_dir, Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.concept_cluster("id", ConceptSeedKind::Query, 5)?;

    assert!(!result.variants.is_empty());
    assert!(
        result
            .variants
            .iter()
            .all(|variant| variant.semantic_state == SemanticState::DisabledLowSignal)
    );
    assert!(result.variants.iter().all(|variant| {
        !variant
            .gaps
            .iter()
            .any(|gap| gap == "semantic_unavailable_fail_open")
    }));
    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

#[test]
fn concept_cluster_marks_non_query_seed_as_not_applicable() -> anyhow::Result<()> {
    let project_dir = temp_project_dir("rmu-investigation-path-seed");
    fs::create_dir_all(project_dir.join("app"))?;
    fs::write(
        project_dir.join("app/origin_service.py"),
        "def resolve_origin(value):\n    return value\n",
    )?;

    let engine = Engine::new(&project_dir, Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.ensure_index_ready_with_policy(true)?;
    let result = engine.concept_cluster("app/origin_service.py", ConceptSeedKind::Path, 5)?;

    assert!(!result.variants.is_empty());
    assert!(
        result
            .variants
            .iter()
            .all(|variant| variant.semantic_state == SemanticState::NotApplicable)
    );
    let _ = fs::remove_dir_all(project_dir);
    Ok(())
}

fn temp_project_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&dir).expect("create temp project dir");
    dir
}
