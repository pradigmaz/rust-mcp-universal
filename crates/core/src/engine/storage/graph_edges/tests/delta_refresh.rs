use rusqlite::Connection;

use super::support::{
    fetch_edges, fetch_metadata, insert_file, insert_ref, insert_symbol,
    prepare_dirty_delta_fixture, run_delta_refresh, run_full_rebuild, setup_graph_edge_schema,
};

#[test]
fn refresh_file_graph_edges_matches_full_rebuild_for_dirty_neighborhood() -> anyhow::Result<()> {
    let mut delta_conn = Connection::open_in_memory()?;
    let mut full_conn = Connection::open_in_memory()?;
    let (dirty_paths, pre_refresh) = prepare_dirty_delta_fixture(&mut delta_conn, &mut full_conn)?;

    run_delta_refresh(&mut delta_conn, &dirty_paths, &pre_refresh)?;
    run_full_rebuild(&mut full_conn)?;

    assert_eq!(fetch_edges(&delta_conn)?, fetch_edges(&full_conn)?);
    assert_eq!(fetch_metadata(&delta_conn)?, fetch_metadata(&full_conn)?);
    Ok(())
}

#[test]
fn refresh_file_graph_edges_does_not_resolve_refs_to_impl_symbols() -> anyhow::Result<()> {
    let mut conn = Connection::open_in_memory()?;
    setup_graph_edge_schema(&conn)?;
    insert_file(&conn, "src/caller.rs")?;
    insert_file(&conn, "src/impl_block.rs")?;
    insert_symbol(&conn, "src/impl_block.rs", "Engine", "impl")?;
    insert_ref(&conn, "src/caller.rs", "Engine")?;

    let dirty_paths = ["src/caller.rs".to_string()].into_iter().collect();
    run_delta_refresh(&mut conn, &dirty_paths, &Default::default())?;

    assert!(fetch_edges(&conn)?.is_empty());
    Ok(())
}
