use super::preflight::preflight_validate;
use crate::args::{Command, IndexCommandArgs};

#[test]
fn delete_index_requires_confirmation_before_engine_init() {
    let err = preflight_validate(&Command::DeleteIndex { yes: false })
        .expect_err("must fail without explicit confirmation");
    assert!(err.to_string().contains("delete-index requires --yes"));
}

#[test]
fn oversized_limit_is_rejected_in_preflight() {
    if usize::BITS < 64 {
        return;
    }

    let oversized = (i64::MAX as usize).saturating_add(1);
    let err = preflight_validate(&Command::Search {
        query: "q".to_string(),
        limit: oversized,
        detailed: false,
        semantic: false,
        auto_index: false,
        semantic_fail_mode: "fail_open".to_string(),
    })
    .expect_err("oversized limit must be rejected");
    assert!(err.to_string().contains("`limit` must be <="));
}

#[test]
fn agent_rejects_whitespace_only_query_in_preflight() {
    let err = preflight_validate(&Command::Agent {
        query: Some("   ".to_string()),
        limit: 20,
        semantic: false,
        auto_index: false,
        semantic_fail_mode: "fail_open".to_string(),
        max_chars: 12_000,
        max_tokens: 3_000,
    })
    .expect_err("whitespace query must be rejected");
    assert!(
        err.to_string()
            .contains("`query` must be non-empty when provided")
    );
}

#[test]
fn symbol_lookup_rejects_whitespace_only_name_in_preflight() {
    let err = preflight_validate(&Command::SymbolLookup {
        name: "   ".to_string(),
        limit: 20,
        auto_index: false,
    })
    .expect_err("whitespace name must be rejected");
    assert!(err.to_string().contains("`name` must be non-empty"));
}

#[test]
fn related_files_rejects_zero_limit_in_preflight() {
    let err = preflight_validate(&Command::RelatedFiles {
        path: "src/main.rs".to_string(),
        limit: 0,
        auto_index: false,
    })
    .expect_err("limit must be >= 1");
    assert!(err.to_string().contains("`limit` must be >= 1"));
}

#[test]
fn call_path_rejects_whitespace_only_from_in_preflight() {
    let err = preflight_validate(&Command::CallPath {
        from: "   ".to_string(),
        to: "src/main.rs".to_string(),
        max_hops: 6,
        auto_index: false,
    })
    .expect_err("whitespace from must be rejected");
    assert!(err.to_string().contains("`from` must be non-empty"));
}

#[test]
fn context_pack_rejects_unknown_mode_in_preflight() {
    let err = preflight_validate(&Command::ContextPack {
        query: "needle".to_string(),
        mode: "unknown".to_string(),
        limit: 20,
        semantic: false,
        auto_index: false,
        semantic_fail_mode: "fail_open".to_string(),
        max_chars: 12_000,
        max_tokens: 3_000,
    })
    .expect_err("unknown mode must be rejected");
    assert!(
        err.to_string()
            .contains("`mode` must be one of: code, design, bugfix")
    );
}

#[test]
fn query_benchmark_rejects_zero_runs_in_preflight() {
    let err = preflight_validate(&Command::QueryBenchmark {
        dataset: "dataset.json".into(),
        k: 10,
        limit: 20,
        semantic: false,
        auto_index: false,
        semantic_fail_mode: "fail_open".to_string(),
        max_chars: 12_000,
        max_tokens: 3_000,
        baseline: Some("baseline.json".into()),
        thresholds: None,
        runs: 0,
        enforce_gates: false,
    })
    .expect_err("runs must be >= 1");
    assert!(err.to_string().contains("`runs` must be >= 1"));
}

#[test]
fn query_benchmark_baseline_mode_requires_baseline_path() {
    let err = preflight_validate(&Command::QueryBenchmark {
        dataset: "dataset.json".into(),
        k: 10,
        limit: 20,
        semantic: true,
        auto_index: false,
        semantic_fail_mode: "fail_open".to_string(),
        max_chars: 12_000,
        max_tokens: 3_000,
        baseline: None,
        thresholds: Some("thresholds.json".into()),
        runs: 1,
        enforce_gates: false,
    })
    .expect_err("baseline mode requires baseline path");
    assert!(err.to_string().contains("requires --baseline"));
}

#[test]
fn query_benchmark_enforce_gates_requires_thresholds() {
    let err = preflight_validate(&Command::QueryBenchmark {
        dataset: "dataset.json".into(),
        k: 10,
        limit: 20,
        semantic: true,
        auto_index: false,
        semantic_fail_mode: "fail_open".to_string(),
        max_chars: 12_000,
        max_tokens: 3_000,
        baseline: Some("baseline.json".into()),
        thresholds: None,
        runs: 1,
        enforce_gates: true,
    })
    .expect_err("enforce gates requires thresholds");
    assert!(
        err.to_string()
            .contains("`--enforce-gates` requires --thresholds")
    );
}

#[test]
fn index_rejects_changed_since_and_changed_since_commit_together() {
    let err = preflight_validate(&Command::Index(IndexCommandArgs {
        changed_since: Some("2026-03-15T10:00:00Z".to_string()),
        changed_since_commit: Some("HEAD".to_string()),
        ..IndexCommandArgs::default()
    }))
    .expect_err("selector modes must be mutually exclusive");
    assert!(
        err.to_string()
            .contains("`changed_since` and `changed_since_commit` are mutually exclusive")
    );
}

#[test]
fn install_ignore_rules_rejects_unknown_target_in_preflight() {
    let err = preflight_validate(&Command::InstallIgnoreRules {
        target: "unknown".to_string(),
    })
    .expect_err("unknown target must be rejected");
    assert!(
        err.to_string()
            .contains("`target` must be one of: git-info-exclude, root-gitignore")
    );
}
