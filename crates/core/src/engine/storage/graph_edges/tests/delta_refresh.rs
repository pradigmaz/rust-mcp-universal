use rusqlite::Connection;

use super::support::{
    fetch_edges, fetch_metadata, prepare_dirty_delta_fixture, run_delta_refresh, run_full_rebuild,
};

#[test]
fn refresh_file_graph_edges_matches_full_rebuild_for_dirty_neighborhood() -> anyhow::Result<()> {
    let mut delta_conn = Connection::open_in_memory()?;
    let mut full_conn = Connection::open_in_memory()?;
    let (dirty_paths, pre_refresh) =
        prepare_dirty_delta_fixture(&mut delta_conn, &mut full_conn)?;

    run_delta_refresh(&mut delta_conn, &dirty_paths, &pre_refresh)?;
    run_full_rebuild(&mut full_conn)?;

    assert_eq!(fetch_edges(&delta_conn)?, fetch_edges(&full_conn)?);
    assert_eq!(fetch_metadata(&delta_conn)?, fetch_metadata(&full_conn)?);
    Ok(())
}
