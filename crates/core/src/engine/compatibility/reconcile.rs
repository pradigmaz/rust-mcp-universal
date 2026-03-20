use anyhow::{Result, anyhow};
use rusqlite::{Connection, Transaction};

use super::meta_store::{
    count_files, read_meta_raw, read_meta_u32, upsert_meta_conn, upsert_meta_tx,
};
use super::types::expected_index_meta;
use super::{
    CURRENT_ANN_VERSION, CURRENT_INDEX_FORMAT_VERSION, CURRENT_SCHEMA_VERSION, LEGACY_ANN_VERSION,
    LEGACY_INDEX_FORMAT_VERSION, META_ANN_VERSION, META_EMBEDDING_DIM, META_EMBEDDING_MODEL_ID,
    META_INDEX_FORMAT_VERSION, META_SCHEMA_VERSION,
};

pub(crate) fn ensure_schema_preflight(conn: &Connection) -> Result<()> {
    let Some(schema_version) = read_meta_u32(conn, META_SCHEMA_VERSION)? else {
        return Ok(());
    };
    if schema_version > CURRENT_SCHEMA_VERSION {
        return Err(anyhow!(
            "database schema_version `{schema_version}` is newer than binary supported `{CURRENT_SCHEMA_VERSION}`; hard-fail to avoid unsafe writes"
        ));
    }
    Ok(())
}

pub(crate) fn reconcile_schema_and_index_meta(conn: &Connection) -> Result<()> {
    upsert_meta_conn(
        conn,
        META_SCHEMA_VERSION,
        &CURRENT_SCHEMA_VERSION.to_string(),
    )?;

    let files = count_files(conn)?;
    let expected = expected_index_meta();

    if read_meta_raw(conn, META_INDEX_FORMAT_VERSION)?.is_none() {
        let value = if files == 0 {
            expected.index_format_version
        } else {
            LEGACY_INDEX_FORMAT_VERSION
        };
        upsert_meta_conn(conn, META_INDEX_FORMAT_VERSION, &value.to_string())?;
    }

    if read_meta_raw(conn, META_ANN_VERSION)?.is_none() {
        let value = if files == 0 {
            expected.ann_version
        } else {
            LEGACY_ANN_VERSION
        };
        upsert_meta_conn(conn, META_ANN_VERSION, &value.to_string())?;
    }

    if read_meta_raw(conn, META_EMBEDDING_MODEL_ID)?.is_none() {
        let value = if files == 0 {
            expected.embedding_model_id
        } else {
            "unknown".to_string()
        };
        upsert_meta_conn(conn, META_EMBEDDING_MODEL_ID, &value)?;
    }

    if read_meta_raw(conn, META_EMBEDDING_DIM)?.is_none() {
        let value = if files == 0 {
            expected.embedding_dim
        } else {
            0
        };
        upsert_meta_conn(conn, META_EMBEDDING_DIM, &value.to_string())?;
    }

    Ok(())
}

pub(crate) fn write_index_identity_meta(
    tx: &Transaction<'_>,
    embedding_model_id: &str,
    embedding_dim: u32,
) -> Result<()> {
    upsert_meta_tx(tx, META_SCHEMA_VERSION, &CURRENT_SCHEMA_VERSION.to_string())?;
    upsert_meta_tx(
        tx,
        META_INDEX_FORMAT_VERSION,
        &CURRENT_INDEX_FORMAT_VERSION.to_string(),
    )?;
    upsert_meta_tx(tx, META_EMBEDDING_MODEL_ID, embedding_model_id)?;
    upsert_meta_tx(tx, META_EMBEDDING_DIM, &embedding_dim.to_string())?;
    upsert_meta_tx(tx, META_ANN_VERSION, &CURRENT_ANN_VERSION.to_string())?;
    Ok(())
}
