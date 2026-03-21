use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};

use crate::model::{IgnoreInstallReport, IgnoreInstallTarget};

use super::{
    ManagedFileUpdate, merge_managed_block, render_managed_block, resolve_git_repo_context,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GitignoreUpdate {
    pub created: bool,
    pub updated: bool,
}

pub fn install_ignore_rules(
    project_root: &Path,
    target: IgnoreInstallTarget,
) -> Result<IgnoreInstallReport> {
    let path = match target {
        IgnoreInstallTarget::RootGitignore => project_root.join(".gitignore"),
        IgnoreInstallTarget::GitInfoExclude => resolve_git_repo_context(project_root)?
            .map(|context| context.info_exclude_path)
            .ok_or_else(|| {
                anyhow!(
                    "install-ignore-rules target `git-info-exclude` requires a git repository; use `root-gitignore` instead"
                )
            })?,
    };
    let update = install_managed_block(&path)?;
    Ok(IgnoreInstallReport {
        target,
        path: path.display().to_string(),
        created: update.created,
        updated: update.updated,
        warning: None,
    })
}

pub fn ensure_root_gitignore(project_root: &Path) -> Result<GitignoreUpdate> {
    let report = install_ignore_rules(project_root, IgnoreInstallTarget::RootGitignore)?;
    Ok(GitignoreUpdate {
        created: report.created,
        updated: report.updated,
    })
}

fn install_managed_block(path: &Path) -> Result<ManagedFileUpdate> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    if !path.exists() {
        fs::write(path, render_managed_block("\n"))
            .with_context(|| format!("failed to create {}", path.display()))?;
        return Ok(ManagedFileUpdate {
            created: true,
            updated: true,
        });
    }

    let existing =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let (next, updated) = merge_managed_block(&existing);
    if updated {
        fs::write(path, next).with_context(|| format!("failed to update {}", path.display()))?;
    }

    Ok(ManagedFileUpdate {
        created: false,
        updated,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::model::IgnoreInstallTarget;

    use super::{ensure_root_gitignore, install_ignore_rules};
    use crate::utils::gitignore::merge_managed_block;

    #[test]
    fn creates_gitignore_with_managed_block_when_missing() {
        let root = temp_dir("gitignore-create");
        fs::create_dir_all(&root).expect("create temp root");

        let update = ensure_root_gitignore(&root).expect("create gitignore");
        assert!(update.created);
        assert!(update.updated);

        let gitignore = fs::read_to_string(root.join(".gitignore")).expect("read gitignore");
        assert!(gitignore.contains(".rmu/"));
        assert!(gitignore.contains(".codex/"));

        cleanup(&root);
    }

    #[test]
    fn appends_managed_block_without_removing_existing_rules() {
        let existing = "target/\n.env\n";
        let (merged, updated) = merge_managed_block(existing);
        assert!(updated);
        assert!(merged.starts_with("target/\n.env\n\n"));
        assert!(merged.contains(".rmu/"));
        assert!(merged.contains(".idea/"));
    }

    #[test]
    fn replaces_existing_managed_block_in_place() {
        let existing = concat!(
            "target/\n\n",
            "# --- RMU managed ignore block: start ---\n",
            ".rmu/\n",
            "# --- RMU managed ignore block: end ---\n\n",
            ".env\n",
        );
        let (merged, updated) = merge_managed_block(existing);
        assert!(updated);
        assert_eq!(
            merged
                .matches("# --- RMU managed ignore block: start ---")
                .count(),
            1
        );
        assert!(merged.contains(".codex/"));
        assert!(merged.ends_with(".env\n"));
    }

    #[test]
    fn installs_git_info_exclude_when_git_directory_exists() {
        let root = temp_dir("git-info-exclude");
        fs::create_dir_all(root.join(".git/info")).expect("create git info dir");

        let report = install_ignore_rules(&root, IgnoreInstallTarget::GitInfoExclude)
            .expect("install git info exclude");
        assert!(report.created);
        assert!(report.updated);
        assert_eq!(report.target, IgnoreInstallTarget::GitInfoExclude);

        let exclude = fs::read_to_string(root.join(".git/info/exclude")).expect("read exclude");
        assert!(exclude.contains(".rmu/"));
        assert!(exclude.contains(".vscode/"));

        cleanup(&root);
    }

    #[test]
    fn git_info_exclude_requires_git_repository() {
        let root = temp_dir("git-info-exclude-missing");
        fs::create_dir_all(&root).expect("create temp root");

        let err = install_ignore_rules(&root, IgnoreInstallTarget::GitInfoExclude)
            .expect_err("missing git repo must fail");
        assert!(err.to_string().contains("git-info-exclude"));

        cleanup(&root);
    }

    fn temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should advance")
            .as_nanos();
        std::env::temp_dir().join(format!("rmu-core-{prefix}-{unique}"))
    }

    fn cleanup(path: &PathBuf) {
        let _ = fs::remove_dir_all(path);
    }
}
