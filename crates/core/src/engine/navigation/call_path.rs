use anyhow::{Result, bail};

use super::super::Engine;
use super::call_path_search::find_call_path;
use crate::model::CallPathResult;

impl Engine {
    pub fn call_path(&self, from: &str, to: &str, max_hops: usize) -> Result<CallPathResult> {
        if max_hops == 0 {
            bail!("`max_hops` must be >= 1");
        }

        let conn = self.open_db()?;
        let from_endpoint = self.resolve_call_path_endpoint(&conn, from)?;
        let to_endpoint = self.resolve_call_path_endpoint(&conn, to)?;
        find_call_path(&conn, from_endpoint, to_endpoint, max_hops)
    }
}
