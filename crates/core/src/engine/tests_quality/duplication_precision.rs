use super::{Engine, OptionalExtension, temp_dir, write_project_file};
use serde_json::Value;

#[test]
fn same_file_duplication_does_not_create_duplication_risk() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-same-file");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}}"#,
    )?;
    write_project_file(
        &root,
        "src/lib.rs",
        r#"pub fn alpha(input: i32) -> i32 {
    let mut total = input;
    total += 1;
    total += 2;
    total += 3;
    total += 4;
    total += 5;
    total += 6;
    total += 7;
    total += 8;
    if total > 10 {
        total -= 2;
    }
    if total % 2 == 0 {
        total += 3;
    }
    if total > 40 {
        total -= 5;
    }
    total
}

pub fn beta(input: i32) -> i32 {
    let mut total = input;
    total += 1;
    total += 2;
    total += 3;
    total += 4;
    total += 5;
    total += 6;
    total += 7;
    total += 8;
    if total > 10 {
        total -= 2;
    }
    if total % 2 == 0 {
        total += 3;
    }
    if total > 40 {
        total -= 5;
    }
    total
}
"#,
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let conn = engine.open_db()?;
    let metric: Option<i64> = conn
        .query_row(
            "SELECT metric_value FROM file_quality_metrics WHERE path = 'src/lib.rs' AND metric_id = 'duplicate_density_bps'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let violation: Option<String> = conn
        .query_row(
            "SELECT rule_id FROM file_rule_violations WHERE path = 'src/lib.rs' AND rule_id = 'max_duplicate_density_bps'",
            [],
            |row| row.get(0),
        )
        .optional()?;

    assert_eq!(metric, Some(0));
    assert!(violation.is_none());

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn test_files_are_excluded_from_duplication_scoring() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-tests");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}}"#,
    )?;
    let repeated = r#"import { describe, expect, it } from "vitest";

describe("suite", () => {
  it("keeps repeated setup", () => {
    const value = buildValue({
      alpha: 1,
      beta: 2,
      gamma: 3,
      delta: 4,
      epsilon: 5,
      zeta: 6,
      eta: 7,
      theta: 8,
    })

    expect(value.ready).toBe(true)
  })
})
"#;
    write_project_file(&root, "src/alpha.test.ts", repeated)?;
    write_project_file(&root, "src/beta.test.ts", repeated)?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let conn = engine.open_db()?;
    for path in ["src/alpha.test.ts", "src/beta.test.ts"] {
        let metric: Option<i64> = conn
            .query_row(
                "SELECT metric_value FROM file_quality_metrics WHERE path = ?1 AND metric_id = 'duplicate_density_bps'",
                [path],
                |row| row.get(0),
            )
            .optional()?;
        let violation: Option<String> = conn
            .query_row(
                "SELECT rule_id FROM file_rule_violations WHERE path = ?1 AND rule_id = 'max_duplicate_density_bps'",
                [path],
                |row| row.get(0),
            )
            .optional()?;

        assert_eq!(
            metric,
            Some(0),
            "duplication metric should be zero for {path}"
        );
        assert!(
            violation.is_none(),
            "test file should not carry duplication violation for {path}"
        );
    }

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn duplication_path_pair_policy_suppresses_intentional_clone_class() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-path-pair");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{
            "version":3,
            "thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1},
            "duplication":{
                "suppressions":[
                    {
                        "id":"intentional-alpha-beta",
                        "path_pairs":[{"left":"src/alpha.rs","right":"src/beta.rs"}],
                        "reason":"shared command shell is intentional"
                    }
                ]
            }
        }"#,
    )?;
    let repeated = r#"pub fn repeated(input: i32) -> i32 {
    let mut total = input;
    total += 1;
    total += 2;
    total += 3;
    total += 4;
    total += 5;
    total += 6;
    total += 7;
    total += 8;
    if total > 10 {
        total -= 2;
    }
    if total % 2 == 0 {
        total += 3;
    }
    if total > 40 {
        total -= 5;
    }
    total
}
"#;
    write_project_file(&root, "src/alpha.rs", repeated)?;
    write_project_file(&root, "src/beta.rs", repeated)?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let conn = engine.open_db()?;
    for path in ["src/alpha.rs", "src/beta.rs"] {
        let metric: Option<i64> = conn
            .query_row(
                "SELECT metric_value FROM file_quality_metrics WHERE path = ?1 AND metric_id = 'duplicate_density_bps'",
                [path],
                |row| row.get(0),
            )
            .optional()?;
        let violation: Option<String> = conn
            .query_row(
                "SELECT rule_id FROM file_rule_violations WHERE path = ?1 AND rule_id = 'max_duplicate_density_bps'",
                [path],
                |row| row.get(0),
            )
            .optional()?;
        assert_eq!(
            metric,
            Some(0),
            "suppressed path pair should zero duplication metric"
        );
        assert!(
            violation.is_none(),
            "suppressed path pair should remove violations"
        );
    }

    let artifact: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".rmu/quality/duplication.clone_classes.json"),
    )?)?;
    let suppressed = artifact["suppressed_clone_classes"]
        .as_array()
        .expect("suppressed clone classes should be present");
    assert_eq!(suppressed.len(), 1);
    assert_eq!(
        suppressed[0]["suppressions"][0]["suppression_id"],
        "intentional-alpha-beta"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn duplication_clone_class_id_policy_suppresses_known_clone_class() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-clone-id");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}}"#,
    )?;
    let repeated = r#"export function repeated(input: number): number {
  let total = input
  total += 1
  total += 2
  total += 3
  total += 4
  total += 5
  total += 6
  total += 7
  total += 8
  if (total > 10) {
    total -= 2
  }
  if (total % 2 === 0) {
    total += 3
  }
  if (total > 40) {
    total -= 5
  }
  return total
}
"#;
    write_project_file(&root, "src/alpha.ts", repeated)?;
    write_project_file(&root, "src/beta.ts", repeated)?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let initial_artifact: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".rmu/quality/duplication.clone_classes.json"),
    )?)?;
    let clone_class_id = initial_artifact["clone_classes"]
        .as_array()
        .and_then(|classes| classes.first())
        .and_then(|class| class["clone_class_id"].as_str())
        .expect("clone class id should exist")
        .to_string();

    write_project_file(
        &root,
        "rmu-quality-policy.json",
        &format!(
            r#"{{
                "version":3,
                "thresholds":{{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}},
                "duplication":{{
                    "suppressions":[
                        {{
                            "id":"intentional-ts-duplication",
                            "clone_class_ids":["{clone_class_id}"],
                            "reason":"known shared shell"
                        }}
                    ]
                }}
            }}"#
        ),
    )?;
    write_project_file(&root, "src/alpha.ts", repeated)?;
    engine.index_path()?;

    let conn = engine.open_db()?;
    for path in ["src/alpha.ts", "src/beta.ts"] {
        let metric: Option<i64> = conn
            .query_row(
                "SELECT metric_value FROM file_quality_metrics WHERE path = ?1 AND metric_id = 'duplicate_density_bps'",
                [path],
                |row| row.get(0),
            )
            .optional()?;
        assert_eq!(
            metric,
            Some(0),
            "suppressed clone class should zero duplication metric"
        );
    }

    let suppressed_artifact: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".rmu/quality/duplication.clone_classes.json"),
    )?)?;
    let suppressed = suppressed_artifact["suppressed_clone_classes"]
        .as_array()
        .expect("suppressed clone classes should be present");
    assert_eq!(suppressed.len(), 1);
    assert_eq!(
        suppressed[0]["clone_class"]["clone_class_id"],
        clone_class_id
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
