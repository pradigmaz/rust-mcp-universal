use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::Result;

use super::policy::QualityPolicy;

#[path = "duplication/artifact.rs"]
pub(crate) mod artifact;
#[path = "duplication/candidates.rs"]
mod candidates;
#[path = "duplication/classify.rs"]
mod classify;
#[path = "duplication/collapse.rs"]
mod collapse;
#[path = "duplication/detect.rs"]
mod detect;
#[path = "duplication/facts.rs"]
mod facts;
#[path = "duplication/matching.rs"]
mod matching;
#[path = "duplication/semantics.rs"]
mod semantics;
#[path = "duplication/support.rs"]
mod support;
#[path = "duplication/surface.rs"]
mod surface;
#[path = "duplication/tokenize.rs"]
mod tokenize;

pub(crate) struct DuplicationAnalysis {
    pub(crate) file_facts: HashMap<String, crate::quality::DuplicationFacts>,
    pub(crate) artifact: artifact::DuplicationArtifact,
}

pub(crate) fn analyze_duplication(
    policy: &QualityPolicy,
    ruleset_id: &str,
    policy_digest: &str,
    candidates: &[candidates::DuplicationCandidate<'_>],
) -> DuplicationAnalysis {
    detect::analyze(policy, ruleset_id, policy_digest, candidates)
}

pub(crate) fn duplication_artifact_path(project_root: &Path) -> std::path::PathBuf {
    project_root.join(".rmu/quality/duplication.clone_classes.json")
}

pub(crate) fn write_duplication_artifact(
    project_root: &Path,
    artifact: &artifact::DuplicationArtifact,
) -> Result<()> {
    let path = duplication_artifact_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(artifact)?;
    fs::write(path, format!("{payload}\n"))?;
    Ok(())
}

pub(crate) use candidates::DuplicationCandidate;
