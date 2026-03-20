use std::error::Error;
use std::fs;

use rmu_core::{Engine, PrivacyMode, QueryOptions, SemanticFailMode};
use rusqlite::Connection;

use crate::common::{cleanup_project, setup_indexed_project, temp_project_dir};

include!("semantic_pipeline/bootstrap.rs");
include!("semantic_pipeline/ann_indexing.rs");
include!("semantic_pipeline/explainability.rs");
include!("semantic_pipeline/fail_modes.rs");
include!("semantic_pipeline/typescript_regressions.rs");
