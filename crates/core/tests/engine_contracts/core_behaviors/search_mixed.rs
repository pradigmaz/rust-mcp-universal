use super::*;

const BACKEND_BATCH_PATH: &str = "backend/app/services/attestation/deadline_validator_batch.py";
const BACKEND_VISIBILITY_PATH: &str = "backend/app/services/lab_visibility/visibility_service.py";
const BACKEND_ENDPOINT_PATH: &str = "backend/app/api/v1/endpoints/journal/grades_bulk.py";
const FRONTEND_HOOK_PATH: &str = "frontend/src/app/admin/journal/hooks/useJournalGrades.ts";
const FRONTEND_PAGE_PATH: &str = "frontend/src/app/admin/journal/page.tsx";
const MIGRATION_PATH: &str = "backend/alembic/versions/001_add_deadline_index.py";
const AI_ANALYSIS_PATH: &str = ".ai/temp/attestation_grading_analysis.json";

fn write_mixed_fullstack_fixture(project_dir: &Path) -> Result<(), Box<dyn Error>> {
    for relative in [
        BACKEND_BATCH_PATH,
        BACKEND_VISIBILITY_PATH,
        BACKEND_ENDPOINT_PATH,
        FRONTEND_HOOK_PATH,
        FRONTEND_PAGE_PATH,
        MIGRATION_PATH,
        AI_ANALYSIS_PATH,
        "docs/design.md",
    ] {
        let path = project_dir.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
    }

    fs::write(
        project_dir.join(BACKEND_BATCH_PATH),
        r#"def validate_bulk_deadline_batch():
    note = "backend attestation bulk grading deadline validator batch service"
    return note
"#,
    )?;
    fs::write(
        project_dir.join(BACKEND_VISIBILITY_PATH),
        r#"def calculate_visibility_for_subject():
    note = "backend attestation deadline visibility service"
    return note
"#,
    )?;
    fs::write(
        project_dir.join(BACKEND_ENDPOINT_PATH),
        r#"def grades_bulk_router():
    note = "backend api endpoint router journal grades bulk grading"
    return note
"#,
    )?;
    fs::write(
        project_dir.join(FRONTEND_HOOK_PATH),
        r#"export function useJournalGrades() {
  const label = "frontend journal grades hook"
  return label
}
"#,
    )?;
    fs::write(
        project_dir.join(FRONTEND_PAGE_PATH),
        r#"export default function JournalPage() {
  const label = "frontend journal grades page"
  return label
}
"#,
    )?;
    fs::write(
        project_dir.join(MIGRATION_PATH),
        r#"def upgrade():
    statement = "migration add index schema"
    return statement
"#,
    )?;
    fs::write(
        project_dir.join(AI_ANALYSIS_PATH),
        r#"{"title":"attestation grading analysis report","kind":"analysis artifact"}"#,
    )?;
    fs::write(
        project_dir.join("docs/design.md"),
        "design-only journal overview\n",
    )?;

    Ok(())
}

fn find_rank(hits: &[rmu_core::SearchHit], path: &str) -> usize {
    hits.iter()
        .position(|hit| hit.path == path)
        .map(|idx| idx + 1)
        .unwrap_or(usize::MAX)
}

fn search_paths(engine: &Engine, query: &str) -> Result<Vec<rmu_core::SearchHit>, Box<dyn Error>> {
    Ok(engine.search(&QueryOptions {
        query: query.to_string(),
        limit: 5,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
        agent_intent_mode: None,
    })?)
}

fn report_paths(engine: &Engine, query: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let report = engine.build_report(
        &QueryOptions {
            query: query.to_string(),
            limit: 5,
            detailed: true,
            semantic: false,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            context_mode: None,
        agent_intent_mode: None,
        },
        12_000,
        3_000,
    )?;
    Ok(report
        .selected_context
        .into_iter()
        .map(|item| item.path)
        .collect())
}

#[test]
fn search_ranks_mixed_fullstack_queries_by_generic_code_intent() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-search-mixed-intent");
    write_mixed_fullstack_fixture(&project_dir)?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    let backend_bulk = search_paths(&engine, "backend bulk grading deadline validator")?;
    assert_eq!(
        backend_bulk.first().map(|hit| hit.path.as_str()),
        Some(BACKEND_BATCH_PATH),
        "backend bulk query order: {:?}",
        backend_bulk
            .iter()
            .map(|hit| (hit.path.as_str(), hit.score))
            .collect::<Vec<_>>()
    );
    assert!(
        find_rank(&backend_bulk, BACKEND_BATCH_PATH)
            < find_rank(&backend_bulk, BACKEND_ENDPOINT_PATH),
        "backend service batch should outrank backend endpoint for service-layer query: {:?}",
        backend_bulk
            .iter()
            .map(|hit| hit.path.as_str())
            .collect::<Vec<_>>()
    );

    let frontend_journal = search_paths(&engine, "frontend journal grades")?;
    let first_frontend = frontend_journal
        .first()
        .map(|hit| hit.path.as_str())
        .expect("frontend query should produce hits");
    assert!(
        matches!(first_frontend, FRONTEND_HOOK_PATH | FRONTEND_PAGE_PATH),
        "frontend query should prefer hook/page file, got {first_frontend}"
    );
    assert!(
        find_rank(&frontend_journal, FRONTEND_HOOK_PATH)
            < find_rank(&frontend_journal, BACKEND_ENDPOINT_PATH),
        "frontend hook should outrank backend endpoint: {:?}",
        frontend_journal
            .iter()
            .map(|hit| hit.path.as_str())
            .collect::<Vec<_>>()
    );

    let backend_api = search_paths(&engine, "backend api grades bulk")?;
    assert_eq!(
        backend_api.first().map(|hit| hit.path.as_str()),
        Some(BACKEND_ENDPOINT_PATH)
    );
    assert!(
        find_rank(&backend_api, BACKEND_ENDPOINT_PATH)
            < find_rank(&backend_api, BACKEND_BATCH_PATH),
        "api-surface query should still prefer backend endpoint: {:?}",
        backend_api
            .iter()
            .map(|hit| hit.path.as_str())
            .collect::<Vec<_>>()
    );

    let migration_hits = search_paths(&engine, "migration add index")?;
    assert_eq!(
        migration_hits.first().map(|hit| hit.path.as_str()),
        Some(MIGRATION_PATH)
    );

    let visibility_hits = search_paths(&engine, "deadline visibility service")?;
    assert_eq!(
        visibility_hits.first().map(|hit| hit.path.as_str()),
        Some(BACKEND_VISIBILITY_PATH),
        "visibility query order: {:?}",
        visibility_hits
            .iter()
            .map(|hit| (hit.path.as_str(), hit.score))
            .collect::<Vec<_>>()
    );
    assert!(
        find_rank(&visibility_hits, BACKEND_VISIBILITY_PATH)
            < find_rank(&visibility_hits, BACKEND_ENDPOINT_PATH),
        "visibility service should outrank backend endpoint: {:?}",
        visibility_hits
            .iter()
            .map(|hit| hit.path.as_str())
            .collect::<Vec<_>>()
    );

    let analysis_hits = search_paths(&engine, "attestation grading analysis")?;
    assert!(
        find_rank(&analysis_hits, BACKEND_BATCH_PATH) < find_rank(&analysis_hits, AI_ANALYSIS_PATH),
        "code-first query should keep support artifact below backend code: {:?}",
        analysis_hits
            .iter()
            .map(|hit| hit.path.as_str())
            .collect::<Vec<_>>()
    );

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn search_and_report_keep_the_same_top_order_for_mixed_queries() -> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-search-mixed-parity");
    write_mixed_fullstack_fixture(&project_dir)?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    engine.index_path()?;

    for query in [
        "backend bulk grading deadline validator",
        "frontend journal grades",
        "deadline visibility service",
    ] {
        let search_hits = search_paths(&engine, query)?;
        let report_paths = report_paths(&engine, query)?;
        let search_paths = search_hits
            .iter()
            .map(|hit| hit.path.clone())
            .take(report_paths.len())
            .collect::<Vec<_>>();
        assert_eq!(
            report_paths, search_paths,
            "search/report parity broke for query `{query}`"
        );
    }

    cleanup_project(&project_dir);
    Ok(())
}
