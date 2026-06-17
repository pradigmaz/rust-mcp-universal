use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use rusqlite::Connection;

use crate::model::{IndexInconsistencies, IndexStatus, IndexedCounts, MemoryStatus};

use super::Engine;
use super::canonical::parse_canonical_note;
use super::notes::scan_markdown_files;
use super::query::{read_counts, read_meta};

pub(super) fn index_status(engine: &Engine) -> Result<IndexStatus> {
    let scan = scan_current_markdown_state(&engine.memory_root)?;
    if !engine.db_path.exists() {
        let inconsistencies = IndexInconsistencies {
            missing_from_index: scan
                .parsed
                .values()
                .map(|note| fingerprint_label(&note.slug, &note.file_path))
                .collect(),
            orphaned_in_index: Vec::new(),
            stale_fingerprints: Vec::new(),
            parse_failures: scan.parse_failures.clone(),
        };
        let mut failures = summarize_inconsistencies(&inconsistencies);
        if failures.is_empty() {
            failures.push("index has not been built yet".to_string());
        }
        return Ok(IndexStatus {
            project: project_name(engine),
            project_root: engine.project_root.display().to_string(),
            memory_root: engine.memory_root.display().to_string(),
            storage_mode: engine.storage_mode,
            db_path: engine.db_path.display().to_string(),
            schema_version: None,
            indexed: false,
            counts: IndexedCounts::default(),
            last_sync: None,
            drift_detected: scan.total_markdown_files > 0,
            fingerprint_drift_detected: false,
            pending_markdown_files: scan.total_markdown_files,
            inconsistencies,
            failures,
        });
    }

    let conn = engine.open_connection()?;
    let counts = read_counts(&conn)?;
    let last_sync = read_meta(&conn, "last_sync")?;
    let schema_version = read_meta(&conn, "schema_version")?.and_then(|value| value.parse().ok());
    let last_error_count = read_meta(&conn, "last_error_count")?
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or_default();
    let indexed = read_indexed_fingerprints(&conn)?;
    let inconsistencies = diff_fingerprints(&scan.parsed, &indexed, &scan.parse_failures);
    let fingerprint_drift_detected = !inconsistencies.stale_fingerprints.is_empty();
    let drift_detected = !inconsistencies.is_empty();
    let pending_markdown_files =
        inconsistencies.missing_from_index.len() + inconsistencies.parse_failures.len();

    let mut failures = summarize_inconsistencies(&inconsistencies);
    if last_error_count > 0 {
        failures.push(format!(
            "last rebuild reported {last_error_count} parse error(s)"
        ));
    }
    if scan.total_markdown_files != counts.notes {
        failures.push("markdown file count differs from indexed note count".to_string());
    }

    Ok(IndexStatus {
        project: project_name(engine),
        project_root: engine.project_root.display().to_string(),
        memory_root: engine.memory_root.display().to_string(),
        storage_mode: engine.storage_mode,
        db_path: engine.db_path.display().to_string(),
        schema_version,
        indexed: true,
        counts,
        last_sync,
        drift_detected,
        fingerprint_drift_detected,
        pending_markdown_files,
        inconsistencies,
        failures,
    })
}

pub(super) fn memory_status(engine: &Engine) -> Result<MemoryStatus> {
    let index = index_status(engine)?;
    let parser_health = if index.inconsistencies.parse_failures.is_empty() {
        "ok"
    } else {
        "warning"
    };
    let index_health = if !index.indexed {
        "empty"
    } else if index.drift_detected || !index.failures.is_empty() {
        "warning"
    } else {
        "ok"
    };
    let health = if !index.indexed {
        "empty"
    } else if parser_health != "ok" || index_health != "ok" {
        "warning"
    } else {
        "ok"
    };
    let issues = index.failures.iter().take(5).cloned().collect::<Vec<_>>();
    let recommended_action = if !index.indexed {
        "run rebuild_index before using project memory".to_string()
    } else if index.drift_detected || !index.failures.is_empty() {
        "run rebuild_index to resync Markdown truth into SQLite".to_string()
    } else {
        "memory is ready".to_string()
    };

    Ok(MemoryStatus {
        project: index.project.clone(),
        project_root: index.project_root.clone(),
        memory_root: index.memory_root.clone(),
        storage_mode: index.storage_mode,
        health: health.to_string(),
        counts: index.counts,
        last_sync: index.last_sync,
        drift_detected: index.drift_detected,
        parser_health: parser_health.to_string(),
        index_health: index_health.to_string(),
        issues,
        recommended_action,
    })
}

#[derive(Debug, Clone)]
struct FingerprintedNote {
    slug: String,
    file_path: String,
    raw_hash: String,
    file_mtime_ms: i64,
}

#[derive(Debug, Clone)]
struct MarkdownScanState {
    parsed: BTreeMap<String, FingerprintedNote>,
    parse_failures: Vec<String>,
    total_markdown_files: usize,
}

fn scan_current_markdown_state(memory_root: &Path) -> Result<MarkdownScanState> {
    let files = scan_markdown_files(memory_root)?;
    let mut parsed = BTreeMap::new();
    let mut parse_failures = Vec::new();
    for file in &files {
        match parse_canonical_note(memory_root, file, None) {
            Ok(note) => {
                parsed.insert(
                    note.slug.clone(),
                    FingerprintedNote {
                        slug: note.slug,
                        file_path: note.file_path,
                        raw_hash: note.raw_hash,
                        file_mtime_ms: note.file_mtime_ms,
                    },
                );
            }
            Err(err) => {
                let relative = file
                    .strip_prefix(memory_root)
                    .unwrap_or(file)
                    .display()
                    .to_string();
                parse_failures.push(format!("{relative}: {err}"));
            }
        }
    }
    Ok(MarkdownScanState {
        parsed,
        parse_failures,
        total_markdown_files: files.len(),
    })
}

fn read_indexed_fingerprints(conn: &Connection) -> Result<BTreeMap<String, FingerprintedNote>> {
    let mut statement = conn.prepare(
        "SELECT slug, file_path, raw_hash, file_mtime_ms
         FROM notes
         ORDER BY slug",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(FingerprintedNote {
            slug: row.get(0)?,
            file_path: row.get(1)?,
            raw_hash: row.get(2)?,
            file_mtime_ms: row.get(3)?,
        })
    })?;
    let notes = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(notes
        .into_iter()
        .map(|note| (note.slug.clone(), note))
        .collect())
}

fn diff_fingerprints(
    parsed: &BTreeMap<String, FingerprintedNote>,
    indexed: &BTreeMap<String, FingerprintedNote>,
    parse_failures: &[String],
) -> IndexInconsistencies {
    let missing_from_index = parsed
        .iter()
        .filter(|(slug, _)| !indexed.contains_key(*slug))
        .map(|(_, note)| fingerprint_label(&note.slug, &note.file_path))
        .collect();
    let orphaned_in_index = indexed
        .iter()
        .filter(|(slug, _)| !parsed.contains_key(*slug))
        .map(|(_, note)| fingerprint_label(&note.slug, &note.file_path))
        .collect();
    let stale_fingerprints = parsed
        .iter()
        .filter_map(|(slug, current)| {
            indexed.get(slug).and_then(|stored| {
                if stored.file_path != current.file_path
                    || stored.raw_hash != current.raw_hash
                    || stored.file_mtime_ms != current.file_mtime_ms
                {
                    Some(fingerprint_label(&current.slug, &current.file_path))
                } else {
                    None
                }
            })
        })
        .collect();

    IndexInconsistencies {
        missing_from_index,
        orphaned_in_index,
        stale_fingerprints,
        parse_failures: parse_failures.to_vec(),
    }
}

fn summarize_inconsistencies(inconsistencies: &IndexInconsistencies) -> Vec<String> {
    let mut failures = Vec::new();
    if !inconsistencies.missing_from_index.is_empty() {
        failures.push(format!(
            "{} canonical note(s) are missing from the derived index",
            inconsistencies.missing_from_index.len()
        ));
    }
    if !inconsistencies.orphaned_in_index.is_empty() {
        failures.push(format!(
            "{} derived index row(s) no longer map to Markdown truth",
            inconsistencies.orphaned_in_index.len()
        ));
    }
    if !inconsistencies.stale_fingerprints.is_empty() {
        failures.push(format!(
            "{} indexed note(s) drifted from current Markdown fingerprints",
            inconsistencies.stale_fingerprints.len()
        ));
    }
    if !inconsistencies.parse_failures.is_empty() {
        failures.push(format!(
            "{} canonical Markdown file(s) failed to parse during diagnostics",
            inconsistencies.parse_failures.len()
        ));
    }
    failures
}

fn fingerprint_label(slug: &str, file_path: &str) -> String {
    format!("{slug} ({file_path})")
}

fn project_name(engine: &Engine) -> String {
    engine.context.project_name()
}

#[cfg(test)]
mod tests {
    use super::{index_status, memory_status};
    use crate::engine::Engine;
    use crate::model::StorageMode;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(prefix: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("obsidian-memory-diagnostics-{prefix}-{suffix}"));
        std::fs::create_dir_all(&root).expect("create temp root");
        root
    }

    fn write_note(root: &Path, relative: &str, title: &str, node_type: &str, body: &str) {
        let path = root.join("memory").join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create parent");
        }
        let normalized_type = node_type
            .trim()
            .replace(['-', ' '], "_")
            .to_ascii_lowercase();
        let slug = if relative == "_index.md" {
            "_index".to_string()
        } else {
            Path::new(relative)
                .file_stem()
                .and_then(|value| value.to_str())
                .expect("slug")
                .to_string()
        };
        let content = format!(
            "---\nid: {normalized_type}-{slug}\ntype: {node_type}\ntitle: {title}\nstatus: active\nproject: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# {title}\n\n## Summary\n{body}\n\n## Observations\n\n## Relations\n\n## References\n"
        );
        std::fs::write(path, content).expect("write note");
    }

    #[test]
    fn diagnostics_snapshot_works_without_db() {
        let root = temp_root("no-db");
        write_note(
            &root,
            "_index.md",
            "Workspace",
            "Project",
            "Project summary.",
        );
        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");

        let index = index_status(&engine).expect("index_status");
        let memory = memory_status(&engine).expect("memory_status");

        assert!(!index.indexed);
        assert!(index.drift_detected);
        assert_eq!(index.pending_markdown_files, 1);
        assert_eq!(memory.health, "empty");
        assert_eq!(memory.index_health, "empty");

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn diagnostics_detect_fingerprint_drift_missing_rows_and_parse_failures() {
        let root = temp_root("drift");
        write_note(
            &root,
            "_index.md",
            "Workspace",
            "Project",
            "Project summary.",
        );
        write_note(
            &root,
            "decisions/auth.md",
            "Auth",
            "Decision",
            "Auth summary.",
        );
        let engine = Engine::new_with_mode(&root, StorageMode::Project).expect("engine");
        engine.rebuild_index().expect("rebuild");

        write_note(
            &root,
            "_index.md",
            "Workspace",
            "Project",
            "Updated summary.",
        );
        std::fs::remove_file(root.join("memory").join("decisions/auth.md"))
            .expect("remove decision");
        write_note(
            &root,
            "risks/runtime.md",
            "Runtime",
            "Risk",
            "Runtime risk.",
        );
        std::fs::create_dir_all(root.join("memory").join("modules"))
            .expect("create modules directory");
        std::fs::write(
            root.join("memory").join("modules/bad.md"),
            "---\nid: module-bad\ntype: Module\ntitle: Broken\nstatus: active\nproject: workspace\ncreated_at: 1\nupdated_at: 1\n---\n\n# Broken\n",
        )
        .expect("write malformed note");

        let status = index_status(&engine).expect("index_status");

        assert!(status.drift_detected);
        assert!(status.fingerprint_drift_detected);
        assert!(
            !status.inconsistencies.missing_from_index.is_empty()
                || status.pending_markdown_files > 0
        );
        assert!(!status.inconsistencies.orphaned_in_index.is_empty());
        assert!(!status.inconsistencies.stale_fingerprints.is_empty());
        assert!(!status.inconsistencies.parse_failures.is_empty());

        let _ = std::fs::remove_dir_all(root);
    }
}
