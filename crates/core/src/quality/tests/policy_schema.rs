use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::quality::policy_schema::{CURRENT_QUALITY_POLICY_VERSION, parse_quality_policy_file};
use crate::quality::{default_quality_policy, load_quality_policy};

fn temp_dir(prefix: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

#[test]
fn policy_schema_accepts_current_version() {
    let parsed = parse_quality_policy_file(
        &format!(r#"{{"version":{CURRENT_QUALITY_POLICY_VERSION},"thresholds":{{}}}}"#),
        std::path::Path::new("policy.json"),
    )
    .expect("current version should parse");
    assert_eq!(parsed.version, CURRENT_QUALITY_POLICY_VERSION);
}

#[test]
fn policy_schema_rejects_unknown_version() {
    let err = parse_quality_policy_file(
        r#"{"version":99,"thresholds":{}}"#,
        std::path::Path::new("policy.json"),
    )
    .expect_err("unknown version must fail");
    assert!(err.to_string().contains("unsupported version"));
}

#[test]
fn load_quality_policy_applies_overrides() {
    let root = temp_dir("rmu-quality-policy-schema");
    fs::create_dir_all(&root).expect("create temp dir");
    fs::write(
        root.join("rmu-quality-policy.json"),
        r#"{"version":2,"thresholds":{"max_non_empty_lines_default":400,"max_function_lines":12}}"#,
    )
    .expect("write policy");

    let policy = load_quality_policy(&root).expect("policy should load");
    assert_eq!(policy.thresholds.max_non_empty_lines_default, 400);
    assert_eq!(policy.thresholds.max_function_lines, 12);
    assert_eq!(
        policy.thresholds.max_import_count,
        default_quality_policy().thresholds.max_import_count
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn load_quality_policy_reads_quality_scope_and_structural_sections() {
    let root = temp_dir("rmu-quality-policy-structural");
    fs::create_dir_all(&root).expect("create temp dir");
    fs::write(
        root.join("rmu-quality-policy.json"),
        r#"{
            "version":2,
            "thresholds":{"max_fan_in_per_file":9,"max_fan_out_per_file":7},
            "quality_scope":{"exclude_paths":["generated/**"]},
            "structural":{
                "zones":[
                    {"id":"ui","paths":["src/ui/**"]},
                    {"id":"domain","paths":["src/domain/**"]}
                ],
                "allowed_directions":[{"from":"ui","to":"domain"}],
                "forbidden_edges":[{"from":"domain","to":"ui","reason":"keep the dependency inverted"}]
            }
        }"#,
    )
    .expect("write policy");

    let policy = load_quality_policy(&root).expect("policy should load");
    assert_eq!(policy.thresholds.max_fan_in_per_file, 9);
    assert_eq!(policy.thresholds.max_fan_out_per_file, 7);
    assert_eq!(policy.quality_scope.exclude_paths, vec!["generated/**"]);
    let structural = policy.structural.expect("structural policy should exist");
    assert_eq!(structural.zones.len(), 2);
    assert_eq!(structural.allowed_directions.len(), 1);
    assert_eq!(structural.forbidden_edges.len(), 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn policy_schema_rejects_duplicate_structural_zone_patterns() {
    let err = parse_quality_policy_file(
        r#"{
            "version":2,
            "thresholds":{},
            "structural":{
                "zones":[
                    {"id":"ui","paths":["src/shared/**"]},
                    {"id":"domain","paths":["src/shared/**"]}
                ]
            }
        }"#,
        std::path::Path::new("policy.json"),
    )
    .expect_err("duplicate structural path patterns must fail");
    assert!(
        err.to_string()
            .contains("overlapping structural zone pattern")
    );
}

#[test]
fn load_quality_policy_applies_rule_metadata_path_scopes_and_suppressions() {
    let root = temp_dir("rmu-quality-policy-stage3");
    fs::create_dir_all(&root).expect("create temp dir");
    fs::write(
        root.join("rmu-quality-policy.json"),
        r#"{
            "version":2,
            "rule_overrides":{
                "max_line_length":{"severity":"high","category":"risk"}
            },
            "path_scopes":[
                {
                    "id":"tests",
                    "paths":["tests/**"],
                    "thresholds":{"max_line_length":999},
                    "rule_overrides":{
                        "max_line_length":{"severity":"low","category":"style"}
                    },
                    "suppressions":[
                        {
                            "id":"tests-line-length",
                            "rule_ids":["max_line_length"],
                            "paths":["tests/**"],
                            "reason":"test fixtures stay verbose"
                        }
                    ]
                }
            ],
            "suppressions":[
                {
                    "id":"root-todo",
                    "rule_ids":["max_todo_count_per_file"],
                    "paths":["src/generated/**"],
                    "reason":"generated snapshots"
                }
            ]
        }"#,
    )
    .expect("write policy");

    let policy = load_quality_policy(&root).expect("policy should load");
    let src_policy = policy.effective_for_path("src/lib.rs");
    assert_eq!(
        src_policy
            .metadata_for_rule("max_line_length")
            .severity
            .as_str(),
        "high"
    );
    assert_eq!(
        src_policy
            .metadata_for_rule("max_line_length")
            .category
            .as_str(),
        "risk"
    );

    let test_policy = policy.effective_for_path("tests/heavy.rs");
    assert_eq!(test_policy.thresholds.max_line_length, 999);
    assert_eq!(
        test_policy
            .metadata_for_rule("max_line_length")
            .severity
            .as_str(),
        "low"
    );
    assert_eq!(
        test_policy
            .metadata_for_rule("max_line_length")
            .category
            .as_str(),
        "style"
    );
    let suppressions = test_policy.suppressions_for_rule("max_line_length");
    assert_eq!(suppressions.len(), 1);
    assert_eq!(suppressions[0].suppression_id, "tests-line-length");
    assert_eq!(suppressions[0].scope_id.as_deref(), Some("tests"));

    let generated_policy = policy.effective_for_path("src/generated/file.rs");
    let root_suppressions = generated_policy.suppressions_for_rule("max_todo_count_per_file");
    assert_eq!(root_suppressions.len(), 1);
    assert_eq!(root_suppressions[0].suppression_id, "root-todo");
    assert!(root_suppressions[0].scope_id.is_none());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn policy_schema_rejects_unknown_rule_metadata_and_suppressions() {
    let err = parse_quality_policy_file(
        r#"{
            "version":2,
            "rule_overrides":{"unknown_rule":{"severity":"high"}},
            "thresholds":{}
        }"#,
        std::path::Path::new("policy.json"),
    )
    .expect_err("unknown rule metadata should fail");
    assert!(err.to_string().contains("unknown rule"));

    let err = parse_quality_policy_file(
        r#"{
            "version":2,
            "thresholds":{},
            "suppressions":[
                {"id":"bad","rule_ids":["unknown_rule"],"paths":["src/**"],"reason":"x"}
            ]
        }"#,
        std::path::Path::new("policy.json"),
    )
    .expect_err("unknown suppression rule should fail");
    assert!(err.to_string().contains("unknown rule"));
}
