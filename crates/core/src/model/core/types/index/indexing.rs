use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::profiles::IndexProfile;
use crate::model::core::serde_glue;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    pub project_root: String,
    pub db_path: String,
    pub files: usize,
    pub symbols: usize,
    pub module_deps: usize,
    pub refs: usize,
    pub semantic_vectors: usize,
    pub file_chunks: usize,
    pub chunk_embeddings: usize,
    pub semantic_model: String,
    pub last_index_lock_wait_ms: u64,
    pub last_embedding_cache_hits: usize,
    pub last_embedding_cache_misses: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IgnoreInstallTarget {
    GitInfoExclude,
    RootGitignore,
}

impl IgnoreInstallTarget {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::GitInfoExclude => "git-info-exclude",
            Self::RootGitignore => "root-gitignore",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "git-info-exclude" => Some(Self::GitInfoExclude),
            "root-gitignore" => Some(Self::RootGitignore),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreInstallReport {
    pub target: IgnoreInstallTarget,
    pub path: String,
    pub created: bool,
    pub updated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexingOptions {
    #[serde(default)]
    pub profile: Option<IndexProfile>,
    #[serde(
        default,
        serialize_with = "serde_glue::serialize_optional_offset_datetime_rfc3339",
        deserialize_with = "serde_glue::deserialize_optional_offset_datetime_rfc3339"
    )]
    pub changed_since: Option<OffsetDateTime>,
    #[serde(default)]
    pub changed_since_commit: Option<String>,
    #[serde(default)]
    pub include_paths: Vec<String>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    #[serde(default)]
    pub reindex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopePreviewResult {
    #[serde(default)]
    pub profile: Option<IndexProfile>,
    #[serde(
        default,
        serialize_with = "serde_glue::serialize_optional_offset_datetime_rfc3339",
        deserialize_with = "serde_glue::deserialize_optional_offset_datetime_rfc3339"
    )]
    pub changed_since: Option<OffsetDateTime>,
    #[serde(default)]
    pub changed_since_commit: Option<String>,
    #[serde(default)]
    pub resolved_merge_base_commit: Option<String>,
    #[serde(default)]
    pub reindex: bool,
    #[serde(default)]
    pub include_paths: Vec<String>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    pub scanned_files: usize,
    pub candidate_count: usize,
    pub excluded_by_scope_count: usize,
    pub ignored_count: usize,
    pub skipped_before_changed_since_count: usize,
    pub repair_backfill_count: usize,
    pub deleted_count: usize,
    #[serde(default)]
    pub candidate_paths: Vec<String>,
    #[serde(default)]
    pub excluded_by_scope_paths: Vec<String>,
    #[serde(default)]
    pub ignored_paths: Vec<String>,
    #[serde(default)]
    pub skipped_before_changed_since_paths: Vec<String>,
    #[serde(default)]
    pub repair_backfill_paths: Vec<String>,
    #[serde(default)]
    pub deleted_paths: Vec<String>,
}
