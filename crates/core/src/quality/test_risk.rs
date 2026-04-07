use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use walkdir::WalkDir;

use crate::index_scope::IndexScope;
use crate::model::IndexingOptions;
use crate::quality::{QualityCandidateFacts, TestRiskFacts, TestRiskPolicy};
use crate::utils::{infer_language, normalize_path};

pub(crate) fn load_test_risk_facts(
    project_root: &Path,
    refresh_inputs: &[(&str, &QualityCandidateFacts)],
    policy: &TestRiskPolicy,
) -> Result<HashMap<String, TestRiskFacts>> {
    let mut facts = refresh_inputs
        .iter()
        .map(|(path, _)| ((*path).to_string(), TestRiskFacts::default()))
        .collect::<HashMap<_, _>>();
    if !policy.enabled || refresh_inputs.is_empty() {
        return Ok(facts);
    }

    let test_paths = collect_test_paths(project_root, &policy.test_paths)?;
    let entrypoint_scope = IndexScope::new(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: policy.entrypoint_globs.clone(),
        exclude_paths: Vec::new(),
        reindex: false,
    })?;

    for (path, candidate_facts) in refresh_inputs {
        if candidate_facts.file_kind == crate::quality::metrics::FileKind::Test {
            continue;
        }
        let nearby_tests = test_paths
            .iter()
            .filter(|test_path| {
                is_nearby_test(path, test_path, policy.nearby_max_directory_distance)
            })
            .cloned()
            .collect::<Vec<_>>();
        let nearby_integration_tests = nearby_tests
            .iter()
            .filter(|test_path| is_integration_test_path(test_path))
            .count();

        facts.insert(
            (*path).to_string(),
            TestRiskFacts {
                nearby_test_file_count: i64::try_from(nearby_tests.len()).unwrap_or(i64::MAX),
                nearby_integration_test_file_count: i64::try_from(nearby_integration_tests)
                    .unwrap_or(i64::MAX),
                has_public_surface: has_public_surface(candidate_facts),
                is_hotspot_candidate: hotspot_signal_score(candidate_facts)
                    >= policy.hotspot_requires_test_evidence_min_score,
                is_integration_entry: entrypoint_scope.allows(path),
            },
        );
    }

    Ok(facts)
}

fn collect_test_paths(project_root: &Path, patterns: &[String]) -> Result<HashSet<String>> {
    let matcher = IndexScope::new(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: patterns.to_vec(),
        exclude_paths: Vec::new(),
        reindex: false,
    })?;
    let mut out = HashSet::new();
    for entry in WalkDir::new(project_root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let Ok(rel_path) = entry.path().strip_prefix(project_root) else {
            continue;
        };
        let rel_path = normalize_path(rel_path);
        if matcher.allows(&rel_path) {
            out.insert(rel_path);
        }
    }
    Ok(out)
}

fn is_nearby_test(source_path: &str, test_path: &str, max_directory_distance: usize) -> bool {
    let source = Path::new(source_path);
    let test = Path::new(test_path);
    if normalized_test_stem(source) != normalized_test_stem(test) {
        return false;
    }
    directory_distance(source.parent(), test.parent()) <= max_directory_distance
}

fn normalized_test_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .trim_end_matches(".test")
        .trim_end_matches(".spec")
        .trim_end_matches("_test")
        .trim_end_matches("_spec")
        .to_ascii_lowercase()
}

fn directory_distance(left: Option<&Path>, right: Option<&Path>) -> usize {
    let left_parts = split_segments(left);
    let right_parts = split_segments(right);
    let mut common = 0;
    while common < left_parts.len()
        && common < right_parts.len()
        && left_parts[common] == right_parts[common]
    {
        common += 1;
    }
    left_parts.len() + right_parts.len() - (common * 2)
}

fn split_segments(path: Option<&Path>) -> Vec<String> {
    path.map(|value| {
        value
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_ascii_lowercase())
            .collect::<Vec<_>>()
    })
    .unwrap_or_default()
}

fn is_integration_test_path(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    lowered.contains("/integration/")
        || lowered.contains("/e2e/")
        || lowered.starts_with("integration/")
        || lowered.starts_with("e2e/")
}

fn has_public_surface(facts: &QualityCandidateFacts) -> bool {
    if facts
        .hotspots
        .max_export_count_per_file
        .as_ref()
        .map(|metric| metric.metric_value > 0)
        .unwrap_or(false)
    {
        return true;
    }
    matches!(
        infer_language(&PathBuf::from(&facts.rel_path)).as_str(),
        "rust" | "javascript" | "typescript" | "tsx" | "jsx"
    ) && facts
        .hotspots
        .max_function_lines
        .as_ref()
        .map(|metric| metric.metric_value > 0)
        .unwrap_or(false)
}

fn hotspot_signal_score(facts: &QualityCandidateFacts) -> f64 {
    let mut score = 0.0;
    if facts.non_empty_lines.unwrap_or_default() >= 300 {
        score += 2.0;
    }
    if facts.structural.fan_in_count.unwrap_or_default() >= 20 {
        score += 4.0;
    }
    if facts.structural.fan_out_count.unwrap_or_default() >= 20 {
        score += 4.0;
    }
    if facts
        .hotspots
        .max_cyclomatic_complexity
        .as_ref()
        .map(|metric| metric.metric_value >= 12)
        .unwrap_or(false)
    {
        score += 4.0;
    }
    if facts
        .hotspots
        .max_cognitive_complexity
        .as_ref()
        .map(|metric| metric.metric_value >= 18)
        .unwrap_or(false)
    {
        score += 4.0;
    }
    if facts.duplication.duplicate_density_bps >= 1_500 {
        score += 2.0;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::load_test_risk_facts;
    use crate::quality::{TestRiskPolicy, build_indexed_quality_facts};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must be monotonic")
            .as_nanos();
        std::env::temp_dir().join(format!("{prefix}-{nanos}"))
    }

    #[test]
    fn test_risk_detects_adjacent_tests() {
        let root = temp_dir("rmu-test-risk");
        fs::create_dir_all(root.join("src")).expect("src dir");
        fs::create_dir_all(root.join("src/__tests__")).expect("tests dir");
        fs::write(root.join("src/lib.rs"), "pub fn alpha() {}\n").expect("write source");
        fs::write(
            root.join("src/__tests__/lib_test.rs"),
            "#[test]\nfn alpha() {}\n",
        )
        .expect("write test");

        let facts =
            build_indexed_quality_facts("src/lib.rs", "rust", 16, None, "pub fn alpha() {}\n");
        let output =
            load_test_risk_facts(&root, &[("src/lib.rs", &facts)], &TestRiskPolicy::default())
                .expect("test risk should load");
        assert_eq!(
            output
                .get("src/lib.rs")
                .expect("facts")
                .nearby_test_file_count,
            1
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_risk_marks_public_surface_without_tests() {
        let root = temp_dir("rmu-test-risk-no-tests");
        fs::create_dir_all(root.join("src/api")).expect("src dir");
        fs::write(
            root.join("src/api/handler.ts"),
            "export function handler() { return 1; }\n",
        )
        .expect("write source");

        let facts = build_indexed_quality_facts(
            "src/api/handler.ts",
            "typescript",
            32,
            None,
            "export function handler() { return 1; }\n",
        );
        let output = load_test_risk_facts(
            &root,
            &[("src/api/handler.ts", &facts)],
            &TestRiskPolicy::default(),
        )
        .expect("test risk should load");
        let handler = output.get("src/api/handler.ts").expect("facts");
        assert_eq!(handler.nearby_test_file_count, 0);
        assert!(handler.has_public_surface);
        assert!(handler.is_integration_entry);

        let _ = fs::remove_dir_all(root);
    }
}
