use serde::Serialize;

use crate::model::QualitySuppression;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DuplicationSignalRole {
    Primary,
    Downweighted,
    Boilerplate,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DuplicationArtifact {
    pub(crate) version: u32,
    pub(crate) ruleset_id: String,
    pub(crate) policy_digest: String,
    pub(crate) generated_at_utc: String,
    pub(crate) clone_classes: Vec<DuplicationCloneClass>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) suppressed_clone_classes: Vec<SuppressedDuplicationCloneClass>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DuplicationCloneClass {
    pub(crate) clone_class_id: String,
    pub(crate) language: String,
    pub(crate) corpus_class: String,
    pub(crate) normalized_token_count: usize,
    pub(crate) similarity_percent: i64,
    #[serde(default, skip_serializing_if = "is_primary_role")]
    pub(crate) signal_role: DuplicationSignalRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) signal_reason: Option<String>,
    pub(crate) same_file: bool,
    pub(crate) cross_file: bool,
    pub(crate) members: Vec<DuplicationCloneMember>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct DuplicationCloneMember {
    pub(crate) path: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    pub(crate) token_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct SuppressedDuplicationCloneClass {
    pub(crate) clone_class: DuplicationCloneClass,
    pub(crate) suppressions: Vec<QualitySuppression>,
}

const fn is_primary_role(role: &DuplicationSignalRole) -> bool {
    matches!(role, DuplicationSignalRole::Primary)
}
