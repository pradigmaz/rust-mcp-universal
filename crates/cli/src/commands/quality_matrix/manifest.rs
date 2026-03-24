use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use rmu_core::{IndexProfile, QualityStatus};
use serde::{Deserialize, Serialize};

pub(crate) const CURRENT_QUALITY_MATRIX_MANIFEST_VERSION: u32 = 1;
const DEFAULT_ARTIFACT_BUNDLE: [&str; 1] = ["default"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QualityMatrixManifest {
    pub(crate) version: u32,
    pub(crate) repositories: Vec<QualityMatrixRepository>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct QualityMatrixRepository {
    pub(crate) id: String,
    pub(crate) role: String,
    pub(crate) path_key: String,
    pub(crate) required: bool,
    pub(crate) profile: IndexProfile,
    pub(crate) include_paths: Vec<String>,
    pub(crate) exclude_paths: Vec<String>,
    pub(crate) expected_languages: Vec<String>,
    pub(crate) size_class: String,
    pub(crate) expected_pre_refresh_statuses: Vec<QualityStatus>,
    pub(crate) expected_post_refresh_statuses: Vec<QualityStatus>,
    pub(crate) allowed_degradation_reasons: Vec<String>,
    pub(crate) artifact_bundle: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedQualityMatrixRepository {
    pub(crate) config: QualityMatrixRepository,
    pub(crate) project_path: PathBuf,
}

pub(crate) fn default_override_path(project_root: &Path) -> PathBuf {
    project_root.join(".codex/quality-matrix.local.json")
}

pub(crate) fn default_output_root(project_root: &Path) -> PathBuf {
    project_root.join(".codex/quality-matrix/runs")
}

pub(crate) fn resolve_from_project_root(project_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

pub(crate) fn canonical_summary_path(manifest_path: &Path) -> Result<PathBuf> {
    let Some(parent) = manifest_path.parent() else {
        bail!(
            "manifest path `{}` must have a parent directory",
            manifest_path.display()
        );
    };
    Ok(parent.join("baseline-summary.json"))
}

pub(crate) fn load_manifest(manifest_path: &Path) -> Result<QualityMatrixManifest> {
    let raw = fs::read(manifest_path)
        .with_context(|| format!("failed to read manifest `{}`", manifest_path.display()))?;
    let content = std::str::from_utf8(&raw)
        .with_context(|| format!("manifest `{}` is not valid UTF-8", manifest_path.display()))?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let manifest: QualityMatrixManifest = serde_json::from_str(content)
        .with_context(|| format!("failed to parse manifest `{}`", manifest_path.display()))?;
    validate_manifest(&manifest, manifest_path)?;
    Ok(manifest)
}

pub(crate) fn load_local_overrides(path: &Path) -> Result<HashMap<String, String>> {
    if !path.exists() {
        return Ok(HashMap::new());
    }
    let raw = fs::read(path)
        .with_context(|| format!("failed to read local override `{}`", path.display()))?;
    let content = std::str::from_utf8(&raw)
        .with_context(|| format!("local override `{}` is not valid UTF-8", path.display()))?;
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    serde_json::from_str(content)
        .with_context(|| format!("failed to parse local override `{}`", path.display()))
}

pub(crate) fn select_repositories(
    manifest: &QualityMatrixManifest,
    overrides: &HashMap<String, String>,
    repo_ids: &[String],
) -> Result<Vec<ResolvedQualityMatrixRepository>> {
    let selected_ids = repo_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if !selected_ids.is_empty() {
        let known = manifest
            .repositories
            .iter()
            .map(|repo| repo.id.as_str())
            .collect::<BTreeSet<_>>();
        if let Some(unknown) = selected_ids
            .iter()
            .find(|repo_id| !known.contains(**repo_id))
        {
            bail!("unknown quality-matrix repo id `{unknown}`");
        }
    }

    let mut resolved = Vec::new();
    for repo in &manifest.repositories {
        if !selected_ids.is_empty() && !selected_ids.contains(repo.id.as_str()) {
            continue;
        }
        let Some(raw_path) = overrides.get(&repo.path_key) else {
            if repo.required {
                bail!(
                    "missing local override for repo `{}` (path key `{}`)",
                    repo.id,
                    repo.path_key
                );
            }
            continue;
        };
        let project_path = PathBuf::from(raw_path);
        if !project_path.is_absolute() {
            bail!(
                "local override for repo `{}` must be an absolute path",
                repo.id
            );
        }
        if !project_path.exists() {
            bail!(
                "local override for repo `{}` points to missing path `{}`",
                repo.id,
                project_path.display()
            );
        }
        resolved.push(ResolvedQualityMatrixRepository {
            config: repo.clone(),
            project_path,
        });
    }
    resolved.sort_by(|left, right| left.config.id.cmp(&right.config.id));
    Ok(resolved)
}

fn validate_manifest(manifest: &QualityMatrixManifest, manifest_path: &Path) -> Result<()> {
    if manifest.version != CURRENT_QUALITY_MATRIX_MANIFEST_VERSION {
        bail!(
            "manifest `{}` declares unsupported version `{}`",
            manifest_path.display(),
            manifest.version
        );
    }
    let mut ids = BTreeSet::new();
    let mut path_keys = BTreeSet::new();
    for repo in &manifest.repositories {
        if repo.id.trim().is_empty() {
            bail!(
                "manifest `{}` contains empty repo id",
                manifest_path.display()
            );
        }
        if !ids.insert(repo.id.as_str()) {
            bail!(
                "manifest `{}` contains duplicate repo id `{}`",
                manifest_path.display(),
                repo.id
            );
        }
        if repo.path_key.trim().is_empty() {
            bail!(
                "manifest `{}` contains empty path_key for repo `{}`",
                manifest_path.display(),
                repo.id
            );
        }
        if !path_keys.insert(repo.path_key.as_str()) {
            bail!(
                "manifest `{}` contains duplicate path_key `{}`",
                manifest_path.display(),
                repo.path_key
            );
        }
        if repo.expected_pre_refresh_statuses.is_empty()
            || repo.expected_post_refresh_statuses.is_empty()
        {
            bail!(
                "manifest `{}` requires non-empty expected status lists for repo `{}`",
                manifest_path.display(),
                repo.id
            );
        }
        if repo.artifact_bundle.is_empty() {
            bail!(
                "manifest `{}` requires non-empty artifact_bundle for repo `{}`",
                manifest_path.display(),
                repo.id
            );
        }
        if repo.artifact_bundle.len() != DEFAULT_ARTIFACT_BUNDLE.len()
            || !repo
                .artifact_bundle
                .iter()
                .zip(DEFAULT_ARTIFACT_BUNDLE.iter())
                .all(|(actual, expected)| actual == expected)
        {
            bail!(
                "manifest `{}` requires artifact_bundle [\"default\"] for repo `{}`",
                manifest_path.display(),
                repo.id
            );
        }
    }
    Ok(())
}
