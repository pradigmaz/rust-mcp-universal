use super::*;

#[test]
fn db_maintenance_reports_size_stats_and_prune_summary() -> Result<(), Box<dyn Error>> {
    let (project_dir, engine) = setup_indexed_project()?;

    let result = engine.db_maintenance(DbMaintenanceOptions {
        stats: true,
        prune: true,
        ..DbMaintenanceOptions::default()
    })?;

    let stats = result.stats.expect("stats should be present");
    assert!(stats.page_size > 0);
    assert!(stats.page_count >= 1);
    assert!(stats.freelist_count >= 0);
    assert!(stats.total_size_bytes >= stats.db_size_bytes);
    assert!(stats.total_size_bytes >= stats.wal_size_bytes);
    assert!(stats.total_size_bytes >= stats.shm_size_bytes);

    let prune = result.prune.expect("prune summary should be present");
    assert_eq!(prune.removed_databases, 0);
    assert_eq!(prune.removed_sidecars, 0);

    cleanup_project(&project_dir);
    Ok(())
}
