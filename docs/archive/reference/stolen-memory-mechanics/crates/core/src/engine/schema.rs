use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};

use super::CURRENT_SCHEMA_VERSION;

pub(super) fn ensure_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        "#,
    )?;
    let version = conn
        .query_row(
            "SELECT value FROM meta WHERE key = 'schema_version'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .and_then(|value| value.parse::<u32>().ok());

    if version != Some(CURRENT_SCHEMA_VERSION) {
        reset_schema(conn)?;
    }

    conn.execute(
        "INSERT INTO meta(key, value) VALUES ('schema_version', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [CURRENT_SCHEMA_VERSION.to_string()],
    )?;
    Ok(())
}

fn reset_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        DROP TABLE IF EXISTS notes_fts;
        DROP TABLE IF EXISTS note_observations;
        DROP TABLE IF EXISTS note_references;
        DROP TABLE IF EXISTS relations;
        DROP TABLE IF EXISTS note_tags;
        DROP TABLE IF EXISTS note_aliases;
        DROP TABLE IF EXISTS notes;
        DROP TABLE IF EXISTS meta;

        CREATE TABLE meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE notes (
            id TEXT NOT NULL UNIQUE,
            slug TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            node_type TEXT NOT NULL,
            status TEXT NOT NULL,
            project TEXT NOT NULL,
            file_path TEXT NOT NULL,
            summary TEXT NOT NULL,
            body TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            raw_hash TEXT NOT NULL,
            file_mtime_ms INTEGER NOT NULL
        );

        CREATE TABLE note_aliases (
            slug TEXT NOT NULL,
            alias TEXT NOT NULL
        );

        CREATE TABLE note_tags (
            slug TEXT NOT NULL,
            tag TEXT NOT NULL
        );

        CREATE TABLE note_observations (
            slug TEXT NOT NULL,
            observation TEXT NOT NULL
        );

        CREATE TABLE note_references (
            slug TEXT NOT NULL,
            reference TEXT NOT NULL
        );

        CREATE TABLE relations (
            source_slug TEXT NOT NULL,
            target_slug TEXT NOT NULL,
            relation_kind TEXT NOT NULL
        );

        CREATE VIRTUAL TABLE notes_fts USING fts5(
            id UNINDEXED,
            slug UNINDEXED,
            title,
            summary,
            body,
            aliases,
            tags,
            observations
        );
        "#,
    )?;
    Ok(())
}
