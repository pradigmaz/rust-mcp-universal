use std::collections::HashMap;

use anyhow::Result;

use crate::engine::{Engine, storage};

pub(super) fn load_existing_file_state_read_only(
    engine: &Engine,
    semantic_model: &str,
) -> Result<HashMap<String, storage::ExistingFileState>> {
    if !engine.db_path.exists() {
        return Ok(HashMap::new());
    }

    let mut conn = engine.open_db_read_only()?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Deferred)?;
    let existing_files = storage::load_existing_file_state(&tx, semantic_model)?;
    tx.commit()?;
    Ok(existing_files)
}
