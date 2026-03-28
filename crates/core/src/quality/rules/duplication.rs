use anyhow::Result;

use super::{QualityRule, RuleContext, metric_with_details, threshold_violation_with_source};
use crate::model::{QualitySource, QualityViolationEntry};

struct DuplicateBlockCountRule;
struct DuplicateDensityRule;
struct DuplicatePeerCountMetric;
struct DuplicateLinesMetric;
struct MaxDuplicateBlockTokensMetric;
struct MaxDuplicateSimilarityMetric;

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![
        Box::new(DuplicateBlockCountRule),
        Box::new(DuplicateDensityRule),
        Box::new(DuplicatePeerCountMetric),
        Box::new(DuplicateLinesMetric),
        Box::new(MaxDuplicateBlockTokensMetric),
        Box::new(MaxDuplicateSimilarityMetric),
    ]
}

impl QualityRule for DuplicateBlockCountRule {
    fn name(&self) -> &'static str {
        "max_duplicate_block_count"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "duplicate_block_count",
            ctx.facts.duplication.duplicate_block_count,
            ctx.facts.duplication.primary_location.clone(),
            Some(QualitySource::Duplication),
        ))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(threshold_violation_with_source(
            ctx,
            self.name(),
            ctx.facts.duplication.duplicate_block_count,
            ctx.thresholds.max_duplicate_block_count,
            "duplicate block count exceeds the allowed threshold",
            ctx.facts.duplication.primary_location.clone(),
            Some(QualitySource::Duplication),
        ))
    }
}

impl QualityRule for DuplicateDensityRule {
    fn name(&self) -> &'static str {
        "max_duplicate_density_bps"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "duplicate_density_bps",
            ctx.facts.duplication.duplicate_density_bps,
            ctx.facts.duplication.primary_location.clone(),
            Some(QualitySource::Duplication),
        ))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(threshold_violation_with_source(
            ctx,
            self.name(),
            ctx.facts.duplication.duplicate_density_bps,
            ctx.thresholds.max_duplicate_density_bps,
            "duplicate density exceeds the allowed threshold",
            ctx.facts.duplication.primary_location.clone(),
            Some(QualitySource::Duplication),
        ))
    }
}

impl QualityRule for DuplicatePeerCountMetric {
    fn name(&self) -> &'static str {
        "duplicate_peer_count"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            self.name(),
            ctx.facts.duplication.duplicate_peer_count,
            None,
            Some(QualitySource::Duplication),
        ))
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

impl QualityRule for DuplicateLinesMetric {
    fn name(&self) -> &'static str {
        "duplicate_lines"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            self.name(),
            ctx.facts.duplication.duplicate_lines,
            ctx.facts.duplication.primary_location.clone(),
            Some(QualitySource::Duplication),
        ))
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

impl QualityRule for MaxDuplicateBlockTokensMetric {
    fn name(&self) -> &'static str {
        "max_duplicate_block_tokens"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            self.name(),
            ctx.facts.duplication.max_duplicate_block_tokens,
            ctx.facts.duplication.primary_location.clone(),
            Some(QualitySource::Duplication),
        ))
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

impl QualityRule for MaxDuplicateSimilarityMetric {
    fn name(&self) -> &'static str {
        "max_duplicate_similarity_percent"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            self.name(),
            ctx.facts.duplication.max_duplicate_similarity_percent,
            ctx.facts.duplication.primary_location.clone(),
            Some(QualitySource::Duplication),
        ))
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}
