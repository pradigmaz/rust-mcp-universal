use super::{
    Duration, Engine, OffsetDateTime, OptionalExtension, PrivacyMode, QualityMode, QueryOptions,
    RuleViolationsOptions, SemanticFailMode, temp_dir, write_project_file,
};

#[test]
fn oversize_files_are_quality_only_and_not_searchable() -> anyhow::Result<()> {
    let root = temp_dir("rmu-quality-oversize");
    std::fs::create_dir_all(&root)?;
    write_project_file(&root, "src/lib.rs", "pub fn searchable_probe() {}\n")?;
    let oversize = format!(
        "oversize_unique_marker\n{}",
        "X".repeat((crate::utils::INDEX_FILE_LIMIT as usize) + 2048)
    );
    write_project_file(&root, "src/big.rs", &oversize)?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let conn = engine.open_db()?;
    let indexed_big: Option<String> = conn
        .query_row(
            "SELECT path FROM files WHERE path = 'src/big.rs'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    assert!(indexed_big.is_none());

    let quality_big: Option<(String, Option<i64>, String)> = conn
        .query_row(
            "SELECT path, total_lines, quality_mode FROM file_quality WHERE path = 'src/big.rs'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let quality_big = quality_big.expect("oversize quality snapshot should exist");
    assert!(quality_big.1.is_none());
    assert_eq!(quality_big.2, "quality-only-oversize");

    let violations = engine.rule_violations(&RuleViolationsOptions::default())?;
    let big_hit = violations
        .hits
        .iter()
        .find(|hit| hit.path == "src/big.rs")
        .expect("oversize path should be reported");
    assert_eq!(big_hit.quality_mode, QualityMode::QualityOnlyOversize);

    let search_hits = engine.search(&QueryOptions {
        query: "oversize_unique_marker".to_string(),
        limit: 5,
        detailed: false,
        semantic: false,
        semantic_fail_mode: SemanticFailMode::FailOpen,
        privacy_mode: PrivacyMode::Off,
        context_mode: None,
    })?;
    assert!(
        search_hits.iter().all(|hit| hit.path != "src/big.rs"),
        "oversize files must not leak into retrieval surfaces"
    );

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn changed_since_backfills_missing_quality_rows_even_when_cutoff_would_skip() -> anyhow::Result<()>
{
    let root = temp_dir("rmu-quality-backfill");
    std::fs::create_dir_all(&root)?;
    write_project_file(&root, "src/lib.rs", "pub fn quality_backfill() {}\n")?;

    let engine = Engine::new(root.clone(), Some(root.join(".rmu/index.db")))?;
    engine.index_path()?;

    let conn = engine.open_db()?;
    conn.execute(
        "DELETE FROM file_rule_violations WHERE path = 'src/lib.rs'",
        [],
    )?;
    conn.execute("DELETE FROM file_quality WHERE path = 'src/lib.rs'", [])?;

    let repaired = engine.index_path_with_options(&crate::model::IndexingOptions {
        changed_since: Some(OffsetDateTime::now_utc() + Duration::days(1)),
        ..crate::model::IndexingOptions::default()
    })?;
    assert_eq!(repaired.indexed, 0);
    assert_eq!(repaired.changed, 0);
    assert_eq!(repaired.unchanged, 1);
    assert_eq!(repaired.skipped_before_changed_since, 0);

    let restored: Option<String> = engine
        .open_db()?
        .query_row(
            "SELECT path FROM file_quality WHERE path = 'src/lib.rs'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    assert_eq!(restored.as_deref(), Some("src/lib.rs"));

    let _ = std::fs::remove_dir_all(root);
    Ok(())
}
