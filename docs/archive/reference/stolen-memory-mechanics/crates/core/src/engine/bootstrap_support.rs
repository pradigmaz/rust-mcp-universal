use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};

use crate::bootstrap::{DecisionLogEntry, RecentChangeItem, normalize_status};
use crate::model::NodeSummary;

pub(super) fn project_summary(conn: &Connection) -> Result<Option<String>> {
    conn.query_row(
        "SELECT summary FROM notes WHERE node_type = 'project' ORDER BY file_mtime_ms DESC LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map(|value| value.and_then(|summary| first_non_empty_line(&summary)))
    .map_err(Into::into)
}

pub(super) fn recent_decisions(conn: &Connection, limit: usize) -> Result<Vec<DecisionLogEntry>> {
    let nodes = recent_node_summaries(conn, Some(&["decision"]), limit, true)?;
    Ok(nodes.into_iter().map(map_decision_entry).collect())
}

pub(super) fn search_decisions(
    conn: &Connection,
    topic: &str,
    limit: usize,
) -> Result<Vec<DecisionLogEntry>> {
    let found = search_node_summaries(conn, topic, limit, true)?;
    Ok(found
        .into_iter()
        .filter(|node| normalize_status(&node.status) != crate::bootstrap::NormalizedStatus::Closed)
        .filter(|node| node.node_type == "decision")
        .take(limit)
        .map(map_decision_entry)
        .collect())
}

pub(super) fn recent_node_summaries(
    conn: &Connection,
    node_types: Option<&[&str]>,
    limit: usize,
    unresolved_only: bool,
) -> Result<Vec<NodeSummary>> {
    let mut sql =
        "SELECT id, slug, title, node_type, status, file_path, summary, updated_at FROM notes"
            .to_string();
    let mut params = Vec::new();
    if let Some(node_types) = node_types {
        sql.push_str(" WHERE node_type IN (");
        sql.push_str(&vec!["?"; node_types.len()].join(", "));
        sql.push(')');
        params.extend(node_types.iter().map(|value| value.to_string()));
    } else {
        sql.push_str(" WHERE node_type != 'section_hub'");
    }
    sql.push_str(" ORDER BY file_mtime_ms DESC LIMIT ?");
    params.push(limit.to_string());
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(params.iter()), map_node_summary)?;
    let mut nodes = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    if unresolved_only {
        nodes.retain(|node| {
            normalize_status(&node.status) != crate::bootstrap::NormalizedStatus::Closed
        });
    }
    Ok(nodes)
}

pub(super) fn unresolved_node_summaries(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<NodeSummary>> {
    Ok(
        recent_node_summaries(conn, Some(&["risk", "constraint"]), limit * 2, false)?
            .into_iter()
            .filter(|node| {
                normalize_status(&node.status) != crate::bootstrap::NormalizedStatus::Closed
            })
            .take(limit)
            .collect(),
    )
}

pub(super) fn search_node_summaries(
    conn: &Connection,
    seed: &str,
    limit: usize,
    include_all_types: bool,
) -> Result<Vec<NodeSummary>> {
    match fts_search_nodes(conn, seed, limit) {
        Ok(nodes) => Ok(filter_context_types(nodes, include_all_types)),
        Err(_) => like_search_nodes(conn, seed, limit)
            .map(|nodes| filter_context_types(nodes, include_all_types)),
    }
}

pub(super) fn map_recent_change(node: NodeSummary) -> RecentChangeItem {
    RecentChangeItem {
        id: node.id,
        slug: node.slug,
        title: node.title,
        node_type: node.node_type,
        status: node.status,
        file_path: node.file_path,
        change_hint: first_non_empty_line(&node.summary).unwrap_or_default(),
        updated_at: node.updated_at,
    }
}

pub(super) fn budget_exhausted(chars: usize, max_chars: usize, max_tokens: usize) -> bool {
    chars > max_chars || chars.div_ceil(4) > max_tokens
}

pub(super) fn related_context_nodes(
    conn: &Connection,
    seed_slugs: &[String],
    limit: usize,
) -> Result<Vec<NodeSummary>> {
    if seed_slugs.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; seed_slugs.len()].join(", ");
    let sql = format!(
        "SELECT DISTINCT CASE WHEN source_slug IN ({placeholders}) THEN target_slug ELSE source_slug END AS related_slug
         FROM relations
         WHERE source_slug IN ({placeholders}) OR target_slug IN ({placeholders})"
    );
    let mut params = Vec::new();
    params.extend(seed_slugs.iter().cloned());
    params.extend(seed_slugs.iter().cloned());
    params.extend(seed_slugs.iter().cloned());
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(params.iter()), |row| {
        row.get::<_, String>(0)
    })?;
    let related = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    node_summaries_for_slugs(conn, &related, limit, false)
}

pub(super) fn matching_recent_changes(
    conn: &Connection,
    seed: &str,
    seed_slugs: &[String],
) -> Result<Vec<RecentChangeItem>> {
    let changes = recent_node_summaries(conn, None, 12, true)?;
    let seed_lower = seed.to_ascii_lowercase();
    let mut matched = changes
        .iter()
        .filter(|node| {
            seed_slugs.contains(&node.slug)
                || node.title.to_ascii_lowercase().contains(&seed_lower)
                || node.summary.to_ascii_lowercase().contains(&seed_lower)
        })
        .cloned()
        .map(map_recent_change)
        .collect::<Vec<_>>();
    if matched.is_empty() {
        matched = changes.into_iter().take(3).map(map_recent_change).collect();
    }
    Ok(matched)
}

fn fts_search_nodes(conn: &Connection, seed: &str, limit: usize) -> Result<Vec<NodeSummary>> {
    let mut statement = conn.prepare(
        "SELECT n.id, n.slug, n.title, n.node_type, n.status, n.file_path, n.summary, n.updated_at
         FROM notes_fts
         JOIN notes n ON n.slug = notes_fts.slug
         WHERE notes_fts MATCH ?1
         ORDER BY bm25(notes_fts), n.file_mtime_ms DESC
         LIMIT ?2",
    )?;
    let rows = statement.query_map(params![seed, limit as i64], map_node_summary)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn like_search_nodes(conn: &Connection, seed: &str, limit: usize) -> Result<Vec<NodeSummary>> {
    let like = format!("%{}%", seed.trim().to_ascii_lowercase());
    let mut statement = conn.prepare(
        "SELECT DISTINCT n.id, n.slug, n.title, n.node_type, n.status, n.file_path, n.summary, n.updated_at
         FROM notes n
         LEFT JOIN note_observations o ON o.slug = n.slug
         WHERE lower(n.title) LIKE ?1
            OR lower(n.summary) LIKE ?1
            OR lower(n.body) LIKE ?1
            OR lower(COALESCE(o.observation, '')) LIKE ?1
         ORDER BY n.file_mtime_ms DESC
         LIMIT ?2",
    )?;
    let rows = statement.query_map(params![like, limit as i64], map_node_summary)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn filter_context_types(nodes: Vec<NodeSummary>, include_all_types: bool) -> Vec<NodeSummary> {
    let nodes = nodes.into_iter().filter(|node| {
        normalize_status(&node.status) != crate::bootstrap::NormalizedStatus::Closed
    });
    if include_all_types {
        return nodes.collect();
    }
    nodes
        .filter(|node| {
            matches!(
                node.node_type.as_str(),
                "decision"
                    | "module"
                    | "artifact"
                    | "architecture_note"
                    | "task"
                    | "progress_entry"
            )
        })
        .collect()
}

fn map_node_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<NodeSummary> {
    Ok(NodeSummary {
        id: row.get(0)?,
        slug: row.get(1)?,
        title: row.get(2)?,
        node_type: row.get(3)?,
        status: row.get(4)?,
        file_path: row.get(5)?,
        summary: row.get(6)?,
        updated_at: row.get(7)?,
        normalized_status: None,
    })
}

fn map_decision_entry(node: NodeSummary) -> DecisionLogEntry {
    DecisionLogEntry {
        id: node.id,
        slug: node.slug,
        title: node.title,
        status: node.status,
        summary: node.summary,
        updated_at: node.updated_at,
    }
}

fn first_non_empty_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn node_summaries_for_slugs(
    conn: &Connection,
    slugs: &[String],
    limit: usize,
    include_all_types: bool,
) -> Result<Vec<NodeSummary>> {
    if slugs.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders = vec!["?"; slugs.len()].join(", ");
    let sql = format!(
        "SELECT id, slug, title, node_type, status, file_path, summary, updated_at
         FROM notes
         WHERE slug IN ({placeholders})
         ORDER BY file_mtime_ms DESC
         LIMIT ?"
    );
    let mut params = slugs.to_vec();
    params.push(limit.to_string());
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(params.iter()), map_node_summary)?;
    Ok(filter_context_types(
        rows.collect::<rusqlite::Result<Vec<_>>>()?,
        include_all_types,
    ))
}
