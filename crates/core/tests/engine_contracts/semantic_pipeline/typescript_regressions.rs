#[test]
fn precise_typescript_anchor_survives_graph_refusion_and_planning_noise()
-> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_typescript_chunk_project()?;

    let report = engine.build_report(
        &QueryOptions {
            query: "ChunkCulling".to_string(),
            limit: 5,
            detailed: true,
            semantic: true,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        agent_intent_mode: None,
        },
        20_000,
        6_000,
    )?;

    let first = report
        .selected_context
        .first()
        .expect("expected at least one selected result");
    assert_eq!(first.path, "src/world/chunks/ChunkCulling.ts");
    assert!(
        report
            .selected_context
            .iter()
            .all(|item| !item.path.contains(".codex-planning/"))
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn precise_multi_token_query_prefers_chunk_visibility_source_file()
-> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_typescript_chunk_project()?;

    let report = engine.build_report(
        &QueryOptions {
            query: "chunk visibility".to_string(),
            limit: 5,
            detailed: true,
            semantic: true,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        agent_intent_mode: None,
        },
        20_000,
        6_000,
    )?;

    let first = report
        .selected_context
        .first()
        .expect("expected at least one selected result");
    assert_eq!(first.path, "src/world/chunks/ChunkVisibility.ts");
    assert!(
        report
            .selected_context
            .iter()
            .all(|item| !item.path.contains(".codex-planning/"))
    );

    cleanup_project(&project_dir);
    Ok(())
}

fn setup_typescript_chunk_project() -> Result<(std::path::PathBuf, Engine), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-typescript-chunk-regressions");
    fs::create_dir_all(project_dir.join("src/world/chunks"))?;
    fs::create_dir_all(project_dir.join("src/world/shared"))?;
    fs::create_dir_all(project_dir.join("src/ui/debug"))?;
    fs::create_dir_all(project_dir.join("src/profiler"))?;
    fs::create_dir_all(project_dir.join(".codex-planning"))?;

    fs::write(
        project_dir.join("src/world/shared/ChunkRuntime.ts"),
        r#"export interface ChunkRuntime {
  viewDistance: number;
  frustumPlanes: number[];
}
"#,
    )?;
    fs::write(
        project_dir.join("src/world/chunks/ChunkCulling.ts"),
        r#"import type { ChunkRuntime } from "../shared/ChunkRuntime";

export class ChunkCulling {
  public compute(runtime: ChunkRuntime): string {
    const label = "chunk culling frustum culling visibility";
    return `${label}:${runtime.viewDistance}`;
  }
}
"#,
    )?;
    fs::write(
        project_dir.join("src/world/chunks/ChunkVisibility.ts"),
        r#"import type { ChunkRuntime } from "../shared/ChunkRuntime";
import { ChunkCulling } from "./ChunkCulling";

export function updateChunkVisibility(runtime: ChunkRuntime): string {
  const culling = new ChunkCulling();
  return `chunk visibility ${culling.compute(runtime)}`;
}
"#,
    )?;
    fs::write(
        project_dir.join("src/ui/debug/FrustumDebugMenu.ts"),
        r#"import type { ChunkRuntime } from "../../world/shared/ChunkRuntime";

export function renderFrustumDebugMenu(runtime: ChunkRuntime): string {
  return `frustum debug menu ${runtime.viewDistance} chunk inspector`;
}
"#,
    )?;
    fs::write(
        project_dir.join("src/profiler/WorldProfiler.ts"),
        r#"import type { ChunkRuntime } from "../world/shared/ChunkRuntime";

export function describeWorldProfiler(runtime: ChunkRuntime): string {
  return `world profiler chunk visibility counters ${runtime.viewDistance}`;
}
"#,
    )?;
    fs::write(
        project_dir.join(".codex-planning/tmp_WorldGenPresets_raw.txt"),
        "chunk visibility chunk visibility frustum culling chunk culling planning notes\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path_with_options(&rmu_core::IndexingOptions {
        profile: Some(rmu_core::IndexProfile::Mixed),
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    Ok((project_dir, engine))
}
