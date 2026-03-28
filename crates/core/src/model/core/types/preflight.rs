use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreflightState {
    Ok,
    Warning,
    Incompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightStatus {
    pub status: PreflightState,
    pub project_path: String,
    pub binary_path: String,
    pub running_binary_version: String,
    pub running_binary_stale: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stale_process_probe_binary_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_schema_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db_schema_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_format_version: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ann_version: Option<u32>,
    #[serde(default)]
    pub same_binary_other_pids: Vec<u32>,
    pub stale_process_suspected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launcher_recommended: Option<String>,
    pub safe_recovery_hint: String,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub errors: Vec<String>,
}
