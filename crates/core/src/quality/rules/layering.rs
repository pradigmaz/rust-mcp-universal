use anyhow::Result;

use super::{QualityRule, RuleContext, explicit_violation, metric_with_details};
use crate::model::{QualitySource, QualityViolationEntry};

struct LayeringForbiddenEdgeRule;
struct LayeringUnmatchedZoneRule;

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![
        Box::new(LayeringForbiddenEdgeRule),
        Box::new(LayeringUnmatchedZoneRule),
    ]
}

impl QualityRule for LayeringForbiddenEdgeRule {
    fn name(&self) -> &'static str {
        "cross_layer_dependency"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "layering_forbidden_edge_count",
            ctx.facts.layering.forbidden_edge_count,
            None,
            Some(QualitySource::Graph),
        ))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(ctx.facts.layering.violation_summary().and_then(|summary| {
            (ctx.facts.layering.forbidden_edge_count > 0
                || ctx.facts.layering.out_of_direction_edge_count > 0)
                .then(|| {
                    explicit_violation(
                        ctx,
                        self.name(),
                        summary.edge_count,
                        0,
                        summary.message,
                        None,
                        Some(QualitySource::Graph),
                    )
                })
        }))
    }
}

impl QualityRule for LayeringUnmatchedZoneRule {
    fn name(&self) -> &'static str {
        "layering_unmatched_zone_dependency"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "layering_unmatched_edge_count",
            ctx.facts.layering.unmatched_edge_count,
            None,
            Some(QualitySource::Graph),
        ))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok((ctx.facts.layering.unmatched_edge_count > 0).then(|| {
            explicit_violation(
                ctx,
                self.name(),
                ctx.facts.layering.unmatched_edge_count,
                0,
                ctx.facts
                    .layering
                    .primary_message
                    .clone()
                    .unwrap_or_else(|| "zone dependency touches an unmatched path".to_string()),
                None,
                Some(QualitySource::Graph),
            )
        }))
    }
}
