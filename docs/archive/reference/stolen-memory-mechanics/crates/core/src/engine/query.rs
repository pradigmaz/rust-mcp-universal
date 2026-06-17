use std::collections::BTreeSet;

use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::json;

use crate::model::{GraphResult, IndexedCounts, NodeDetails, NodeRelation, SearchHit};

use super::{Engine, StateFailure};

pub(super) fn search_memory(engine: &Engine, query: &str, limit: usize) -> Result<Vec<SearchHit>> {
    let conn = engine.open_connection()?;
    let mut hits = exact_search_hits(&conn, query, limit)?;
    let mut seen = hits
        .iter()
        .map(|hit| hit.slug.clone())
        .collect::<BTreeSet<_>>();
    let Some(fts_query) = fts_safe_query(query) else {
        return Ok(hits);
    };
    let mut statement = conn.prepare(
        "SELECT n.id, n.slug, n.title, n.node_type, n.status, n.file_path, n.summary,
                bm25(notes_fts) AS score
         FROM notes_fts
         JOIN notes n ON n.slug = notes_fts.slug
         WHERE notes_fts MATCH ?1
         ORDER BY CASE WHEN n.node_type = 'section_hub' THEN 1 ELSE 0 END, score
         LIMIT ?2",
    )?;
    let rows = statement.query_map(params![fts_query, limit as i64], |row| {
        Ok(SearchHit {
            id: row.get(0)?,
            slug: row.get(1)?,
            title: row.get(2)?,
            node_type: row.get(3)?,
            status: row.get(4)?,
            file_path: row.get(5)?,
            summary: row.get(6)?,
            score: row.get(7)?,
        })
    })?;
    for hit in rows.collect::<rusqlite::Result<Vec<_>>>()? {
        if seen.insert(hit.slug.clone()) {
            hits.push(hit);
        }
        if hits.len() >= limit {
            break;
        }
    }
    Ok(hits)
}

pub(super) fn open_nodes(engine: &Engine, slugs: &[String]) -> Result<Vec<NodeDetails>> {
    let conn = engine.open_connection()?;
    let mut nodes = Vec::new();
    for token in slugs {
        let Some(slug) = resolve_node_slug(&conn, token)? else {
            continue;
        };
        let Some(mut node) = load_node(&conn, &slug)? else {
            continue;
        };
        node.aliases = load_string_rows(
            &conn,
            "SELECT alias FROM note_aliases WHERE slug = ?1 ORDER BY alias",
            &slug,
        )?;
        node.tags = load_string_rows(
            &conn,
            "SELECT tag FROM note_tags WHERE slug = ?1 ORDER BY tag",
            &slug,
        )?;
        node.observations = load_string_rows(
            &conn,
            "SELECT observation FROM note_observations WHERE slug = ?1",
            &slug,
        )?;
        node.references = load_string_rows(
            &conn,
            "SELECT reference FROM note_references WHERE slug = ?1",
            &slug,
        )?;
        node.relations = load_relations_for_slug(&conn, &slug)?;
        nodes.push(node);
    }
    Ok(nodes)
}

pub(super) fn read_graph(engine: &Engine, slugs: &[String]) -> Result<GraphResult> {
    let conn = engine.open_connection()?;
    let mut requested = Vec::new();
    for token in slugs {
        if let Some(slug) = resolve_node_slug(&conn, token)? {
            requested.push(slug);
        }
    }
    let mut relations = Vec::new();
    for slug in requested.clone() {
        let slug_relations = load_relations_for_slug(&conn, &slug)?;
        for relation in &slug_relations {
            if !requested.contains(&relation.target_slug) {
                requested.push(relation.target_slug.clone());
            }
            if !requested.contains(&relation.source_slug) {
                requested.push(relation.source_slug.clone());
            }
        }
        relations.extend(slug_relations);
    }
    let nodes = open_nodes(engine, &requested)?;
    Ok(GraphResult { nodes, relations })
}

fn exact_search_hits(conn: &Connection, token: &str, limit: usize) -> Result<Vec<SearchHit>> {
    let lookup = normalize_lookup_text(token);
    let mut statement = conn.prepare(
        "SELECT DISTINCT n.id, n.slug, n.title, n.node_type, n.status, n.file_path, n.summary
         FROM notes n
         LEFT JOIN note_aliases a ON a.slug = n.slug
         WHERE n.id = ?1
            OR n.slug = ?1
            OR lower(trim(n.title)) = ?2
            OR lower(trim(a.alias)) = ?2
         ORDER BY CASE WHEN n.node_type = 'section_hub' THEN 1 ELSE 0 END, n.title
         LIMIT ?3",
    )?;
    let rows = statement.query_map(params![token, lookup, limit as i64], |row| {
        Ok(SearchHit {
            id: row.get(0)?,
            slug: row.get(1)?,
            title: row.get(2)?,
            node_type: row.get(3)?,
            status: row.get(4)?,
            file_path: row.get(5)?,
            summary: row.get(6)?,
            score: -1000.0,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn resolve_node_slug(conn: &Connection, token: &str) -> Result<Option<String>> {
    if let Some(slug) = resolve_slug_by_column(conn, "id", token)? {
        return Ok(Some(slug));
    }
    if let Some(slug) = resolve_slug_by_column(conn, "slug", token)? {
        return Ok(Some(slug));
    }
    resolve_slug_by_title_or_alias(conn, token)
}

fn resolve_slug_by_column(conn: &Connection, column: &str, token: &str) -> Result<Option<String>> {
    let sql = format!("SELECT slug FROM notes WHERE {column} = ?1 ORDER BY slug");
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([token], |row| row.get::<_, String>(0))?;
    single_resolved_slug(token, column, rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn resolve_slug_by_title_or_alias(conn: &Connection, token: &str) -> Result<Option<String>> {
    let lookup = normalize_lookup_text(token);
    if lookup.is_empty() {
        return Ok(None);
    }
    let mut statement = conn.prepare(
        "SELECT DISTINCT n.slug
         FROM notes n
         LEFT JOIN note_aliases a ON a.slug = n.slug
         WHERE lower(trim(n.title)) = ?1 OR lower(trim(a.alias)) = ?1
         ORDER BY n.slug",
    )?;
    let rows = statement.query_map([lookup], |row| row.get::<_, String>(0))?;
    single_resolved_slug(
        token,
        "title_or_alias",
        rows.collect::<rusqlite::Result<Vec<_>>>()?,
    )
}

fn single_resolved_slug(token: &str, column: &str, slugs: Vec<String>) -> Result<Option<String>> {
    if slugs.len() > 1 {
        return Err(StateFailure::new(
            "E_AMBIGUOUS_TARGET",
            format!("node `{token}` matched multiple records by {column}"),
            json!({
                "node": token,
                "column": column,
                "matches": slugs,
            }),
        )
        .into());
    }
    Ok(slugs.into_iter().next())
}

fn fts_safe_query(raw: &str) -> Option<String> {
    let tokens = raw
        .split(|character: char| !character.is_alphanumeric() && character != '_')
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    if tokens.is_empty() {
        None
    } else {
        Some(tokens.join(" "))
    }
}

fn load_node(conn: &Connection, slug: &str) -> Result<Option<NodeDetails>> {
    conn.query_row(
        "SELECT id, slug, title, node_type, status, project, file_path, summary, created_at, updated_at
         FROM notes WHERE slug = ?1",
        [slug],
        |row| {
            Ok(NodeDetails {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                node_type: row.get(3)?,
                status: row.get(4)?,
                project: row.get(5)?,
                file_path: row.get(6)?,
                summary: row.get(7)?,
                aliases: Vec::new(),
                tags: Vec::new(),
                observations: Vec::new(),
                relations: Vec::new(),
                references: Vec::new(),
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
}

pub(super) fn read_counts(conn: &Connection) -> Result<IndexedCounts> {
    Ok(IndexedCounts {
        notes: query_count(conn, "SELECT COUNT(*) FROM notes")?,
        decisions: query_count(
            conn,
            "SELECT COUNT(*) FROM notes WHERE node_type = 'decision'",
        )?,
        risks: query_count(conn, "SELECT COUNT(*) FROM notes WHERE node_type = 'risk'")?,
        constraints: query_count(
            conn,
            "SELECT COUNT(*) FROM notes WHERE node_type = 'constraint'",
        )?,
        artifacts: query_count(
            conn,
            "SELECT COUNT(*) FROM notes WHERE node_type = 'artifact'",
        )?,
        relations: query_count(conn, "SELECT COUNT(*) FROM relations")?,
        aliases: query_count(conn, "SELECT COUNT(*) FROM note_aliases")?,
        tags: query_count(conn, "SELECT COUNT(*) FROM note_tags")?,
        observations: query_count(conn, "SELECT COUNT(*) FROM note_observations")?,
    })
}

fn query_count(conn: &Connection, sql: &str) -> Result<usize> {
    let raw: i64 = conn.query_row(sql, [], |row| row.get(0))?;
    Ok(usize::try_from(raw).unwrap_or_default())
}

pub(super) fn read_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .optional()
    .map_err(Into::into)
}

fn load_string_rows(conn: &Connection, sql: &str, slug: &str) -> Result<Vec<String>> {
    let mut statement = conn.prepare(sql)?;
    let rows = statement.query_map([slug], |row| row.get(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn load_relations_for_slug(conn: &Connection, slug: &str) -> Result<Vec<NodeRelation>> {
    let mut statement = conn.prepare(
        "SELECT source_slug, target_slug, relation_kind
         FROM relations WHERE source_slug = ?1 OR target_slug = ?1
         ORDER BY source_slug, target_slug, relation_kind",
    )?;
    let rows = statement.query_map([slug], |row| {
        Ok(NodeRelation {
            source_slug: row.get(0)?,
            target_slug: row.get(1)?,
            relation_kind: row.get(2)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn normalize_lookup_text(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub(super) fn project_name(engine: &Engine) -> String {
    engine.context.project_name()
}
