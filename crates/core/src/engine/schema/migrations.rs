use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail};
use rusqlite::{Connection, Transaction, params};
use time::OffsetDateTime;

use super::backup::create_pre_migration_backup;
use super::table_ensure::{
    ensure_file_chunks_excerpt_column, ensure_file_graph_edges_table, ensure_file_quality_tables,
    ensure_files_artifact_fingerprint_columns, ensure_files_graph_count_columns,
    ensure_files_graph_edge_columns, ensure_files_graph_fingerprint_columns,
    ensure_files_source_mtime_column, ensure_refs_position_columns, ensure_schema_migrations_table,
    ensure_semantic_ann_buckets_table, ensure_symbols_position_columns,
};

#[derive(Clone, Copy)]
pub(super) struct SchemaMigration {
    pub(super) id: u32,
    pub(super) name: &'static str,
    pub(super) apply: fn(&Transaction<'_>) -> Result<()>,
}

pub(super) const MIGRATIONS: [SchemaMigration; 12] = [
    SchemaMigration {
        id: 1,
        name: "file_chunks_excerpt_column",
        apply: migration_file_chunks_excerpt,
    },
    SchemaMigration {
        id: 2,
        name: "semantic_ann_buckets_table",
        apply: migration_semantic_ann_buckets,
    },
    SchemaMigration {
        id: 3,
        name: "symbols_and_refs_position_columns",
        apply: migration_symbols_and_refs_positions,
    },
    SchemaMigration {
        id: 4,
        name: "files_source_mtime_column",
        apply: migration_files_source_mtime,
    },
    SchemaMigration {
        id: 5,
        name: "files_graph_count_columns",
        apply: migration_files_graph_counts,
    },
    SchemaMigration {
        id: 6,
        name: "files_graph_fingerprint_columns",
        apply: migration_files_graph_fingerprint,
    },
    SchemaMigration {
        id: 7,
        name: "files_artifact_fingerprint_columns",
        apply: migration_files_artifact_fingerprint,
    },
    SchemaMigration {
        id: 8,
        name: "file_graph_edges_table",
        apply: migration_file_graph_edges_table,
    },
    SchemaMigration {
        id: 9,
        name: "files_graph_edge_columns",
        apply: migration_files_graph_edge_columns,
    },
    SchemaMigration {
        id: 10,
        name: "file_quality_tables",
        apply: migration_file_quality_tables,
    },
    SchemaMigration {
        id: 11,
        name: "file_quality_metrics_and_columns",
        apply: migration_file_quality_metrics_and_columns,
    },
    SchemaMigration {
        id: 12,
        name: "file_rule_violation_locations",
        apply: migration_file_rule_violation_locations,
    },
];

pub(super) fn apply_schema_migrations(
    conn: &mut Connection,
    db_path: &Path,
    database_preexisted: bool,
) -> Result<()> {
    apply_schema_migrations_plan(conn, db_path, database_preexisted, &MIGRATIONS)
}

pub(super) fn apply_schema_migrations_plan(
    conn: &mut Connection,
    db_path: &Path,
    database_preexisted: bool,
    plan: &[SchemaMigration],
) -> Result<()> {
    validate_migration_plan(plan)?;
    ensure_schema_migrations_table(conn)?;
    reject_unknown_future_migrations(conn, plan)?;

    let applied = load_applied_migrations(conn)?;
    let pending = plan
        .iter()
        .copied()
        .filter(|migration| !applied.contains(&migration.id))
        .collect::<Vec<_>>();
    if pending.is_empty() {
        return Ok(());
    }

    if database_preexisted {
        let from = applied.iter().copied().max().unwrap_or(0);
        let to = pending.last().map(|migration| migration.id).unwrap_or(from);
        create_pre_migration_backup(conn, db_path, from, to)?;
    }

    for migration in pending {
        apply_single_migration(conn, migration)?;
    }
    Ok(())
}

fn apply_single_migration(conn: &mut Connection, migration: SchemaMigration) -> Result<()> {
    let tx = conn.transaction()?;
    (migration.apply)(&tx).with_context(|| {
        format!(
            "schema migration {} ({}) failed during apply",
            migration.id, migration.name
        )
    })?;
    let applied_at =
        OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?;
    tx.execute(
        "INSERT INTO schema_migrations(id, name, applied_at_utc)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO NOTHING",
        params![i64::from(migration.id), migration.name, applied_at],
    )?;
    tx.commit()?;
    Ok(())
}

fn validate_migration_plan(plan: &[SchemaMigration]) -> Result<()> {
    if plan.is_empty() {
        return Ok(());
    }
    let mut prev = 0_u32;
    for migration in plan {
        if migration.id <= prev {
            bail!(
                "migration ids must be strictly monotonic; got {} after {}",
                migration.id,
                prev
            );
        }
        prev = migration.id;
    }
    Ok(())
}

fn reject_unknown_future_migrations(conn: &Connection, plan: &[SchemaMigration]) -> Result<()> {
    let max_known = plan.last().map(|migration| migration.id).unwrap_or(0);
    let max_applied = conn.query_row("SELECT MAX(id) FROM schema_migrations", [], |row| {
        row.get::<_, Option<i64>>(0)
    })?;
    let Some(max_applied) = max_applied else {
        return Ok(());
    };
    let max_applied = u32::try_from(max_applied).map_err(|_| {
        anyhow!("schema_migrations contains non-u32 id `{max_applied}`; refusing to continue")
    })?;
    if max_applied > max_known {
        bail!(
            "database has migration id `{max_applied}` newer than binary supported `{max_known}`; silent downgrade is forbidden"
        );
    }
    Ok(())
}

fn load_applied_migrations(conn: &Connection) -> Result<HashSet<u32>> {
    let mut stmt = conn.prepare("SELECT id FROM schema_migrations")?;
    let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
    let mut out = HashSet::new();
    for row in rows {
        let id_raw = row?;
        let id = u32::try_from(id_raw)
            .map_err(|_| anyhow!("schema_migrations contains non-u32 id `{id_raw}`"))?;
        out.insert(id);
    }
    Ok(out)
}

fn migration_file_chunks_excerpt(tx: &Transaction<'_>) -> Result<()> {
    ensure_file_chunks_excerpt_column(tx)
}

fn migration_semantic_ann_buckets(tx: &Transaction<'_>) -> Result<()> {
    ensure_semantic_ann_buckets_table(tx)
}

fn migration_symbols_and_refs_positions(tx: &Transaction<'_>) -> Result<()> {
    ensure_symbols_position_columns(tx)?;
    ensure_refs_position_columns(tx)
}

fn migration_files_source_mtime(tx: &Transaction<'_>) -> Result<()> {
    ensure_files_source_mtime_column(tx)
}

fn migration_files_graph_counts(tx: &Transaction<'_>) -> Result<()> {
    ensure_files_graph_count_columns(tx)
}

fn migration_files_graph_fingerprint(tx: &Transaction<'_>) -> Result<()> {
    ensure_files_graph_fingerprint_columns(tx)
}

fn migration_files_artifact_fingerprint(tx: &Transaction<'_>) -> Result<()> {
    ensure_files_artifact_fingerprint_columns(tx)
}

fn migration_file_graph_edges_table(tx: &Transaction<'_>) -> Result<()> {
    ensure_file_graph_edges_table(tx)
}

fn migration_files_graph_edge_columns(tx: &Transaction<'_>) -> Result<()> {
    ensure_files_graph_edge_columns(tx)
}

fn migration_file_quality_tables(tx: &Transaction<'_>) -> Result<()> {
    ensure_file_quality_tables(tx)
}

fn migration_file_quality_metrics_and_columns(tx: &Transaction<'_>) -> Result<()> {
    ensure_file_quality_tables(tx)
}

fn migration_file_rule_violation_locations(tx: &Transaction<'_>) -> Result<()> {
    ensure_file_quality_tables(tx)
}
