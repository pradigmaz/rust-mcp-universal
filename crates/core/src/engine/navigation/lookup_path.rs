use std::path::Path;

use anyhow::{Result, anyhow};

use super::super::Engine;
use super::validation::require_non_empty;
use crate::utils::normalize_path;

impl Engine {
    pub(crate) fn normalize_lookup_path(&self, path: &str) -> Result<String> {
        let raw = require_non_empty(path, "path")?;
        let input_path = Path::new(raw);
        if !input_path.is_absolute() {
            return Ok(normalize_path(input_path));
        }

        if let Ok(relative) = input_path.strip_prefix(&self.project_root) {
            return Ok(normalize_path(relative));
        }

        #[cfg(windows)]
        {
            if let (Ok(canonical_input), Ok(canonical_root)) =
                (input_path.canonicalize(), self.project_root.canonicalize())
            {
                if let Ok(relative) = canonical_input.strip_prefix(&canonical_root) {
                    return Ok(normalize_path(relative));
                }
            }
        }

        Err(anyhow!("path `{raw}` is outside project root"))
    }
}
