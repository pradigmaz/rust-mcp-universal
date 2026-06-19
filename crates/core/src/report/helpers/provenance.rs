use crate::model::{
    CanonicalBasis, CanonicalFreshness, CanonicalProvenance, CanonicalStrength,
    RankExplainBreakdown,
};

pub(crate) fn canonical_provenance_for_context_item(
    chunk_source: &str,
    explain: RankExplainBreakdown,
    score: f32,
) -> CanonicalProvenance {
    let preview_fallback = chunk_source == "preview_fallback";
    let graph_derived =
        explain.graph > 0.0 || explain.graph_hops > 0 || !explain.graph_seed_path.is_empty();
    let indexed = !chunk_source.is_empty() && !preview_fallback;
    let heuristic = explain.semantic_source == "none"
        || matches!(
            explain.semantic_outcome.as_str(),
            "not_requested" | "short_circuit_lexical" | "not_applied"
        );

    let basis = match (preview_fallback, graph_derived, indexed, heuristic) {
        (true, true, _, _) | (true, _, true, _) | (false, true, true, _) => CanonicalBasis::Mixed,
        (true, false, false, _) => CanonicalBasis::PreviewFallback,
        (false, true, false, _) => CanonicalBasis::GraphDerived,
        (false, false, true, _) => CanonicalBasis::Indexed,
        _ => CanonicalBasis::Heuristic,
    };
    let freshness = if preview_fallback {
        CanonicalFreshness::LiveRead
    } else if indexed || graph_derived {
        CanonicalFreshness::IndexSnapshot
    } else {
        CanonicalFreshness::Unknown
    };
    let strength = if preview_fallback {
        CanonicalStrength::FallbackOnly
    } else if matches!(
        explain.semantic_outcome.as_str(),
        "applied_indexed" | "applied_mixed"
    ) || graph_derived
    {
        CanonicalStrength::Strong
    } else if score >= 0.15 || explain.semantic_source != "none" {
        CanonicalStrength::Moderate
    } else {
        CanonicalStrength::Weak
    };

    let mut reasons = Vec::new();
    reasons.push(format!("chunk_source:{chunk_source}"));
    reasons.push(format!("semantic_source:{}", explain.semantic_source));
    reasons.push(format!("semantic_outcome:{}", explain.semantic_outcome));
    if explain.graph_hops > 0 {
        reasons.push(format!("graph_hops:{}", explain.graph_hops));
    }
    if !explain.graph_seed_path.is_empty() {
        reasons.push(format!("graph_seed_path:{}", explain.graph_seed_path));
    }

    CanonicalProvenance {
        basis,
        derivation: "context_selection".to_string(),
        freshness,
        strength,
        reasons,
    }
}

pub(crate) fn summarize_provenance(
    inputs: &[CanonicalProvenance],
    derivation: &str,
) -> CanonicalProvenance {
    if inputs.is_empty() {
        return CanonicalProvenance {
            basis: CanonicalBasis::Heuristic,
            derivation: derivation.to_string(),
            freshness: CanonicalFreshness::Unknown,
            strength: CanonicalStrength::Weak,
            reasons: vec!["no_surface_evidence".to_string()],
        };
    }

    let basis = {
        let mut counts = std::collections::HashMap::<CanonicalBasis, usize>::new();
        for item in inputs {
            *counts.entry(item.basis).or_default() += 1;
        }
        if counts.len() > 1 {
            CanonicalBasis::Mixed
        } else {
            counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(basis, _)| basis)
                .unwrap_or(CanonicalBasis::Heuristic)
        }
    };

    let freshness = if inputs
        .iter()
        .all(|item| item.freshness == CanonicalFreshness::LiveRead)
    {
        CanonicalFreshness::LiveRead
    } else if inputs
        .iter()
        .any(|item| item.freshness == CanonicalFreshness::IndexSnapshot)
    {
        CanonicalFreshness::IndexSnapshot
    } else {
        CanonicalFreshness::Unknown
    };

    let strength = if inputs
        .iter()
        .all(|item| item.strength == CanonicalStrength::FallbackOnly)
    {
        CanonicalStrength::FallbackOnly
    } else if inputs
        .iter()
        .any(|item| item.strength == CanonicalStrength::Strong)
    {
        CanonicalStrength::Strong
    } else if inputs
        .iter()
        .any(|item| item.strength == CanonicalStrength::Moderate)
    {
        CanonicalStrength::Moderate
    } else {
        CanonicalStrength::Weak
    };

    let mut reasons = inputs
        .iter()
        .flat_map(|item| item.reasons.iter().cloned())
        .fold(Vec::<String>::new(), |mut acc, reason| {
            if !acc.contains(&reason) {
                acc.push(reason);
            }
            acc
        });
    reasons.truncate(8);
    reasons.insert(
        0,
        format!("dominant_basis:{:?}", basis).to_ascii_lowercase(),
    );

    CanonicalProvenance {
        basis,
        derivation: derivation.to_string(),
        freshness,
        strength,
        reasons,
    }
}
