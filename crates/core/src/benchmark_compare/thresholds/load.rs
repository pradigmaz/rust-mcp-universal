use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use super::super::parsing::read_json_file;

pub(super) fn load_thresholds_value(path: &Path) -> Result<Value> {
    read_json_file(path, "thresholds")
}

pub(super) fn source_label(path: &Path) -> String {
    format!("thresholds file `{}`", path.display())
}
