use super::{Engine, OptionalExtension, temp_dir, write_project_file};
use serde_json::Value;

#[test]
fn java_annotation_wrappers_stay_info_only() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-java-wrapper");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}}"#,
    )?;
    let left = r#"@Entity
@Table(name = "user_config_view")
public class UserConfigView {
    private String id;
    private String title;
    private String locale;
    private String timezone;
    private String section;
    private String description;
    private String icon;
    private String badge;
    private String status;
    private String owner;
}
"#;
    let right = r#"@Entity
@Table(name = "admin_config_view")
public class AdminConfigView {
    private String id;
    private String title;
    private String locale;
    private String timezone;
    private String section;
    private String description;
    private String icon;
    private String badge;
    private String status;
    private String owner;
}
"#;
    write_project_file(&root, "src/main/java/app/admin/UserConfigView.java", left)?;
    write_project_file(&root, "src/main/java/app/admin/AdminConfigView.java", right)?;
    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;
    let conn = engine.open_db()?;
    for path in [
        "src/main/java/app/admin/UserConfigView.java",
        "src/main/java/app/admin/AdminConfigView.java",
    ] {
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
            "annotation wrapper should stay info-only for {path}"
        );
    }

    let artifact: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(".rmu/quality/duplication.clone_classes.json"))?)?;
    assert!(
        artifact["clone_classes"]
            .as_array()
            .expect("clone classes")
            .iter()
            .any(|class| {
                class["signal_reason"] == "java_annotation_wrapper"
                    && class["signal_role"] == "boilerplate"
            })
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
#[test]
fn rust_macro_shells_stay_info_only() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-rust-macro-shell");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}}"#,
    )?;
    let alpha = r#"#[allow(dead_code)]
pub fn alpha_shell() {
    tracing::info!("alpha");
    tracing::debug!("alpha");
    tracing::warn!("alpha");
    tracing::error!("alpha");
    tracing::info!("alpha");
    tracing::debug!("alpha");
    tracing::warn!("alpha");
    tracing::error!("alpha");
}
"#;
    let beta = r#"#[allow(dead_code)]
pub fn beta_shell() {
    tracing::info!("beta");
    tracing::debug!("beta");
    tracing::warn!("beta");
    tracing::error!("beta");
    tracing::info!("beta");
    tracing::debug!("beta");
    tracing::warn!("beta");
    tracing::error!("beta");
}
"#;
    write_project_file(&root, "src/alpha_shell.rs", alpha)?;
    write_project_file(&root, "src/beta_shell.rs", beta)?;
    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;
    let conn = engine.open_db()?;
    for path in ["src/alpha_shell.rs", "src/beta_shell.rs"] {
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
            "macro shell should stay info-only for {path}"
        );
    }

    let artifact: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(".rmu/quality/duplication.clone_classes.json"))?)?;
    assert!(
        artifact["clone_classes"]
            .as_array()
            .expect("clone classes")
            .iter()
            .any(|class| class["signal_reason"] == "rust_macro_shell")
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
#[test]
fn python_model_boilerplate_stays_info_only() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-python-models");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}}"#,
    )?;
    let left = r#"@dataclass
class UserSchema:
    id: int
    name: str
    email: str
    active: bool = True
    locale: str = "ru"
    timezone: str = "UTC"
    tags: list[str] = field(default_factory=list)
    last_login: str | None = None
    avatar_url: str | None = None
    notes: str | None = None
    role: str = "user"
    archived: bool = False
"#;
    let right = r#"@dataclass
class AdminSchema:
    id: int
    name: str
    email: str
    active: bool = True
    locale: str = "ru"
    timezone: str = "UTC"
    tags: list[str] = field(default_factory=list)
    last_login: str | None = None
    avatar_url: str | None = None
    notes: str | None = None
    role: str = "admin"
    archived: bool = False
"#;
    write_project_file(&root, "app/models/user_schema.py", left)?;
    write_project_file(&root, "app/models/admin_schema.py", right)?;
    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;
    let conn = engine.open_db()?;
    for path in ["app/models/user_schema.py", "app/models/admin_schema.py"] {
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
            "boilerplate model should stay info-only for {path}"
        );
    }

    let artifact: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(".rmu/quality/duplication.clone_classes.json"))?)?;
    assert!(
        artifact["clone_classes"]
            .as_array()
            .expect("clone classes")
            .iter()
            .any(|class| {
                class["signal_reason"] == "python_model_boilerplate"
                    && class["signal_role"] == "boilerplate"
            })
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
#[test]
fn tsx_wrapper_shells_do_not_raise_duplication_risk() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-duplication-jsx-wrapper");
    std::fs::create_dir_all(&root)?;
    write_project_file(
        &root,
        "rmu-quality-policy.json",
        r#"{"version":3,"thresholds":{"max_duplicate_block_count":0,"max_duplicate_density_bps":1}}"#,
    )?;
    let alpha = r#"export function AlphaCard() {
  return (
    <Card className="border">
      <CardHeader>
        <CardTitle>Alpha</CardTitle>
      </CardHeader>
      <CardContent>
        <Badge variant="secondary">Ready</Badge>
      </CardContent>
    </Card>
  )
}
"#;
    let beta = r#"export function BetaCard() {
  return (
    <Card className="border">
      <CardHeader>
        <CardTitle>Beta</CardTitle>
      </CardHeader>
      <CardContent>
        <Badge variant="secondary">Ready</Badge>
      </CardContent>
    </Card>
  )
}
"#;
    write_project_file(&root, "src/components/AlphaCard.tsx", alpha)?;
    write_project_file(&root, "src/components/BetaCard.tsx", beta)?;
    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;
    let conn = engine.open_db()?;
    for path in [
        "src/components/AlphaCard.tsx",
        "src/components/BetaCard.tsx",
    ] {
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
            "wrapper shell should stay info-only for {path}"
        );
    }

    let artifact: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join(".rmu/quality/duplication.clone_classes.json"))?)?;
    assert!(
        artifact["clone_classes"]
            .as_array()
            .expect("clone classes")
            .iter()
            .any(|class| class["signal_role"] == "boilerplate")
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
