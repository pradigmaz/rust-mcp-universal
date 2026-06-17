#[path = "bootstrap_risks.rs"]
mod risks;
#[path = "bootstrap_support.rs"]
mod support;

use std::collections::BTreeSet;

use anyhow::Result;

use crate::bootstrap::{
    ContextPack, ContextPackBudget, ContextPackNode, DecisionLogEntry, RecentChangeItem,
    RiskHotspots, mark_node_summary_status,
};
use crate::model::ProjectBrief;

use super::Engine;
use super::query::{project_name, read_counts};
use risks::{matching_risks, query_risk_hotspots};
use support::{
    budget_exhausted, map_recent_change, matching_recent_changes, project_summary,
    recent_decisions, recent_node_summaries, related_context_nodes, search_decisions,
    search_node_summaries, unresolved_node_summaries,
};

pub(super) fn project_brief(engine: &Engine) -> Result<ProjectBrief> {
    let conn = engine.open_connection()?;
    let counts = read_counts(&conn)?;
    let top_decisions = recent_node_summaries(&conn, Some(&["decision"]), 5, true)?;
    let top_risks = unresolved_node_summaries(&conn, 5)?
        .into_iter()
        .map(mark_node_summary_status)
        .collect();
    let recent_changes = recent_node_summaries(&conn, None, 8, true)?;

    Ok(ProjectBrief {
        project: project_name(engine),
        summary: project_summary(&conn)?.unwrap_or_else(|| {
            format!(
                "Indexed {} notes, {} decisions, {} risks, {} constraints, {} relations, {} observations.",
                counts.notes,
                counts.decisions,
                counts.risks,
                counts.constraints,
                counts.relations,
                counts.observations
            )
        }),
        top_decisions,
        top_risks,
        recent_changes,
    })
}

pub(super) fn recent_changes(engine: &Engine, limit: usize) -> Result<Vec<RecentChangeItem>> {
    let conn = engine.open_connection()?;
    let nodes = recent_node_summaries(&conn, None, limit, false)?;
    Ok(nodes.into_iter().map(map_recent_change).collect())
}

pub(super) fn decision_log(
    engine: &Engine,
    topic: Option<&str>,
    limit: usize,
) -> Result<Vec<DecisionLogEntry>> {
    let conn = engine.open_connection()?;
    match topic.map(str::trim).filter(|value| !value.is_empty()) {
        Some(topic) => search_decisions(&conn, topic, limit),
        None => recent_decisions(&conn, limit),
    }
}

pub(super) fn risk_hotspots(engine: &Engine, limit: usize) -> Result<RiskHotspots> {
    let conn = engine.open_connection()?;
    query_risk_hotspots(&conn, limit)
}

pub(super) fn context_pack(
    engine: &Engine,
    seed: &str,
    limit: usize,
    max_chars: usize,
    max_tokens: usize,
) -> Result<ContextPack> {
    let conn = engine.open_connection()?;
    let mut brief = project_brief(engine)?;
    let mut truncated = trim_brief_to_budget(&mut brief, max_chars, max_tokens)?;
    let direct = search_node_summaries(&conn, seed, limit, false)?;
    let direct_slugs = direct
        .iter()
        .map(|node| node.slug.clone())
        .collect::<Vec<_>>();
    let related = related_context_nodes(&conn, &direct_slugs, limit)?;
    let risks = matching_risks(&conn, seed, &direct_slugs, limit)?;
    let recent_changes = matching_recent_changes(&conn, seed, &direct_slugs)?;

    let mut used_chars = serde_json::to_string(&brief)?.len();
    let mut included_nodes = Vec::new();
    let mut seen = BTreeSet::new();
    truncated |= budget_exhausted(used_chars, max_chars, max_tokens);

    for (node, why) in direct
        .into_iter()
        .map(|node| (node, "direct_match"))
        .chain(related.into_iter().map(|node| (node, "related")))
    {
        if !seen.insert(node.slug.clone()) {
            continue;
        }
        let candidate = ContextPackNode {
            id: node.id,
            slug: node.slug,
            title: node.title,
            node_type: node.node_type,
            summary: node.summary,
            updated_at: node.updated_at,
            why_included: why.to_string(),
        };
        let candidate_chars = serde_json::to_string(&candidate)?.len();
        if budget_exhausted(used_chars + candidate_chars, max_chars, max_tokens) {
            truncated = true;
            break;
        }
        used_chars += candidate_chars;
        included_nodes.push(candidate);
    }

    let mut packed_changes = Vec::new();
    for change in recent_changes {
        let candidate_chars = serde_json::to_string(&change)?.len();
        if budget_exhausted(used_chars + candidate_chars, max_chars, max_tokens) {
            truncated = true;
            break;
        }
        used_chars += candidate_chars;
        packed_changes.push(change);
    }

    let mut packed_risks = Vec::new();
    for risk in risks {
        let candidate_chars = serde_json::to_string(&risk)?.len();
        if budget_exhausted(used_chars + candidate_chars, max_chars, max_tokens) {
            truncated = true;
            break;
        }
        used_chars += candidate_chars;
        packed_risks.push(risk);
    }

    Ok(ContextPack {
        seed: seed.trim().to_string(),
        brief,
        included_nodes,
        recent_changes: packed_changes,
        risks: packed_risks,
        budget: ContextPackBudget {
            max_chars,
            max_tokens,
            used_chars,
            truncated,
        },
    })
}

fn trim_brief_to_budget(
    brief: &mut ProjectBrief,
    max_chars: usize,
    max_tokens: usize,
) -> Result<bool> {
    let mut truncated = false;
    if !budget_exhausted(serde_json::to_string(brief)?.len(), max_chars, max_tokens) {
        return Ok(false);
    }

    if !brief.recent_changes.is_empty() {
        brief.recent_changes.clear();
        truncated = true;
    }
    if !budget_exhausted(serde_json::to_string(brief)?.len(), max_chars, max_tokens) {
        return Ok(truncated);
    }
    if !brief.top_risks.is_empty() {
        brief.top_risks.clear();
        truncated = true;
    }
    if !budget_exhausted(serde_json::to_string(brief)?.len(), max_chars, max_tokens) {
        return Ok(truncated);
    }
    if !brief.top_decisions.is_empty() {
        brief.top_decisions.clear();
        truncated = true;
    }
    if !budget_exhausted(serde_json::to_string(brief)?.len(), max_chars, max_tokens) {
        return Ok(truncated);
    }

    while !brief.summary.is_empty()
        && budget_exhausted(serde_json::to_string(brief)?.len(), max_chars, max_tokens)
    {
        let keep = brief.summary.len() / 2;
        brief.summary.truncate(keep);
        truncated = true;
    }
    Ok(truncated)
}

#[cfg(test)]
#[path = "bootstrap_tests.rs"]
mod tests;
