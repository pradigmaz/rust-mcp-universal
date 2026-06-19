use std::collections::BTreeSet;

use crate::model::QualitySuppression;

use super::QualitySuppressionPolicy;

#[derive(Debug, Clone)]
pub(crate) struct QualitySuppressionMatch {
    pub(crate) suppression_id: String,
    pub(crate) reason: String,
    pub(crate) scope_id: Option<String>,
    pub(super) rule_ids: BTreeSet<String>,
}

pub(super) fn matching_suppressions(
    suppressions: &[QualitySuppressionPolicy],
    rel_path: &str,
) -> Vec<QualitySuppressionMatch> {
    suppressions
        .iter()
        .filter(|suppression| suppression.matcher.matches(rel_path))
        .map(|suppression| QualitySuppressionMatch {
            suppression_id: suppression.suppression_id.clone(),
            reason: suppression.reason.clone(),
            scope_id: suppression.scope_id.clone(),
            rule_ids: suppression.rule_ids.clone(),
        })
        .collect()
}

pub(super) fn suppressions_for_rule(
    suppression_matches: &[QualitySuppressionMatch],
    rule_id: &str,
) -> Vec<QualitySuppression> {
    suppression_matches
        .iter()
        .filter(|suppression| suppression.rule_ids.contains(rule_id))
        .map(|suppression| QualitySuppression {
            suppression_id: suppression.suppression_id.clone(),
            reason: suppression.reason.clone(),
            scope_id: suppression.scope_id.clone(),
        })
        .collect()
}
