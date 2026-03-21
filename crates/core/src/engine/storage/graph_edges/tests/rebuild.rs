use rusqlite::{Connection, params};

use super::support::{
    assert_edge_weight, assert_reset_metadata, fetch_edges, insert_file, run_full_rebuild,
    setup_graph_edge_schema,
};

#[test]
fn rebuild_file_graph_edges_materializes_ref_exact_edges() -> anyhow::Result<()> {
    let mut conn = Connection::open_in_memory()?;
    setup_graph_edge_schema(&conn)?;
    insert_file(&conn, "src/a.rs")?;
    insert_file(&conn, "src/b.rs")?;
    conn.execute(
        "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
        params!["src/b.rs", "Helper"],
    )?;
    conn.execute(
        "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
        params!["src/a.rs", "Helper"],
    )?;

    run_full_rebuild(&mut conn)?;

    assert_eq!(
        fetch_edges(&conn)?,
        vec![(
            "src/a.rs".to_string(),
            "src/b.rs".to_string(),
            "ref_exact".to_string(),
            1,
            1.0
        )]
    );
    Ok(())
}

#[test]
fn rebuild_file_graph_edges_materializes_ref_tail_unique_edges() -> anyhow::Result<()> {
    let mut conn = Connection::open_in_memory()?;
    setup_graph_edge_schema(&conn)?;
    insert_file(&conn, "src/a.rs")?;
    insert_file(&conn, "src/b.rs")?;
    conn.execute(
        "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
        params!["src/b.rs", "Helper"],
    )?;
    conn.execute(
        "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
        params!["src/a.rs", "crate::nested::Helper"],
    )?;

    run_full_rebuild(&mut conn)?;

    let edges = fetch_edges(&conn)?;
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].0, "src/a.rs");
    assert_eq!(edges[0].1, "src/b.rs");
    assert_eq!(edges[0].2, "ref_tail_unique");
    assert_eq!(edges[0].3, 1);
    assert_edge_weight(edges[0].4, 0.72);
    Ok(())
}

#[test]
fn rebuild_file_graph_edges_materializes_shared_dep_edges() -> anyhow::Result<()> {
    let mut conn = Connection::open_in_memory()?;
    setup_graph_edge_schema(&conn)?;
    insert_file(&conn, "src/a.rs")?;
    insert_file(&conn, "src/b.rs")?;
    conn.execute(
        "INSERT INTO module_deps(path, dep) VALUES (?1, ?2)",
        params!["src/a.rs", "serde"],
    )?;
    conn.execute(
        "INSERT INTO module_deps(path, dep) VALUES (?1, ?2)",
        params!["src/b.rs", "serde"],
    )?;

    run_full_rebuild(&mut conn)?;

    let edges = fetch_edges(&conn)?;
    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].0, "src/a.rs");
    assert_eq!(edges[0].1, "src/b.rs");
    assert_eq!(edges[0].2, "shared_dep");
    assert_eq!(edges[0].3, 1);
    assert_edge_weight(edges[0].4, 0.35);
    assert_eq!(edges[1].0, "src/b.rs");
    assert_eq!(edges[1].1, "src/a.rs");
    assert_eq!(edges[1].2, "shared_dep");
    assert_eq!(edges[1].3, 1);
    assert_edge_weight(edges[1].4, 0.35);
    Ok(())
}

#[test]
fn rebuild_file_graph_edges_skips_self_edges() -> anyhow::Result<()> {
    let mut conn = Connection::open_in_memory()?;
    setup_graph_edge_schema(&conn)?;
    insert_file(&conn, "src/a.rs")?;
    conn.execute(
        "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
        params!["src/a.rs", "Helper"],
    )?;
    conn.execute(
        "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
        params!["src/a.rs", "Helper"],
    )?;

    run_full_rebuild(&mut conn)?;

    assert!(fetch_edges(&conn)?.is_empty());
    Ok(())
}

#[test]
fn rebuild_file_graph_edges_resets_metadata_before_refresh() -> anyhow::Result<()> {
    let mut conn = Connection::open_in_memory()?;
    setup_graph_edge_schema(&conn)?;
    insert_file(&conn, "src/a.rs")?;
    insert_file(&conn, "src/b.rs")?;
    insert_file(&conn, "src/c.rs")?;
    conn.execute(
        "INSERT INTO file_graph_edges(src_path, dst_path, edge_kind, raw_count, weight)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["old/src.rs", "old/dst.rs", "stale", 99_i64, 42.0_f64],
    )?;
    conn.execute(
        "INSERT INTO symbols(path, name) VALUES (?1, ?2)",
        params!["src/b.rs", "Helper"],
    )?;
    conn.execute(
        "INSERT INTO refs(path, symbol) VALUES (?1, ?2)",
        params!["src/a.rs", "Helper"],
    )?;

    run_full_rebuild(&mut conn)?;
    assert_reset_metadata(&conn, 3)?;
    Ok(())
}
