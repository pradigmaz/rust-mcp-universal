use anyhow::{Context, Result, anyhow};
use rusqlite::OptionalExtension;

pub(in crate::engine) fn load_cached_chunk_embedding(
    tx: &rusqlite::Transaction<'_>,
    chunk_hash: &str,
    model: &str,
    expected_dim: usize,
) -> Result<Option<Vec<f32>>> {
    let cached = tx
        .query_row(
            "SELECT dim, vector_json FROM chunk_embeddings WHERE chunk_hash = ?1 AND model = ?2",
            rusqlite::params![chunk_hash, model],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;

    let Some((dim, raw)) = cached else {
        return Ok(None);
    };

    if usize::try_from(dim).ok() != Some(expected_dim) {
        return Err(anyhow!(
            "cached chunk embedding dimension mismatch for chunk `{chunk_hash}` model `{model}`: expected {expected_dim}, stored {dim}"
        ));
    }

    let vector = serde_json::from_str::<Vec<f32>>(&raw).with_context(|| {
        format!("failed to decode cached chunk embedding for chunk `{chunk_hash}` model `{model}`")
    })?;
    if vector.len() != expected_dim {
        return Err(anyhow!(
            "cached chunk embedding payload length mismatch for chunk `{chunk_hash}` model `{model}`: expected {expected_dim}, got {}",
            vector.len()
        ));
    }
    if let Some((idx, value)) = vector.iter().enumerate().find(|(_, v)| !v.is_finite()) {
        return Err(anyhow!(
            "cached chunk embedding contains non-finite value for chunk `{chunk_hash}` model `{model}` at index {idx}: {value}"
        ));
    }
    Ok(Some(vector))
}

#[cfg(test)]
mod tests {
    use super::load_cached_chunk_embedding;
    use rusqlite::{Connection, params};

    fn setup_chunk_embeddings(conn: &Connection) -> anyhow::Result<()> {
        conn.execute_batch(
            "CREATE TABLE chunk_embeddings (
                chunk_hash TEXT NOT NULL,
                model TEXT NOT NULL,
                dim INTEGER NOT NULL,
                vector_json TEXT NOT NULL,
                created_at_utc TEXT NOT NULL,
                PRIMARY KEY(chunk_hash, model)
            );",
        )?;
        Ok(())
    }

    #[test]
    fn cached_chunk_embedding_returns_none_when_missing() -> anyhow::Result<()> {
        let mut conn = Connection::open_in_memory()?;
        setup_chunk_embeddings(&conn)?;
        let tx = conn.transaction()?;

        let cached = load_cached_chunk_embedding(&tx, "missing", "model-a", 3)?;
        assert!(cached.is_none());
        Ok(())
    }

    #[test]
    fn cached_chunk_embedding_rejects_dimension_mismatch() -> anyhow::Result<()> {
        let mut conn = Connection::open_in_memory()?;
        setup_chunk_embeddings(&conn)?;
        conn.execute(
            "INSERT INTO chunk_embeddings(chunk_hash, model, dim, vector_json, created_at_utc)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                "chunk-1",
                "model-a",
                2_i64,
                "[0.1,0.2]",
                "2026-03-02T00:00:00Z"
            ],
        )?;
        let tx = conn.transaction()?;

        let err = load_cached_chunk_embedding(&tx, "chunk-1", "model-a", 3)
            .expect_err("must reject cached dim mismatch");
        assert!(err.to_string().contains("dimension mismatch"));
        Ok(())
    }

    #[test]
    fn cached_chunk_embedding_rejects_invalid_json_payload() -> anyhow::Result<()> {
        let mut conn = Connection::open_in_memory()?;
        setup_chunk_embeddings(&conn)?;
        conn.execute(
            "INSERT INTO chunk_embeddings(chunk_hash, model, dim, vector_json, created_at_utc)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                "chunk-2",
                "model-a",
                3_i64,
                "not-json",
                "2026-03-02T00:00:00Z"
            ],
        )?;
        let tx = conn.transaction()?;

        let err = load_cached_chunk_embedding(&tx, "chunk-2", "model-a", 3)
            .expect_err("must reject corrupted cached vector");
        assert!(
            err.to_string()
                .contains("failed to decode cached chunk embedding")
        );
        Ok(())
    }
}
