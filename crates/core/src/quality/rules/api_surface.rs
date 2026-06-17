use anyhow::Result;

use super::{
    QualityRule, RuleContext, explicit_violation, metric_with_details,
    threshold_violation_with_source,
};
use crate::model::{QualitySource, QualityViolationEntry};

struct WidePublicApiSurfaceRule;
struct PublicReexportHubRule;
struct PublicApiHubRule;
struct UnstablePublicHubRule;
struct RestrictedExportMetricRule;
struct PublicTypeMetricRule;
struct PublicFunctionMetricRule;

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![
        Box::new(WidePublicApiSurfaceRule),
        Box::new(PublicReexportHubRule),
        Box::new(PublicApiHubRule),
        Box::new(UnstablePublicHubRule),
        Box::new(RestrictedExportMetricRule),
        Box::new(PublicTypeMetricRule),
        Box::new(PublicFunctionMetricRule),
    ]
}

impl QualityRule for WidePublicApiSurfaceRule {
    fn name(&self) -> &'static str {
        "wide_public_api_surface"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        public_surface_metric(
            ctx,
            "public_api_export_count",
            ctx.facts.api_surface.public_export_count,
        )
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(threshold_violation_with_source(
            ctx,
            self.name(),
            ctx.facts.api_surface.public_export_count,
            ctx.thresholds.max_public_api_exports_per_file,
            "public API surface exceeds the allowed export threshold",
            ctx.facts.api_surface.primary_location.clone(),
            Some(QualitySource::ParserLight),
        ))
    }
}

impl QualityRule for PublicReexportHubRule {
    fn name(&self) -> &'static str {
        "public_reexport_hub"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        public_surface_metric(
            ctx,
            "public_api_reexport_count",
            ctx.facts.api_surface.public_reexport_count,
        )
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(threshold_violation_with_source(
            ctx,
            self.name(),
            ctx.facts.api_surface.public_reexport_count,
            ctx.thresholds.max_public_reexports_per_file,
            "public re-export hub exposes too many external contracts",
            ctx.facts.api_surface.primary_location.clone(),
            Some(QualitySource::ParserLight),
        ))
    }
}

impl QualityRule for PublicApiHubRule {
    fn name(&self) -> &'static str {
        "public_api_hub"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        let score = public_api_hub_score(ctx);
        (score > 0).then(|| {
            metric_with_details(
                "public_api_hub_score",
                score,
                None,
                Some(QualitySource::Graph),
            )
        })
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let fan_in = ctx.facts.structural.fan_in_count.unwrap_or_default();
        let score = public_api_hub_score(ctx);
        Ok((fan_in > ctx.thresholds.max_fan_in_per_file
            && score > ctx.thresholds.max_public_api_hub_score)
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    score,
                    ctx.thresholds.max_public_api_hub_score,
                    format!(
                        "public API hub has {} exports and fan-in {fan_in}",
                        ctx.facts.api_surface.public_export_count
                    ),
                    ctx.facts.api_surface.primary_location.clone(),
                    Some(QualitySource::Graph),
                )
            }))
    }
}

impl QualityRule for UnstablePublicHubRule {
    fn name(&self) -> &'static str {
        "unstable_public_hub"
    }

    fn metric(&self, _ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        None
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let fan_in = ctx.facts.structural.fan_in_count.unwrap_or_default();
        let score = public_api_hub_score(ctx);
        let churn = ctx.facts.git_risk.recent_churn_lines;
        Ok((fan_in > ctx.thresholds.max_fan_in_per_file
            && score > ctx.thresholds.max_public_api_hub_score
            && churn > ctx.effective_policy.git_risk.max_recent_churn_lines_per_file)
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    churn,
                    ctx.effective_policy.git_risk.max_recent_churn_lines_per_file,
                    format!(
                        "public API hub is unstable: hub score {score}, fan-in {fan_in}, recent churn {churn} lines",
                    ),
                    ctx.facts.api_surface.primary_location.clone(),
                    Some(QualitySource::Git),
                )
            }))
    }
}

impl QualityRule for RestrictedExportMetricRule {
    fn name(&self) -> &'static str {
        "public_api_restricted_export_count_metric"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        public_surface_metric(
            ctx,
            "public_api_restricted_export_count",
            ctx.facts.api_surface.restricted_export_count,
        )
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

impl QualityRule for PublicTypeMetricRule {
    fn name(&self) -> &'static str {
        "public_api_type_count_metric"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        public_surface_metric(
            ctx,
            "public_api_type_count",
            ctx.facts.api_surface.public_type_count,
        )
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

impl QualityRule for PublicFunctionMetricRule {
    fn name(&self) -> &'static str {
        "public_api_function_count_metric"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        public_surface_metric(
            ctx,
            "public_api_function_count",
            ctx.facts.api_surface.public_function_count,
        )
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

fn public_surface_metric(
    ctx: &RuleContext<'_>,
    metric_id: &str,
    value: i64,
) -> Option<crate::quality::QualityMetricEntry> {
    (value > 0).then(|| {
        metric_with_details(
            metric_id,
            value,
            ctx.facts.api_surface.primary_location.clone(),
            Some(QualitySource::ParserLight),
        )
    })
}

fn public_api_hub_score(ctx: &RuleContext<'_>) -> i64 {
    ctx.facts
        .api_surface
        .public_export_count
        .saturating_mul(ctx.facts.structural.fan_in_count.unwrap_or_default())
}
