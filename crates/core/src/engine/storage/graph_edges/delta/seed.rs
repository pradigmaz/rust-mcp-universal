use std::collections::BTreeSet;

use anyhow::Result;

#[derive(Debug, Clone, Default)]
pub(in crate::engine) struct GraphRefreshSeed {
    pub(in crate::engine) symbol_names: BTreeSet<String>,
    pub(in crate::engine) dep_names: BTreeSet<String>,
    pub(in crate::engine) neighbor_paths: BTreeSet<String>,
}

pub(in crate::engine) fn capture_graph_refresh_seed(
    tx: &rusqlite::Transaction<'_>,
    path: &str,
) -> Result<GraphRefreshSeed> {
    let mut seed = GraphRefreshSeed::default();

    let mut symbols = tx.prepare("SELECT name FROM symbols WHERE path = ?1 ORDER BY name ASC")?;
    let symbol_rows = symbols.query_map([path], |row| row.get::<_, String>(0))?;
    for row in symbol_rows {
        seed.symbol_names.insert(row?);
    }

    let mut deps = tx.prepare("SELECT dep FROM module_deps WHERE path = ?1 ORDER BY dep ASC")?;
    let dep_rows = deps.query_map([path], |row| row.get::<_, String>(0))?;
    for row in dep_rows {
        seed.dep_names.insert(row?);
    }

    let mut neighbors = tx.prepare(
        r#"
        SELECT DISTINCT
            CASE
                WHEN src_path = ?1 THEN dst_path
                ELSE src_path
            END AS neighbor_path
        FROM file_graph_edges
        WHERE src_path = ?1 OR dst_path = ?1
        ORDER BY neighbor_path ASC
        "#,
    )?;
    let neighbor_rows = neighbors.query_map([path], |row| row.get::<_, String>(0))?;
    for row in neighbor_rows {
        seed.neighbor_paths.insert(row?);
    }

    Ok(seed)
}
