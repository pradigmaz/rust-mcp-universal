use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::OptionalExtension;
use time::{Duration, OffsetDateTime};

use super::Engine;
use crate::model::{
    PrivacyMode, QualityMode, QueryOptions, RuleViolationsOptions, SemanticFailMode,
};

mod duplication_precision;
mod duplication_semantics;
mod duplication_signal_roles;
mod hotspots;
mod oversize_and_backfill;
mod structural;
mod summary_rules;
mod wave4_security;

fn temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock must be monotonic")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{nanos}"))
}

fn write_project_file(root: &Path, relative: &str, contents: &str) -> anyhow::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}

fn repeated_lines(prefix: &str, count: usize) -> String {
    (0..count)
        .map(|idx| format!("{prefix}_{idx}\n"))
        .collect::<String>()
}
