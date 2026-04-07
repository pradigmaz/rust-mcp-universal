use anyhow::Result;

use super::{QualityRule, RuleContext, explicit_violation, metric_with_details};
use crate::model::{QualitySource, QualityViolationEntry};

struct NearbyTestCountRule;
struct NearbyIntegrationTestCountRule;
struct PublicSurfaceWithoutTestsRule;
struct HotspotWithoutTestsRule;
struct IntegrationEntryWithoutTestsRule;

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![
        Box::new(NearbyTestCountRule),
        Box::new(NearbyIntegrationTestCountRule),
        Box::new(PublicSurfaceWithoutTestsRule),
        Box::new(HotspotWithoutTestsRule),
        Box::new(IntegrationEntryWithoutTestsRule),
    ]
}

impl QualityRule for NearbyTestCountRule {
    fn name(&self) -> &'static str {
        "public_surface_without_tests"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "test_nearby_test_file_count",
            ctx.facts.test_risk.nearby_test_file_count,
            None,
            Some(QualitySource::Test),
        ))
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

impl QualityRule for NearbyIntegrationTestCountRule {
    fn name(&self) -> &'static str {
        "integration_entry_without_tests"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "test_nearby_integration_test_file_count",
            ctx.facts.test_risk.nearby_integration_test_file_count,
            None,
            Some(QualitySource::Test),
        ))
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

impl QualityRule for PublicSurfaceWithoutTestsRule {
    fn name(&self) -> &'static str {
        "public_surface_without_tests"
    }

    fn metric(&self, _ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        None
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok((ctx.facts.test_risk.has_public_surface
            && ctx.facts.test_risk.nearby_test_file_count == 0)
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    1,
                    0,
                    "public surface has no nearby test evidence".to_string(),
                    None,
                    Some(QualitySource::Test),
                )
            }))
    }
}

impl QualityRule for HotspotWithoutTestsRule {
    fn name(&self) -> &'static str {
        "hotspot_without_test_evidence"
    }

    fn metric(&self, _ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        None
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok((ctx.facts.test_risk.is_hotspot_candidate
            && ctx.facts.test_risk.nearby_test_file_count == 0)
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    1,
                    0,
                    "hotspot candidate has no nearby test evidence".to_string(),
                    None,
                    Some(QualitySource::Test),
                )
            }))
    }
}

impl QualityRule for IntegrationEntryWithoutTestsRule {
    fn name(&self) -> &'static str {
        "integration_entry_without_tests"
    }

    fn metric(&self, _ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        None
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok((ctx.facts.test_risk.is_integration_entry
            && ctx.facts.test_risk.nearby_integration_test_file_count == 0)
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    1,
                    0,
                    "integration entry has no nearby integration-test evidence".to_string(),
                    None,
                    Some(QualitySource::Test),
                )
            }))
    }
}
