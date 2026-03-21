use super::*;

#[cfg(windows)]
#[test]
fn call_path_accepts_absolute_windows_paths_with_case_only_differences()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-call-path-windows-case");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/lib.rs"),
        "pub fn windows_case_symbol() {}\n",
    )?;

    let engine = Engine::new(project_dir.clone(), Some(project_dir.join(".rmu/index.db")))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let absolute_path = project_dir
        .join("src/lib.rs")
        .to_string_lossy()
        .to_uppercase();
    let result = engine.call_path(&absolute_path, &absolute_path, 3)?;

    assert!(result.found);
    assert_eq!(result.hops, 0);
    assert_eq!(result.path, vec!["src/lib.rs"]);

    cleanup_project(&project_dir);
    Ok(())
}

#[test]
fn changed_since_run_repairs_corrupted_file_graph_edges_without_forced_full_reindex()
-> Result<(), Box<dyn Error>> {
    let project_dir = temp_project_dir("rmu-core-tests-graph-edge-repair");
    fs::create_dir_all(project_dir.join("src"))?;
    fs::write(
        project_dir.join("src/main.rs"),
        r#"
mod worker;

pub fn anchor_entry() {
    let note = "graph_repair_anchor";
    worker::render_worker();
    println!("{note}");
}
"#,
    )?;
    fs::write(
        project_dir.join("src/worker.rs"),
        r#"
pub fn render_worker() {
    println!("worker implementation only");
}
"#,
    )?;

    let db_path = project_dir.join(".rmu/index.db");
    let engine = Engine::new(project_dir.clone(), Some(db_path.clone()))?;
    let _ = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: None,
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: true,
    })?;

    let original_edge_count = file_graph_edge_count(&db_path)?;
    assert!(original_edge_count > 0, "expected indexed graph edges");

    {
        let conn = Connection::open(&db_path)?;
        conn.execute("DELETE FROM file_graph_edges", [])?;
        conn.execute(
            "UPDATE files
             SET graph_edge_out_count = NULL,
                 graph_edge_in_count = NULL,
                 graph_edge_hash = NULL,
                 graph_edge_fingerprint_version = NULL
             WHERE path IN (?1, ?2)",
            ["src/main.rs", "src/worker.rs"],
        )?;
    }

    let summary = engine.index_path_with_options(&IndexingOptions {
        profile: None,
        changed_since: Some(OffsetDateTime::now_utc() + Duration::hours(1)),
        changed_since_commit: None,
        include_paths: vec![],
        exclude_paths: vec![],
        reindex: false,
    })?;

    assert!(
        summary.indexed >= 1,
        "expected explicit incremental index run to repair corrupted graph metadata"
    );
    assert_eq!(file_graph_edge_count(&db_path)?, original_edge_count);
    assert!(graph_edge_metadata_present(&db_path, "src/main.rs")?);
    assert!(graph_edge_metadata_present(&db_path, "src/worker.rs")?);

    cleanup_project(&project_dir);
    Ok(())
}
