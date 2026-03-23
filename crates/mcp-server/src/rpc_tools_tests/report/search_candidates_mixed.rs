use std::fs;
use std::path::Path;

use serde_json::json;

use super::*;

const BACKEND_BATCH_PATH: &str = "backend/app/services/attestation/deadline_validator_batch.py";
const BACKEND_VISIBILITY_PATH: &str = "backend/app/services/lab_visibility/visibility_service.py";
const BACKEND_ENDPOINT_PATH: &str = "backend/app/api/v1/endpoints/journal/grades_bulk.py";
const FRONTEND_HOOK_PATH: &str = "frontend/src/app/admin/journal/hooks/useJournalGrades.ts";
const FRONTEND_PAGE_PATH: &str = "frontend/src/app/admin/journal/page.tsx";
const MIGRATION_PATH: &str = "backend/alembic/versions/001_add_deadline_index.py";
const AI_ANALYSIS_PATH: &str = ".ai/temp/attestation_grading_analysis.json";

fn write_mixed_fixture(root: &Path) {
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
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
    }

    fs::write(
        root.join(BACKEND_BATCH_PATH),
        r#"def validate_bulk_deadline_batch():
    note = "backend attestation bulk grading deadline validator batch service"
    return note
"#,
    )
    .expect("write backend batch");
    fs::write(
        root.join(BACKEND_VISIBILITY_PATH),
        r#"def calculate_visibility_for_subject():
    note = "backend attestation deadline visibility service"
    return note
"#,
    )
    .expect("write backend visibility");
    fs::write(
        root.join(BACKEND_ENDPOINT_PATH),
        r#"def grades_bulk_router():
    note = "backend api endpoint router journal grades bulk grading"
    return note
"#,
    )
    .expect("write backend endpoint");
    fs::write(
        root.join(FRONTEND_HOOK_PATH),
        r#"export function useJournalGrades() {
  const label = "frontend journal grades hook"
  return label
}
"#,
    )
    .expect("write frontend hook");
    fs::write(
        root.join(FRONTEND_PAGE_PATH),
        r#"export default function JournalPage() {
  const label = "frontend journal grades page"
  return label
}
"#,
    )
    .expect("write frontend page");
    fs::write(
        root.join(MIGRATION_PATH),
        r#"def upgrade():
    statement = "migration add index schema"
    return statement
"#,
    )
    .expect("write migration");
    fs::write(
        root.join(AI_ANALYSIS_PATH),
        r#"{"title":"attestation grading analysis report","kind":"analysis artifact"}"#,
    )
    .expect("write analysis artifact");
    fs::write(
        root.join("docs/design.md"),
        "design-only journal overview\n",
    )
    .expect("write docs");
}

fn find_rank(paths: &[String], path: &str) -> usize {
    paths
        .iter()
        .position(|candidate| candidate == path)
        .map(|idx| idx + 1)
        .unwrap_or(usize::MAX)
}

#[test]
fn search_candidates_ranks_mixed_fullstack_queries_by_generic_intent() {
    let project_dir = temp_dir("rmu-mcp-tests-search-candidates-mixed");
    write_mixed_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));

    let mut query_paths = |query: &str| -> Vec<String> {
        let result = handle_tool_call(
            Some(json!({
                "name": "search_candidates",
                "arguments": {
                    "query": query,
                    "limit": 5,
                    "auto_index": true
                }
            })),
            &mut state,
        )
        .expect("search_candidates should succeed");
        assert_eq!(result["isError"], json!(false));
        result["structuredContent"]["hits"]
            .as_array()
            .expect("hits should be array")
            .iter()
            .filter_map(|hit| hit["path"].as_str().map(str::to_string))
            .collect()
    };

    let backend_bulk = query_paths("backend bulk grading deadline validator");
    assert_eq!(
        backend_bulk.first().map(String::as_str),
        Some(BACKEND_BATCH_PATH)
    );
    assert!(
        find_rank(&backend_bulk, BACKEND_BATCH_PATH)
            < find_rank(&backend_bulk, BACKEND_ENDPOINT_PATH),
        "backend service-layer query should rank service above endpoint: {backend_bulk:?}"
    );

    let frontend_journal = query_paths("frontend journal grades");
    assert!(
        matches!(
            frontend_journal.first().map(String::as_str),
            Some(FRONTEND_HOOK_PATH | FRONTEND_PAGE_PATH)
        ),
        "frontend query should prefer frontend hook/page: {frontend_journal:?}"
    );
    assert!(
        find_rank(&frontend_journal, FRONTEND_HOOK_PATH)
            < find_rank(&frontend_journal, BACKEND_ENDPOINT_PATH),
        "frontend query should keep backend endpoint below frontend hook: {frontend_journal:?}"
    );

    let backend_api = query_paths("backend api grades bulk");
    assert_eq!(
        backend_api.first().map(String::as_str),
        Some(BACKEND_ENDPOINT_PATH)
    );
    assert!(
        find_rank(&backend_api, BACKEND_ENDPOINT_PATH)
            < find_rank(&backend_api, BACKEND_BATCH_PATH),
        "api-surface query should prefer backend endpoint: {backend_api:?}"
    );

    let migration_hits = query_paths("migration add index");
    assert_eq!(
        migration_hits.first().map(String::as_str),
        Some(MIGRATION_PATH)
    );

    let visibility_hits = query_paths("deadline visibility service");
    assert_eq!(
        visibility_hits.first().map(String::as_str),
        Some(BACKEND_VISIBILITY_PATH)
    );

    let analysis_hits = query_paths("attestation grading analysis");
    assert!(
        find_rank(&analysis_hits, BACKEND_BATCH_PATH) < find_rank(&analysis_hits, AI_ANALYSIS_PATH),
        "analysis artifact should stay below backend code in code-first query: {analysis_hits:?}"
    );

    let _ = fs::remove_dir_all(project_dir);
}
