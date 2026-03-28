use anyhow::{Result, bail};
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct DuplicationPolicyFile {
    #[serde(default)]
    pub(crate) suppressions: Vec<DuplicationSuppressionFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DuplicationSuppressionFile {
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) clone_class_ids: Vec<String>,
    #[serde(default)]
    pub(crate) path_pairs: Vec<DuplicationPathPairFile>,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct DuplicationPathPairFile {
    pub(crate) left: String,
    pub(crate) right: String,
}

pub(crate) fn validate_duplication_suppressions(
    policy_path: &std::path::Path,
    scope_id: Option<&str>,
    suppressions: &[DuplicationSuppressionFile],
) -> Result<()> {
    let mut ids = std::collections::BTreeSet::new();
    for suppression in suppressions {
        let id = suppression.id.trim();
        if id.is_empty() {
            bail_with_scope(
                policy_path,
                scope_id,
                "contains a duplication suppression with an empty `id`",
            )?;
        }
        if !ids.insert(id.to_string()) {
            bail_with_scope(
                policy_path,
                scope_id,
                &format!("declares duplicate duplication suppression `{id}`"),
            )?;
        }
        if suppression.reason.trim().is_empty() {
            bail_with_scope(
                policy_path,
                scope_id,
                &format!("declares duplication suppression `{id}` without a reason"),
            )?;
        }
        if suppression.clone_class_ids.is_empty() && suppression.path_pairs.is_empty() {
            bail_with_scope(
                policy_path,
                scope_id,
                &format!(
                    "declares duplication suppression `{id}` without any `clone_class_ids` or `path_pairs`"
                ),
            )?;
        }
        for pair in &suppression.path_pairs {
            if pair.left.trim().is_empty() || pair.right.trim().is_empty() {
                bail_with_scope(
                    policy_path,
                    scope_id,
                    &format!(
                        "declares duplication suppression `{id}` with an empty `path_pairs` side"
                    ),
                )?;
            }
        }
    }
    Ok(())
}

fn bail_with_scope(
    policy_path: &std::path::Path,
    scope_id: Option<&str>,
    message: &str,
) -> Result<()> {
    match scope_id {
        Some(scope_id) => bail!(
            "quality policy `{}` path scope `{scope_id}` {message}",
            policy_path.display()
        ),
        None => bail!("quality policy `{}` {message}", policy_path.display()),
    }
}
