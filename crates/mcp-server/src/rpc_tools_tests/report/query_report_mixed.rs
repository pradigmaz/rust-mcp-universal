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

fn query_search_paths(state: &mut ServerState, query: &str) -> Vec<String> {
    let result = handle_tool_call(
        Some(json!({
            "name": "search_candidates",
            "arguments": {
                "query": query,
                "limit": 5,
                "auto_index": true
            }
        })),
        state,
    )
    .expect("search_candidates should succeed");
    assert_eq!(result["isError"], json!(false));
    result["structuredContent"]["hits"]
        .as_array()
        .expect("hits should be array")
        .iter()
        .filter_map(|hit| hit["path"].as_str().map(str::to_string))
        .collect()
}

fn query_report_paths(state: &mut ServerState, query: &str) -> Vec<String> {
    let result = handle_tool_call(
        Some(json!({
            "name": "query_report",
            "arguments": {
                "query": query,
                "limit": 5,
                "max_chars": 12000,
                "max_tokens": 3000,
                "auto_index": true
            }
        })),
        state,
    )
    .expect("query_report should succeed");
    assert_eq!(result["isError"], json!(false));
    result["structuredContent"]["selected_context"]
        .as_array()
        .expect("selected_context should be array")
        .iter()
        .filter_map(|item| item["path"].as_str().map(str::to_string))
        .collect()
}

#[test]
fn query_report_stays_in_sync_with_search_candidates_for_mixed_queries() {
    let project_dir = temp_dir("rmu-mcp-tests-query-report-mixed");
    write_mixed_fixture(&project_dir);

    let mut state = state_for(project_dir.clone(), Some(project_dir.join(".rmu/index.db")));
    for query in [
        "backend bulk grading deadline validator",
        "frontend journal grades",
        "deadline visibility service",
    ] {
        let search_paths = query_search_paths(&mut state, query);
        let report_paths = query_report_paths(&mut state, query);
        assert_eq!(
            report_paths,
            search_paths
                .into_iter()
                .take(report_paths.len())
                .collect::<Vec<_>>(),
            "mcp search/report parity broke for query `{query}`"
        );
    }

    let _ = fs::remove_dir_all(project_dir);
}
