use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::model::StorageMode;
use crate::utils::hash_bytes;

const CODEX_HOME_ENV: &str = "CODEX_HOME";
const PROJECT_MEMORY_DIR: &str = "memory";
const LEGACY_PROJECT_MEMORY_DIR: &str = ".memory";
const CODEX_MEMORY_ROOT: &str = "memory";
const PROJECT_BINDING_DIR: &str = ".codex";
const PROJECT_BINDING_FILE: &str = "project-memory.json";
const PROJECT_BINDING_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ProjectBindingMarker {
    schema_version: u32,
    project_slug: String,
    project_id: String,
    project_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContext {
    pub project_root: PathBuf,
    pub memory_root: PathBuf,
    pub storage_mode: StorageMode,
    pub project_slug: String,
    pub project_key: String,
}

impl ProjectContext {
    pub fn resolve(project_root: impl AsRef<Path>, storage_mode: StorageMode) -> Result<Self> {
        let project_root = std::fs::canonicalize(project_root.as_ref())?;
        let derived_project_slug = normalize_slug(
            project_root
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("memory-project"),
        )
        .unwrap_or_else(|| "memory-project".to_string());
        let marker = load_project_binding_marker(&project_root)?;
        let project_key = marker
            .as_ref()
            .map(|marker| marker.project_key.clone())
            .unwrap_or_else(|| derive_project_key(&project_root, &derived_project_slug));
        let project_slug = marker
            .as_ref()
            .map(|marker| marker.project_slug.clone())
            .unwrap_or(derived_project_slug);
        let memory_root = match storage_mode {
            StorageMode::Codex => codex_memory_root(&codex_home()?, &project_key),
            StorageMode::Project => project_memory_root(&project_root),
        };
        Ok(Self {
            project_root,
            memory_root,
            storage_mode,
            project_slug,
            project_key,
        })
    }

    pub fn ensure_project_binding_marker(&self) -> Result<()> {
        let marker_path = project_binding_marker_path(&self.project_root);
        let expected_marker = ProjectBindingMarker {
            schema_version: PROJECT_BINDING_SCHEMA_VERSION,
            project_slug: self.project_slug.clone(),
            project_id: project_id_from_key(&self.project_key).to_string(),
            project_key: self.project_key.clone(),
        };
        if let Some(existing) = load_project_binding_marker(&self.project_root)?
            && existing == expected_marker
        {
            return Ok(());
        }
        if let Some(parent) = marker_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::write(
            &marker_path,
            serde_json::to_string_pretty(&expected_marker)?,
        )
        .with_context(|| format!("failed to write {}", marker_path.display()))?;
        Ok(())
    }

    pub fn project_name(&self) -> String {
        self.project_root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("memory-project")
            .to_string()
    }
}

pub fn project_memory_root(project_root: &Path) -> PathBuf {
    project_root.join(PROJECT_MEMORY_DIR)
}

pub fn legacy_project_memory_root(project_root: &Path) -> PathBuf {
    project_root.join(LEGACY_PROJECT_MEMORY_DIR)
}

pub fn project_binding_marker_path(project_root: &Path) -> PathBuf {
    project_root
        .join(PROJECT_BINDING_DIR)
        .join(PROJECT_BINDING_FILE)
}

fn codex_memory_root(codex_home: &Path, project_key: &str) -> PathBuf {
    codex_home.join(CODEX_MEMORY_ROOT).join(project_key)
}

pub fn codex_home() -> Result<PathBuf> {
    if let Some(path) = env::var_os(CODEX_HOME_ENV) {
        return Ok(PathBuf::from(path));
    }
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .or_else(
            || match (env::var_os("HOMEDRIVE"), env::var_os("HOMEPATH")) {
                (Some(drive), Some(path)) => {
                    let mut combined = PathBuf::from(drive);
                    combined.push(path);
                    Some(combined.into_os_string())
                }
                _ => None,
            },
        )
        .context("failed to resolve CODEX_HOME fallback from user home directory")?;
    Ok(PathBuf::from(home).join(".codex"))
}

fn normalize_slug(raw: &str) -> Option<String> {
    let mut slug = String::with_capacity(raw.len());
    let mut last_was_dash = false;
    for character in raw.chars() {
        let next = if character.is_ascii_alphanumeric() {
            character.to_ascii_lowercase()
        } else if matches!(character, '-' | '_' | ' ') {
            '-'
        } else {
            continue;
        };
        if next == '-' {
            if slug.is_empty() || last_was_dash {
                continue;
            }
            last_was_dash = true;
        } else {
            last_was_dash = false;
        }
        slug.push(next);
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    if slug.is_empty() { None } else { Some(slug) }
}

fn derive_project_key(project_root: &Path, project_slug: &str) -> String {
    let project_hash = hash_bytes(project_root.to_string_lossy().as_bytes());
    format!("{project_slug}--{}", &project_hash[..10])
}

fn project_id_from_key(project_key: &str) -> &str {
    project_key
        .split_once("--")
        .map(|(_, project_id)| project_id)
        .unwrap_or(project_key)
}

fn load_project_binding_marker(project_root: &Path) -> Result<Option<ProjectBindingMarker>> {
    let marker_path = project_binding_marker_path(project_root);
    if !marker_path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&marker_path)
        .with_context(|| format!("failed to read {}", marker_path.display()))?;
    let marker = serde_json::from_str::<ProjectBindingMarker>(&raw)
        .with_context(|| format!("failed to parse {}", marker_path.display()))?;
    validate_project_binding_marker(&marker, &marker_path)?;
    Ok(Some(marker))
}

fn validate_project_binding_marker(
    marker: &ProjectBindingMarker,
    marker_path: &Path,
) -> Result<()> {
    if marker.schema_version != PROJECT_BINDING_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported schema_version `{}` in {}",
            marker.schema_version,
            marker_path.display()
        );
    }
    if marker.project_slug.trim().is_empty()
        || marker.project_id.trim().is_empty()
        || marker.project_key.trim().is_empty()
    {
        anyhow::bail!(
            "project binding marker in {} must be non-empty",
            marker_path.display()
        );
    }
    if !marker.project_key.ends_with(&marker.project_id) {
        anyhow::bail!(
            "project binding marker in {} has inconsistent project_id/project_key",
            marker_path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ProjectContext, codex_memory_root, project_binding_marker_path};
    use crate::model::StorageMode;

    #[test]
    fn project_mode_uses_visible_memory_directory() {
        let root = tempfile_dir("project-mode");
        let context = ProjectContext::resolve(&root, StorageMode::Project).expect("context");

        assert_eq!(context.memory_root, root.join("memory"));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn codex_mode_uses_flat_codex_memory_directory() {
        let root = tempfile_dir("codex-mode");
        let codex_home = tempfile_dir("codex-home");
        let project_root = std::fs::canonicalize(&root).expect("canonical root");
        let project_slug = project_root
            .file_name()
            .and_then(|value| value.to_str())
            .expect("project slug");
        let project_hash = crate::utils::hash_bytes(project_root.to_string_lossy().as_bytes());
        let project_key = format!("{project_slug}--{}", &project_hash[..10]);

        assert_eq!(
            codex_memory_root(&codex_home, &project_key),
            codex_home.join("memory").join(project_key)
        );

        let _ = std::fs::remove_dir_all(root);
        let _ = std::fs::remove_dir_all(codex_home);
    }

    #[test]
    fn codex_mode_prefers_stable_project_binding_marker() {
        let root = tempfile_dir("codex-bound");
        let marker_path = project_binding_marker_path(&root);
        std::fs::create_dir_all(marker_path.parent().expect("marker dir")).expect("marker dir");
        std::fs::write(
            &marker_path,
            r#"{
  "schema_version": 1,
  "project_slug": "obsidian-mcp-memory",
  "project_id": "0638380514",
  "project_key": "obsidian-mcp-memory--0638380514"
}"#,
        )
        .expect("marker");

        let context = ProjectContext::resolve(&root, StorageMode::Codex).expect("context");
        assert_eq!(context.project_key, "obsidian-mcp-memory--0638380514");
        assert!(
            context
                .memory_root
                .ends_with("obsidian-mcp-memory--0638380514")
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn ensure_project_binding_marker_persists_stable_key() {
        let root = tempfile_dir("codex-marker-write");
        let context = ProjectContext::resolve(&root, StorageMode::Project).expect("context");

        context
            .ensure_project_binding_marker()
            .expect("persist marker");

        let marker = std::fs::read_to_string(project_binding_marker_path(&root)).expect("marker");
        assert!(marker.contains("\"project_key\""));
        assert!(marker.contains(&context.project_key));

        let _ = std::fs::remove_dir_all(root);
    }

    fn tempfile_dir(prefix: &str) -> std::path::PathBuf {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("{prefix}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create root");
        std::fs::canonicalize(root).expect("canonical root")
    }
}
