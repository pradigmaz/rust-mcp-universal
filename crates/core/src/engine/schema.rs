use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

#[path = "schema/backup.rs"]
mod backup;
#[path = "schema/constants.rs"]
mod constants;
#[path = "schema/migrations.rs"]
mod migrations;
#[path = "schema/table_ensure.rs"]
mod table_ensure;

pub(super) const INIT_DB_SCHEMA_SQL: &str = constants::INIT_DB_SCHEMA_SQL;
pub(super) const OPEN_DB_PRAGMAS_SQL: &str = constants::OPEN_DB_PRAGMAS_SQL;
pub(super) const OPEN_DB_READ_ONLY_PRAGMAS_SQL: &str = constants::OPEN_DB_READ_ONLY_PRAGMAS_SQL;
pub(crate) const CURRENT_SCHEMA_MIGRATION_VERSION: u32 =
    migrations::CURRENT_SCHEMA_MIGRATION_VERSION;

pub(crate) fn required_schema_exists(conn: &Connection) -> Result<bool> {
    table_ensure::required_schema_exists(conn)
}

pub(super) fn apply_schema_migrations(
    conn: &mut Connection,
    db_path: &Path,
    database_preexisted: bool,
) -> Result<()> {
    migrations::apply_schema_migrations(conn, db_path, database_preexisted)
}

#[cfg(test)]
use migrations::SchemaMigration;

#[cfg(test)]
const MIGRATIONS: [SchemaMigration; 15] = migrations::MIGRATIONS;

#[cfg(test)]
fn apply_schema_migrations_plan(
    conn: &mut Connection,
    db_path: &Path,
    database_preexisted: bool,
    plan: &[SchemaMigration],
) -> Result<()> {
    migrations::apply_schema_migrations_plan(conn, db_path, database_preexisted, plan)
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "schema_quality_tests.rs"]
mod quality_tests;
