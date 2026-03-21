use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use super::resolve_git_repo_context;

struct IgnoreRuleMatcher {
    base_path: PathBuf,
    matcher: Gitignore,
}

pub struct ProjectIgnoreMatcher {
    project_root: PathBuf,
    matchers: Vec<IgnoreRuleMatcher>,
}

impl ProjectIgnoreMatcher {
    pub fn new(project_root: &Path) -> Result<Self> {
        let mut matchers = Vec::new();

        if let Some(project_gitignore) =
            build_matcher(project_root, project_root, project_root.join(".gitignore"))?
        {
            matchers.push(project_gitignore);
        }

        if let Some(context) = resolve_git_repo_context(project_root)? {
            if let Some(git_info_exclude) =
                build_matcher(project_root, &context.repo_root, context.info_exclude_path)?
            {
                matchers.push(git_info_exclude);
            }
        }

        Ok(Self {
            project_root: project_root.to_path_buf(),
            matchers,
        })
    }

    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        let absolute_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.project_root.join(path)
        };

        self.matchers.iter().any(|rule| {
            absolute_path
                .strip_prefix(&rule.base_path)
                .ok()
                .is_some_and(|candidate| {
                    rule.matcher
                        .matched_path_or_any_parents(candidate, is_dir)
                        .is_ignore()
                })
        })
    }
}

fn build_matcher(
    project_root: &Path,
    base_path: &Path,
    rules_path: PathBuf,
) -> Result<Option<IgnoreRuleMatcher>> {
    if !rules_path.is_file() {
        return Ok(None);
    }

    let mut builder = GitignoreBuilder::new(base_path);
    if let Some(err) = builder.add(&rules_path) {
        return Err(err).with_context(|| {
            format!(
                "failed to load ignore rules from {} for project {}",
                rules_path.display(),
                project_root.display()
            )
        });
    }
    let matcher = builder.build().with_context(|| {
        format!(
            "failed to build ignore matcher from {}",
            rules_path.display()
        )
    })?;

    Ok(Some(IgnoreRuleMatcher {
        base_path: base_path.to_path_buf(),
        matcher,
    }))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::ProjectIgnoreMatcher;

    #[test]
    fn matcher_loads_root_gitignore_rules() {
        let root = temp_dir("matcher-root");
        fs::create_dir_all(&root).expect("create temp root");
        fs::write(root.join(".gitignore"), "generated/\n").expect("write gitignore");

        let matcher = ProjectIgnoreMatcher::new(&root).expect("build matcher");
        assert!(matcher.is_ignored(PathBuf::from("generated/file.txt").as_path(), false));

        cleanup(&root);
    }

    #[test]
    fn matcher_loads_git_info_exclude_rules() {
        let root = temp_dir("matcher-git-info");
        fs::create_dir_all(root.join(".git/info")).expect("create git info dir");
        fs::write(root.join(".git/info/exclude"), "scratch/\n").expect("write exclude");

        let matcher = ProjectIgnoreMatcher::new(&root).expect("build matcher");
        assert!(matcher.is_ignored(PathBuf::from("scratch/file.txt").as_path(), false));

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
