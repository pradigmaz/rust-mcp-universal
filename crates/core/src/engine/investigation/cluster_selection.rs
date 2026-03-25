use crate::model::{ConceptSeedKind, ImplementationVariant};

const PREFIX_WINDOW: usize = 4;
const MIN_EXECUTION_VARIANTS_IN_PREFIX: usize = 2;
const EXECUTION_PROMOTION_GAP: f32 = 0.05;

pub(super) fn diversify_variants(
    mut variants: Vec<ImplementationVariant>,
    seed_kind: ConceptSeedKind,
) -> Vec<ImplementationVariant> {
    if seed_kind != ConceptSeedKind::Query || variants.len() < 3 {
        return variants;
    }
    let total_execution_variants = variants.iter().filter(|item| is_execution(item)).count();
    if total_execution_variants == 0 {
        return variants;
    }
    let prefix_len = variants.len().min(PREFIX_WINDOW);
    let required_in_prefix = total_execution_variants.min(MIN_EXECUTION_VARIANTS_IN_PREFIX);
    let mut execution_in_prefix = variants[..1.min(prefix_len)]
        .iter()
        .filter(|item| is_execution(item))
        .count();

    for idx in 1..prefix_len {
        if is_execution(&variants[idx]) {
            execution_in_prefix += 1;
            continue;
        }
        if execution_in_prefix >= required_in_prefix {
            break;
        }
        let current_confidence = variants[idx].confidence;
        let Some(promote_idx) = ((idx + 1)..variants.len()).find(|candidate_idx| {
            is_execution(&variants[*candidate_idx])
                && variants[*candidate_idx].confidence + EXECUTION_PROMOTION_GAP
                    >= current_confidence
        }) else {
            continue;
        };
        variants.swap(idx, promote_idx);
        execution_in_prefix += 1;
    }

    variants
}

fn is_execution(variant: &ImplementationVariant) -> bool {
    matches!(
        variant.entry_anchor.kind.as_deref(),
        Some("Endpoint" | "Service" | "Crud" | "Query")
    )
}

#[cfg(test)]
mod tests {
    use crate::model::{
        ConceptSeedKind, ImplementationVariant, InvestigationAnchor, SemanticState,
        VariantScoreBreakdown,
    };

    use super::diversify_variants;

    #[test]
    fn query_diversification_promotes_execution_variants_into_prefix() {
        let variants = vec![
            variant("ui-a", "Ui", 0.67),
            variant("ui-b", "Ui", 0.67),
            variant("ui-c", "Ui", 0.67),
            variant("endpoint", "Endpoint", 0.668),
            variant("service", "Service", 0.661),
        ];

        let diversified = diversify_variants(variants, ConceptSeedKind::Query);
        let top_paths = diversified
            .iter()
            .take(3)
            .map(|item| item.entry_anchor.path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(top_paths[0], "ui-a");
        assert!(top_paths.contains(&"endpoint"));
        assert!(top_paths.contains(&"service"));
    }

    #[test]
    fn query_diversification_keeps_backend_heavy_prefix_stable() {
        let variants = vec![
            variant("service-a", "Service", 0.655),
            variant("service-b", "Service", 0.652),
            variant("endpoint", "Endpoint", 0.583),
            variant("ui-a", "Ui", 0.545),
        ];

        let diversified = diversify_variants(variants, ConceptSeedKind::Query);
        let top_paths = diversified
            .iter()
            .take(3)
            .map(|item| item.entry_anchor.path.as_str())
            .collect::<Vec<_>>();

        assert_eq!(top_paths, vec!["service-a", "service-b", "endpoint"]);
    }

    fn variant(path: &str, kind: &str, confidence: f32) -> ImplementationVariant {
        ImplementationVariant {
            id: format!("variant:{path}"),
            entry_anchor: InvestigationAnchor {
                path: path.to_string(),
                language: "python".to_string(),
                symbol: None,
                kind: Some(kind.to_string()),
                line: None,
                column: None,
            },
            body_anchor: None,
            route: Vec::new(),
            constraints: Vec::new(),
            related_tests: Vec::new(),
            lexical_proximity: 0.0,
            semantic_proximity: 0.0,
            route_centrality: 0.0,
            symbol_overlap: 0.0,
            constraint_overlap: 0.0,
            test_adjacency: 0.0,
            semantic_state: SemanticState::Used,
            score_model: "heuristic_v2".to_string(),
            score_breakdown: VariantScoreBreakdown::default(),
            confidence,
            gaps: Vec::new(),
        }
    }
}
