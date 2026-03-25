use std::collections::HashSet;

use crate::model::{ConstraintEvidence, ImplementationVariant};

pub(super) fn normalized_constraint_items(
    variants: &[ImplementationVariant],
) -> Vec<ConstraintEvidence> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    for item in variants
        .iter()
        .flat_map(|variant| variant.constraints.iter())
    {
        if seen.insert((
            item.path.clone(),
            item.line_start,
            item.constraint_kind.clone(),
            item.normalized_key.clone(),
        )) {
            items.push(item.clone());
        }
    }
    items.sort_by(|left, right| {
        constraint_priority(left)
            .cmp(&constraint_priority(right))
            .then_with(|| right.confidence.total_cmp(&left.confidence))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    items
}

fn constraint_priority(item: &ConstraintEvidence) -> usize {
    match (item.strength.as_str(), item.source_kind.as_str()) {
        ("strong", "index_declaration") => 0,
        ("strong", "migration_declaration") => 1,
        ("strong", "model_declaration") => 2,
        ("strong", "schema_hint") => 3,
        ("weak", "schema_hint") => 4,
        _ => 5,
    }
}
