use std::fs;

use anyhow::Result;

use super::Engine;
use crate::model::{
    DbCheckpointResult, DbMaintenanceOptions, DbMaintenanceResult, DbMaintenanceStats,
};

impl Engine {
    pub fn db_maintenance(&self, options: DbMaintenanceOptions) -> Result<DbMaintenanceResult> {
        let options = options.normalized();
        let conn = self.open_db()?;

        let (integrity_ok, integrity_message) = if options.integrity_check {
            let result: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
            (Some(result.eq_ignore_ascii_case("ok")), Some(result))
        } else {
            (None, None)
        };

        let checkpoint = if options.checkpoint {
            let (busy, wal_pages, checkpointed_pages): (i64, i64, i64) =
                conn.query_row("PRAGMA wal_checkpoint(PASSIVE)", [], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })?;
            Some(DbCheckpointResult {
                busy,
                wal_pages,
                checkpointed_pages,
            })
        } else {
            None
        };

        if options.analyze {
            conn.execute_batch("ANALYZE;")?;
        }
        if options.vacuum {
            conn.execute_batch("VACUUM;")?;
        }

        let prune = if options.prune {
            Some(self.cleanup_stale_indexes()?)
        } else {
            None
        };

        let stats = if options.stats {
            let page_size: i64 = conn.query_row("PRAGMA page_size", [], |row| row.get(0))?;
            let page_count: i64 = conn.query_row("PRAGMA page_count", [], |row| row.get(0))?;
            let freelist_count: i64 =
                conn.query_row("PRAGMA freelist_count", [], |row| row.get(0))?;
            let db_size_bytes = fs::metadata(&self.db_path)
                .map(|meta| meta.len())
                .unwrap_or(0);
            let wal_size_bytes = fs::metadata(format!("{}-wal", self.db_path.display()))
                .map(|meta| meta.len())
                .unwrap_or(0);
            let shm_size_bytes = fs::metadata(format!("{}-shm", self.db_path.display()))
                .map(|meta| meta.len())
                .unwrap_or(0);
            Some(DbMaintenanceStats {
                page_size,
                page_count,
                freelist_count,
                approx_free_bytes: (page_size.max(0) as u64) * (freelist_count.max(0) as u64),
                db_size_bytes,
                wal_size_bytes,
                shm_size_bytes,
                total_size_bytes: db_size_bytes
                    .saturating_add(wal_size_bytes)
                    .saturating_add(shm_size_bytes),
            })
        } else {
            None
        };

        Ok(DbMaintenanceResult {
            db_path: self.db_path.display().to_string(),
            options,
            integrity_ok,
            integrity_message,
            checkpoint,
            vacuum_ran: options.vacuum,
            analyze_ran: options.analyze,
            stats,
            prune,
        })
    }
}
