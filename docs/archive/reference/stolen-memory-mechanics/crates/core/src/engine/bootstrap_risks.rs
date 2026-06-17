use std::collections::BTreeMap;

use anyhow::Result;
use rusqlite::{Connection, params, params_from_iter};

use crate::bootstrap::{RiskHotspotItem, RiskHotspots, normalize_status};

use super::support::node_summaries_for_slugs;

pub(super) fn query_risk_hotspots(conn: &Connection, limit: usize) -> Result<RiskHotspots> {
    let slugs = unresolved_hotspot_slugs(conn, limit)?;
    let mut risks = Vec::new();
    let mut constraints = Vec::new();
    for (node_type, item) in hotspot_items(conn, &slugs)? {
        match node_type.as_str() {
            "risk" => risks.push(item),
            "constraint" => constraints.push(item),
            _ => {}
        }
    }
    Ok(RiskHotspots { risks, constraints })
}

pub(super) fn matching_risks(
    conn: &Connection,
    seed: &str,
    seed_slugs: &[String],
    limit: usize,
) -> Result<Vec<RiskHotspotItem>> {
    let ranked = query_risk_hotspots(conn, limit * 2)?;
    let mut items = ranked.risks;
    items.extend(ranked.constraints);
    let seed_lower = seed.to_ascii_lowercase();
    items.retain(|item| {
        seed_slugs.contains(&item.slug)
            || item.title.to_ascii_lowercase().contains(&seed_lower)
            || item.summary.to_ascii_lowercase().contains(&seed_lower)
            || item.blocks.iter().any(|slug| seed_slugs.contains(slug))
            || item.affects.iter().any(|slug| seed_slugs.contains(slug))
    });
    items.truncate(limit);
    Ok(items)
}

fn unresolved_hotspot_slugs(conn: &Connection, limit: usize) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT n.slug
         FROM notes n
         LEFT JOIN relations blocks ON blocks.source_slug = n.slug AND blocks.relation_kind = 'blocks'
         LEFT JOIN relations affects ON affects.source_slug = n.slug AND affects.relation_kind = 'affects'
         WHERE n.node_type IN ('risk', 'constraint')
         GROUP BY n.slug
         ORDER BY COUNT(blocks.target_slug) DESC, COUNT(affects.target_slug) DESC, MAX(n.file_mtime_ms) DESC
         LIMIT ?1",
    )?;
    let rows = statement.query_map(params![limit as i64 * 2], |row| row.get::<_, String>(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

fn hotspot_items(conn: &Connection, slugs: &[String]) -> Result<Vec<(String, RiskHotspotItem)>> {
    if slugs.is_empty() {
        return Ok(Vec::new());
    }
    let summaries = node_summaries_for_slugs(conn, slugs, slugs.len(), true)?;
    let relations = relation_targets(conn, slugs)?;
    let order = slugs
        .iter()
        .enumerate()
        .map(|(index, slug)| (slug.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut items = summaries
        .into_iter()
        .filter_map(|node| {
            let normalized = normalize_status(&node.status);
            (normalized != crate::bootstrap::NormalizedStatus::Closed).then(|| {
                let node_type = node.node_type.clone();
                let slug = node.slug.clone();
                (
                    node_type,
                    RiskHotspotItem {
                        id: node.id,
                        slug: slug.clone(),
                        title: node.title,
                        status: node.status,
                        normalized_status: normalized,
                        summary: node.summary,
                        updated_at: node.updated_at,
                        blocks: relations
                            .get(&(slug.clone(), "blocks".to_string()))
                            .cloned()
                            .unwrap_or_default(),
                        affects: relations
                            .get(&(slug, "affects".to_string()))
                            .cloned()
                            .unwrap_or_default(),
                    },
                )
            })
        })
        .collect::<Vec<_>>();
    items.sort_by_key(|(_, item)| order.get(&item.slug).copied().unwrap_or(usize::MAX));
    Ok(items)
}

fn relation_targets(
    conn: &Connection,
    slugs: &[String],
) -> Result<BTreeMap<(String, String), Vec<String>>> {
    let placeholders = vec!["?"; slugs.len()].join(", ");
    let sql = format!(
        "SELECT source_slug, relation_kind, target_slug
         FROM relations
         WHERE source_slug IN ({placeholders}) AND relation_kind IN ('blocks', 'affects')
         ORDER BY source_slug, relation_kind, target_slug"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(params_from_iter(slugs.iter()), |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut mapped = BTreeMap::new();
    for row in rows {
        let (source, kind, target) = row?;
        mapped
            .entry((source, kind))
            .or_insert_with(Vec::new)
            .push(target);
    }
    Ok(mapped)
}
