use std::path::{Path, PathBuf};

use assert_cmd::cargo::cargo_bin_cmd;
use rusqlite::Connection;
use serde_json::{Value, json};
use tempfile::tempdir;

fn create_repo(root: &Path, name: &str) -> PathBuf {
    let repo_path = root.join(name);
    std::fs::create_dir_all(repo_path.join("src")).expect("create repo src");
    std::fs::write(
        repo_path.join("src/lib.rs"),
        "pub fn quality_matrix_fixture() -> &'static str { \"ok\" }\n",
    )
    .expect("write repo fixture");
    repo_path
}

fn repo_config(
    id: &str,
    path_key: &str,
    expected_languages: &[&str],
    expected_pre_refresh_statuses: &[&str],
    expected_post_refresh_statuses: &[&str],
    allowed_degradation_reasons: &[&str],
    artifact_bundle: &[&str],
) -> Value {
    json!({
        "id": id,
        "role": "test-repo",
        "path_key": path_key,
        "required": true,
        "profile": "mixed",
        "include_paths": [],
        "exclude_paths": [],
        "expected_languages": expected_languages,
        "size_class": "small",
        "expected_pre_refresh_statuses": expected_pre_refresh_statuses,
        "expected_post_refresh_statuses": expected_post_refresh_statuses,
        "allowed_degradation_reasons": allowed_degradation_reasons,
        "artifact_bundle": artifact_bundle
    })
}

fn write_manifest(project_root: &Path, version: u32, repositories: &[Value]) {
    std::fs::create_dir_all(project_root.join("baseline/quality")).expect("create baseline dir");
    std::fs::write(
        project_root.join("baseline/quality/validation-matrix.json"),
        serde_json::to_string_pretty(&json!({
            "version": version,
            "repositories": repositories,
        }))
        .expect("serialize manifest"),
    )
    .expect("write manifest");
}

fn write_override(project_root: &Path, mappings: &[(&str, &Path)]) {
    std::fs::create_dir_all(project_root.join(".codex")).expect("create codex dir");
    let payload = mappings
        .iter()
        .map(|(key, path)| {
            (
                (*key).to_string(),
                Value::String(path.display().to_string()),
            )
        })
        .collect::<serde_json::Map<String, Value>>();
    std::fs::write(
        project_root.join(".codex/quality-matrix.local.json"),
        serde_json::to_string_pretty(&payload).expect("serialize override"),
    )
    .expect("write override");
}

fn run_quality_matrix(project_root: &Path, repo_ids: &[&str]) -> Value {
    let project_path = project_root.to_str().expect("utf-8 project path");
    let mut cmd = cargo_bin_cmd!("rmu-cli");
    cmd.arg("--project-path")
        .arg(project_path)
        .arg("--json")
        .arg("quality-matrix")
        .arg("--manifest")
        .arg("baseline/quality/validation-matrix.json");
    for repo_id in repo_ids {
        cmd.arg("--repo").arg(repo_id);
    }
    let assert = cmd.assert().success();
    serde_json::from_slice(&assert.get_output().stdout).expect("json stdout")
}

fn index_repo(repo_path: &Path) {
    cargo_bin_cmd!("rmu-cli")
        .arg("--project-path")
        .arg(repo_path.to_str().expect("utf-8 repo path"))
        .arg("semantic-index")
        .arg("--reindex")
        .assert()
        .success();
}

fn make_quality_stale(repo_path: &Path) {
    let conn = Connection::open(repo_path.join(".rmu/index.db")).expect("open db");
    conn.execute("DELETE FROM file_rule_violations", [])
        .expect("delete violations");
    conn.execute("DELETE FROM file_quality_metrics", [])
        .expect("delete metrics");
    conn.execute("DELETE FROM file_quality", [])
        .expect("delete quality");
}

#[test]
fn quality_matrix_single_repo_writes_canonical_summary_without_absolute_repo_paths() {
    let matrix_root = tempdir().expect("temp dir");
    let fixtures_root = tempdir().expect("fixtures dir");
    let repo = create_repo(fixtures_root.path(), "repo-alpha");
    write_manifest(
        matrix_root.path(),
        1,
        &[repo_config(
            "repo_alpha",
            "repo_alpha_path",
            &["rust"],
            &["ready", "stale"],
            &["ready"],
            &[],
            &["default"],
        )],
    );
    write_override(matrix_root.path(), &[("repo_alpha_path", repo.as_path())]);

    let aggregate = run_quality_matrix(matrix_root.path(), &["repo_alpha"]);
    assert_eq!(aggregate["repos"].as_array().map(Vec::len), Some(1));
    assert!(
        aggregate["run_root"]
            .as_str()
            .unwrap_or_default()
            .contains(".codex")
    );
    assert!(
        aggregate["canonical_summary_path"]
            .as_str()
            .unwrap_or_default()
            .contains("baseline-summary.json")
    );

    let canonical_path = matrix_root
        .path()
        .join("baseline/quality/baseline-summary.json");
    let canonical_raw = std::fs::read_to_string(&canonical_path).expect("read canonical summary");
    let canonical: Value = serde_json::from_str(&canonical_raw).expect("parse canonical summary");
    assert_eq!(canonical["manifest_version"], json!(1));
    assert_eq!(canonical["repos"].as_array().map(Vec::len), Some(1));
    assert!(canonical["generated_at_utc"].is_string());
    assert_eq!(
        aggregate["repos"][0]["artifacts"]["violations_by_metric_max_cognitive_complexity"],
        json!("violations.by_metric_max_cognitive_complexity.json")
    );
    assert_eq!(
        aggregate["repos"][0]["artifacts"]["violations_by_metric_duplicate_density_bps"],
        json!("violations.by_metric_duplicate_density_bps.json")
    );
    assert_eq!(
        aggregate["repos"][0]["artifacts"]["duplication_clone_classes"],
        json!("duplication.clone_classes.json")
    );
    assert!(
        aggregate["repos"][0]["latency_summary"]["metric_max_cognitive_complexity_ms"]
            .as_u64()
            .is_some()
    );
    assert!(
        aggregate["repos"][0]["latency_summary"]["metric_duplicate_density_bps_ms"]
            .as_u64()
            .is_some()
    );
    assert_eq!(
        aggregate["repos"][0]["noise_summary"]["manual_review_required"],
        json!(false)
    );
    assert_eq!(
        aggregate["repos"][0]["noise_summary"]["review_shortlist"],
        json!([])
    );
    assert!(aggregate["repos"][0]["top_hot_files"]["metric_max_cognitive_complexity"].is_array());
    assert!(aggregate["repos"][0]["top_hot_files"]["metric_duplicate_density_bps"].is_array());
    let notes = std::fs::read_to_string(
        matrix_root
            .path()
            .join(".codex/quality-matrix/runs")
            .read_dir()
            .expect("runs dir")
            .next()
            .expect("run dir")
            .expect("run dir entry")
            .path()
            .join("repo_alpha/notes.md"),
    )
    .expect("read notes");
    assert!(notes.contains("manual_review_required=false"));
    assert!(notes.contains("review_shortlist="));
    assert!(!canonical_raw.contains(&repo.display().to_string()));
    assert!(!canonical_raw.contains(&matrix_root.path().display().to_string()));
}

#[test]
fn quality_matrix_multi_repo_selection_is_deterministic() {
    let matrix_root = tempdir().expect("temp dir");
    let fixtures_root = tempdir().expect("fixtures dir");
    let repo_a = create_repo(fixtures_root.path(), "repo-a");
    let repo_b = create_repo(fixtures_root.path(), "repo-b");
    write_manifest(
        matrix_root.path(),
        1,
        &[
            repo_config(
                "repo_b",
                "repo_b_path",
                &["rust"],
                &["ready", "stale"],
                &["ready"],
                &[],
                &["default"],
            ),
            repo_config(
                "repo_a",
                "repo_a_path",
                &["rust"],
                &["ready", "stale"],
                &["ready"],
                &[],
                &["default"],
            ),
        ],
    );
    write_override(
        matrix_root.path(),
        &[
            ("repo_a_path", repo_a.as_path()),
            ("repo_b_path", repo_b.as_path()),
        ],
    );

    let aggregate_one = run_quality_matrix(matrix_root.path(), &["repo_b", "repo_a"]);
    let repos_one = aggregate_one["repos"]
        .as_array()
        .expect("repos array")
        .iter()
        .map(|repo| repo["repo_id"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    let aggregate_two = run_quality_matrix(matrix_root.path(), &["repo_a", "repo_b"]);
    let repos_two = aggregate_two["repos"]
        .as_array()
        .expect("repos array")
        .iter()
        .map(|repo| repo["repo_id"].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();

    assert_eq!(repos_one, vec!["repo_a".to_string(), "repo_b".to_string()]);
    assert_eq!(repos_one, repos_two);
}

#[test]
fn quality_matrix_missing_override_fails_for_required_repo() {
    let matrix_root = tempdir().expect("temp dir");
    write_manifest(
        matrix_root.path(),
        1,
        &[repo_config(
            "repo_alpha",
            "repo_alpha_path",
            &["rust"],
            &["ready"],
            &["ready"],
            &[],
            &["default"],
        )],
    );

    cargo_bin_cmd!("rmu-cli")
        .arg("--project-path")
        .arg(matrix_root.path().to_str().expect("utf-8 path"))
        .arg("quality-matrix")
        .arg("--manifest")
        .arg("baseline/quality/validation-matrix.json")
        .assert()
        .failure()
        .stderr(predicates::str::contains("missing local override"));
}

#[test]
fn quality_matrix_unknown_repo_flag_fails() {
    let matrix_root = tempdir().expect("temp dir");
    let fixtures_root = tempdir().expect("fixtures dir");
    let repo = create_repo(fixtures_root.path(), "repo-alpha");
    write_manifest(
        matrix_root.path(),
        1,
        &[repo_config(
            "repo_alpha",
            "repo_alpha_path",
            &["rust"],
            &["ready"],
            &["ready"],
            &[],
            &["default"],
        )],
    );
    write_override(matrix_root.path(), &[("repo_alpha_path", repo.as_path())]);

    cargo_bin_cmd!("rmu-cli")
        .arg("--project-path")
        .arg(matrix_root.path().to_str().expect("utf-8 path"))
        .arg("quality-matrix")
        .arg("--manifest")
        .arg("baseline/quality/validation-matrix.json")
        .arg("--repo")
        .arg("missing_repo")
        .assert()
        .failure()
        .stderr(predicates::str::contains("unknown quality-matrix repo id"));
}

#[test]
fn quality_matrix_invalid_manifest_version_and_artifact_bundle_fail_validation() {
    let matrix_root = tempdir().expect("temp dir");
    let fixtures_root = tempdir().expect("fixtures dir");
    let repo = create_repo(fixtures_root.path(), "repo-alpha");
    write_manifest(
        matrix_root.path(),
        99,
        &[repo_config(
            "repo_alpha",
            "repo_alpha_path",
            &["rust"],
            &["ready"],
            &["ready"],
            &[],
            &["default"],
        )],
    );
    write_override(matrix_root.path(), &[("repo_alpha_path", repo.as_path())]);
    cargo_bin_cmd!("rmu-cli")
        .arg("--project-path")
        .arg(matrix_root.path().to_str().expect("utf-8 path"))
        .arg("quality-matrix")
        .arg("--manifest")
        .arg("baseline/quality/validation-matrix.json")
        .assert()
        .failure()
        .stderr(predicates::str::contains("unsupported version"));

    write_manifest(
        matrix_root.path(),
        1,
        &[repo_config(
            "repo_alpha",
            "repo_alpha_path",
            &["rust"],
            &["ready"],
            &["ready"],
            &[],
            &["custom"],
        )],
    );
    cargo_bin_cmd!("rmu-cli")
        .arg("--project-path")
        .arg(matrix_root.path().to_str().expect("utf-8 path"))
        .arg("quality-matrix")
        .arg("--manifest")
        .arg("baseline/quality/validation-matrix.json")
        .assert()
        .failure()
        .stderr(predicates::str::contains("artifact_bundle"));
}

#[test]
fn quality_matrix_allows_and_rejects_degraded_reasons_from_quality_meta() {
    let matrix_root = tempdir().expect("temp dir");
    let fixtures_root = tempdir().expect("fixtures dir");
    let repo = create_repo(fixtures_root.path(), "repo-alpha");
    index_repo(&repo);
    make_quality_stale(&repo);
    std::fs::write(repo.join("rmu-quality-policy.json"), "{not-valid-json")
        .expect("write invalid policy");

    write_manifest(
        matrix_root.path(),
        1,
        &[repo_config(
            "repo_alpha",
            "repo_alpha_path",
            &["rust"],
            &["stale", "degraded"],
            &["degraded"],
            &["quality_policy"],
            &["default"],
        )],
    );
    write_override(matrix_root.path(), &[("repo_alpha_path", repo.as_path())]);
    let allowed = run_quality_matrix(matrix_root.path(), &["repo_alpha"]);
    assert_eq!(
        allowed["repos"][0]["post_refresh_status"],
        json!("degraded")
    );

    write_manifest(
        matrix_root.path(),
        1,
        &[repo_config(
            "repo_alpha",
            "repo_alpha_path",
            &["rust"],
            &["stale", "degraded"],
            &["degraded"],
            &["some_other_reason"],
            &["default"],
        )],
    );
    cargo_bin_cmd!("rmu-cli")
        .arg("--project-path")
        .arg(matrix_root.path().to_str().expect("utf-8 path"))
        .arg("quality-matrix")
        .arg("--manifest")
        .arg("baseline/quality/validation-matrix.json")
        .arg("--repo")
        .arg("repo_alpha")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "disallowed reason `quality_policy`",
        ));
}

#[test]
fn quality_matrix_reuses_committed_hotspot_baseline_for_deltas() {
    let matrix_root = tempdir().expect("temp dir");
    let fixtures_root = tempdir().expect("fixtures dir");
    let repo = create_repo(fixtures_root.path(), "repo-alpha");
    write_manifest(
        matrix_root.path(),
        1,
        &[repo_config(
            "repo_alpha",
            "repo_alpha_path",
            &["rust"],
            &["ready", "stale"],
            &["ready"],
            &[],
            &["default"],
        )],
    );
    write_override(matrix_root.path(), &[("repo_alpha_path", repo.as_path())]);

    let first = run_quality_matrix(matrix_root.path(), &["repo_alpha"]);
    assert_eq!(first["repos"][0]["repo_id"], json!("repo_alpha"));
    assert!(
        matrix_root
            .path()
            .join("baseline/quality/repos/repo_alpha/file-hotspots.json")
            .exists()
    );
    assert!(
        matrix_root
            .path()
            .join("baseline/quality/repos/repo_alpha/directory-hotspots.json")
            .exists()
    );
    assert!(
        matrix_root
            .path()
            .join("baseline/quality/repos/repo_alpha/module-hotspots.json")
            .exists()
    );

    let second = run_quality_matrix(matrix_root.path(), &["repo_alpha"]);
    assert_eq!(second["repos"][0]["new_violations"], json!(0));
    assert_eq!(second["repos"][0]["resolved_violations"], json!(0));
    assert_eq!(second["repos"][0]["risk_score_delta_total"], json!(0.0));
    assert_eq!(second["repos"][0]["hotspot_score_delta_total"], json!(0.0));
}
