use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::model::{IndexedCounts, RebuildIndexResult};
use anyhow::Result;

use super::Engine;
use super::notes::{parse_note, scan_markdown_files};
use super::schema::ensure_schema;

pub(super) fn rebuild_index(engine: &Engine) -> Result<RebuildIndexResult> {
    let started_at = Instant::now();
    if let Some(parent) = engine.db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut errors = Vec::new();
    let files = scan_markdown_files(&engine.memory_root)?;
    let mut notes = Vec::new();
    for file in &files {
        match parse_note(&engine.memory_root, file) {
            Ok(note) => notes.push(note),
            Err(err) => errors.push(format!("{}: {err}", file.display())),
        }
    }

    let mut conn = engine.open_connection()?;
    ensure_schema(&conn)?;
    let tx = conn.transaction()?;
    tx.execute_batch(
        r#"
        DELETE FROM note_aliases;
        DELETE FROM note_tags;
        DELETE FROM note_observations;
        DELETE FROM note_references;
        DELETE FROM relations;
        DELETE FROM notes;
        DELETE FROM notes_fts;
        "#,
    )?;

    let mut counts = IndexedCounts::default();
    for note in &notes {
        tx.execute(
            "INSERT INTO notes(
                id, slug, title, node_type, status, project, file_path, summary, body,
                created_at, updated_at, raw_hash, file_mtime_ms
             ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            (
                &note.id,
                &note.slug,
                &note.title,
                &note.node_type,
                &note.status,
                &note.project,
                &note.file_path,
                &note.summary,
                &note.body,
                &note.created_at,
                &note.updated_at,
                &note.raw_hash,
                note.file_mtime_ms,
            ),
        )?;

        let alias_blob = note.aliases.join("\n");
        let tag_blob = note.tags.join("\n");
        let observation_blob = note.observations.join("\n");
        tx.execute(
            "INSERT INTO notes_fts(id, slug, title, summary, body, aliases, tags, observations)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                &note.id,
                &note.slug,
                &note.title,
                &note.summary,
                &note.body,
                &alias_blob,
                &tag_blob,
                &observation_blob,
            ),
        )?;

        for alias in &note.aliases {
            tx.execute(
                "INSERT INTO note_aliases(slug, alias) VALUES(?1, ?2)",
                (&note.slug, alias),
            )?;
            counts.aliases += 1;
        }

        for tag in &note.tags {
            tx.execute(
                "INSERT INTO note_tags(slug, tag) VALUES(?1, ?2)",
                (&note.slug, tag),
            )?;
            counts.tags += 1;
        }

        for observation in &note.observations {
            tx.execute(
                "INSERT INTO note_observations(slug, observation) VALUES(?1, ?2)",
                (&note.slug, observation),
            )?;
            counts.observations += 1;
        }

        for reference in &note.references {
            tx.execute(
                "INSERT INTO note_references(slug, reference) VALUES(?1, ?2)",
                (&note.slug, reference),
            )?;
        }

        for relation in &note.relations {
            tx.execute(
                "INSERT INTO relations(source_slug, target_slug, relation_kind)
                 VALUES(?1, ?2, ?3)",
                (&note.slug, &relation.target_slug, &relation.relation_kind),
            )?;
            counts.relations += 1;
        }

        counts.notes += 1;
        match note.node_type.as_str() {
            "decision" => counts.decisions += 1,
            "risk" => counts.risks += 1,
            "constraint" => counts.constraints += 1,
            "artifact" => counts.artifacts += 1,
            _ => {}
        }
    }

    let last_sync = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_millis()
        .to_string();
    tx.execute(
        "INSERT INTO meta(key, value) VALUES('last_sync', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [last_sync],
    )?;
    tx.execute(
        "INSERT INTO meta(key, value) VALUES('last_error_count', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [errors.len().to_string()],
    )?;
    tx.execute(
        "INSERT INTO meta(key, value) VALUES('last_indexed_file_count', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        [counts.notes.to_string()],
    )?;
    tx.commit()?;

    Ok(RebuildIndexResult {
        rebuilt: true,
        indexed_files: counts.notes,
        counts,
        duration_ms: started_at.elapsed().as_millis(),
        errors,
    })
}
