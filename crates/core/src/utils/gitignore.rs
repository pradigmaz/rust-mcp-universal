use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::gitignore::{Gitignore, GitignoreBuilder};

pub struct ProjectIgnoreMatcher {
    project_root: PathBuf,
    gitignore: Gitignore,
}

impl ProjectIgnoreMatcher {
    pub fn new(project_root: &Path) -> Result<Self> {
        let mut builder = GitignoreBuilder::new(project_root);
        let root_gitignore = project_root.join(".gitignore");
        if root_gitignore.is_file() {
            if let Some(err) = builder.add(&root_gitignore) {
                return Err(err)
                    .with_context(|| format!("failed to load {}", root_gitignore.display()));
            }
        }
        let gitignore = builder
            .build()
            .context("failed to build project .gitignore matcher")?;
        Ok(Self {
            project_root: project_root.to_path_buf(),
            gitignore,
        })
    }

    pub fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        let candidate = if path.is_absolute() {
            path.strip_prefix(&self.project_root)
                .unwrap_or(path)
                .to_path_buf()
        } else {
            path.to_path_buf()
        };
        self.gitignore
            .matched_path_or_any_parents(&candidate, is_dir)
            .is_ignore()
    }
}
