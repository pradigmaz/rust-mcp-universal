use anyhow::Result;

use super::{QualityRule, RuleContext, metric_with_details, threshold_violation_with_source};
use crate::model::QualityViolationEntry;
use crate::quality::ObservedMetric;

struct CyclomaticComplexityRule;
struct CognitiveComplexityRule;
struct BranchCountMetric;
struct EarlyReturnCountMetric;

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![
        Box::new(CyclomaticComplexityRule),
        Box::new(CognitiveComplexityRule),
        Box::new(BranchCountMetric),
        Box::new(EarlyReturnCountMetric),
    ]
}

impl QualityRule for CyclomaticComplexityRule {
    fn name(&self) -> &'static str {
        "max_cyclomatic_complexity"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        complexity_metric(
            self.name(),
            ctx.facts.hotspots.max_cyclomatic_complexity.as_ref(),
        )
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(complexity_violation(
            ctx,
            self.name(),
            ctx.facts.hotspots.max_cyclomatic_complexity.as_ref(),
            ctx.thresholds.max_cyclomatic_complexity,
            "cyclomatic complexity exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for CognitiveComplexityRule {
    fn name(&self) -> &'static str {
        "max_cognitive_complexity"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        complexity_metric(
            self.name(),
            ctx.facts.hotspots.max_cognitive_complexity.as_ref(),
        )
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(complexity_violation(
            ctx,
            self.name(),
            ctx.facts.hotspots.max_cognitive_complexity.as_ref(),
            ctx.thresholds.max_cognitive_complexity,
            "cognitive complexity exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for BranchCountMetric {
    fn name(&self) -> &'static str {
        "max_branch_count"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        complexity_metric(self.name(), ctx.facts.hotspots.max_branch_count.as_ref())
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

impl QualityRule for EarlyReturnCountMetric {
    fn name(&self) -> &'static str {
        "max_early_return_count"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        complexity_metric(
            self.name(),
            ctx.facts.hotspots.max_early_return_count.as_ref(),
        )
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

fn complexity_metric(
    metric_id: &str,
    observed: Option<&ObservedMetric>,
) -> Option<crate::quality::QualityMetricEntry> {
    observed.map(|value| {
        metric_with_details(
            metric_id,
            value.metric_value,
            value.location.clone(),
            Some(value.source),
        )
    })
}

fn complexity_violation(
    ctx: &RuleContext<'_>,
    rule_id: &str,
    observed: Option<&ObservedMetric>,
    threshold: i64,
    message: &str,
) -> Option<QualityViolationEntry> {
    let observed = observed?;
    threshold_violation_with_source(
        ctx,
        rule_id,
        observed.metric_value,
        threshold,
        message,
        observed.location.clone(),
        Some(observed.source),
    )
}
