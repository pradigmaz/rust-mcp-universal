use super::{Engine, temp_dir, write_project_file};
use serde_json::Value;

#[test]
fn type3_near_miss_clone_records_sub_100_similarity() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-type3");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}}"#,
    )?;
    write_project_file(
        &root,
        "src/alpha.ts",
        r#"export function alpha(input: number): number {
  let total = input
  total += 1
  total += 2
  total += 3
  total += 4
  if (total > 4) {
    total -= 1
  }
  total += 5
  total += 6
  total += 7
  total += 8
  if (total > 8) {
    total -= 2
  }
  total += 9
  total += 10
  total += 11
  total += 12
  if (total > 12) {
    total -= 3
  }
  return total
}
"#,
    )?;
    write_project_file(
        &root,
        "src/beta.ts",
        r#"export function beta(input: number): number {
  let total = input
  total += 1
  total += 2
  total += 3
  total += 4
  if (total > 4) {
    total += 99
  }
  total += 5
  total += 6
  total += 7
  total += 8
  if (total > 8) {
    total += 77
  }
  total += 9
  total += 10
  total += 11
  total += 12
  if (total > 12) {
    total += 55
  }
  return total
}
"#,
    )?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let artifact: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".rmu/quality/duplication.clone_classes.json"),
    )?)?;
    assert!(
        artifact["clone_classes"]
            .as_array()
            .expect("clone classes")
            .iter()
            .any(|class| {
                class["normalized_token_count"]
                    .as_u64()
                    .is_some_and(|value| value >= 32)
                    && class["similarity_percent"]
                        .as_i64()
                        .is_some_and(|value| (85..100).contains(&value))
            })
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn mixed_model_and_service_clone_class_stays_primary() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-mixed-role");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}}"#,
    )?;
    let repeated = r#"@dataclass
class SharedProjection:
    id: int
    title: str
    locale: str
    timezone: str
    section: str
    description: str
    icon: str
    badge: str
    status: str
    owner: str
    category: str
    channel: str
    subtitle: str
    summary: str
    slug: str
    region: str
    language: str
    audience: str
    theme: str
    source: str
    accent: str
    layout: str
    feature_flag: str
    rollout_bucket: str
    help_url: str
    support_url: str
    owner_avatar: str
    icon_variant: str
    sort_order: int
    sort_group: str
    created_by: str
    updated_by: str
    sync_token: str
    version_tag: str
    priority: int
    archived: bool = False
"#;
    write_project_file(&root, "app/models/user_schema.py", repeated)?;
    write_project_file(&root, "app/models/admin_schema.py", repeated)?;
    write_project_file(&root, "app/services/user_projection.py", repeated)?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let artifact: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".rmu/quality/duplication.clone_classes.json"),
    )?)?;
    assert!(
        artifact["clone_classes"]
            .as_array()
            .expect("clone classes")
            .iter()
            .any(|class| {
                class["members"]
                    .as_array()
                    .is_some_and(|members| members.len() >= 3)
                    && class["signal_role"] == "boilerplate"
            })
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn mixed_imperative_clone_class_stays_primary() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-mixed-imperative-role");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}}"#,
    )?;
    let repeated = r#"def build_projection(input_value: int) -> int:
    total = input_value
    total += 1
    total += 2
    total += 3
    total += 4
    total += 5
    total += 6
    total += 7
    total += 8
    total += 9
    total += 10
    total += 11
    total += 12
    total += 13
    total += 14
    total += 15
    total += 16
    total += 17
    total += 18
    total += 19
    total += 20
    total += 21
    total += 22
    total += 23
    total += 24
    total += 25
    total += 26
    total += 27
    total += 28
    total += 29
    total += 30
    total += 31
    total += 32
    total += 33
    total += 34
    total += 35
    total += 36
    total += 37
    total += 38
    total += 39
    total += 40
    return total
"#;
    write_project_file(&root, "app/models/user_projection.py", repeated)?;
    write_project_file(&root, "app/models/admin_projection.py", repeated)?;
    write_project_file(&root, "app/services/user_projection.py", repeated)?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let artifact: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join(".rmu/quality/duplication.clone_classes.json"),
    )?)?;
    assert!(
        artifact["clone_classes"]
            .as_array()
            .expect("clone classes")
            .iter()
            .any(|class| {
                class["members"]
                    .as_array()
                    .is_some_and(|members| members.len() >= 3)
                    && class["signal_reason"].is_null()
            })
    );
    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
