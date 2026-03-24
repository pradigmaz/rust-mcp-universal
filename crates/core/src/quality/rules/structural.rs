use anyhow::Result;

use super::{QualityRule, RuleContext, explicit_violation, metric, threshold_violation};
use crate::model::QualityViolationEntry;

struct FanInRule;
struct FanOutRule;
struct CycleMemberRule;
struct HubModuleRule;
struct CrossLayerDependencyRule;
struct OrphanModuleRule;

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![
        Box::new(FanInRule),
        Box::new(FanOutRule),
        Box::new(CycleMemberRule),
        Box::new(HubModuleRule),
        Box::new(CrossLayerDependencyRule),
        Box::new(OrphanModuleRule),
    ]
}

impl QualityRule for FanInRule {
    fn name(&self) -> &'static str {
        "max_fan_in_per_file"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        ctx.facts
            .structural
            .fan_in_count
            .map(|value| metric("fan_in_count", value, None))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(ctx.facts.structural.fan_in_count.and_then(|actual| {
            threshold_violation(
                ctx,
                self.name(),
                actual,
                ctx.thresholds.max_fan_in_per_file,
                "direct fan-in exceeds the allowed threshold",
                None,
            )
        }))
    }
}

impl QualityRule for FanOutRule {
    fn name(&self) -> &'static str {
        "max_fan_out_per_file"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        ctx.facts
            .structural
            .fan_out_count
            .map(|value| metric("fan_out_count", value, None))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(ctx.facts.structural.fan_out_count.and_then(|actual| {
            threshold_violation(
                ctx,
                self.name(),
                actual,
                ctx.thresholds.max_fan_out_per_file,
                "direct fan-out exceeds the allowed threshold",
                None,
            )
        }))
    }
}

impl QualityRule for CycleMemberRule {
    fn name(&self) -> &'static str {
        "module_cycle_member"
    }

    fn metric(&self, _ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        None
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(ctx
            .facts
            .structural
            .cycle_member
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    1,
                    0,
                    "file participates in a direct dependency cycle".to_string(),
                    None,
                    None,
                )
            }))
    }
}

impl QualityRule for HubModuleRule {
    fn name(&self) -> &'static str {
        "hub_module"
    }

    fn metric(&self, _ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        None
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let fan_in = ctx.facts.structural.fan_in_count.unwrap_or_default();
        let fan_out = ctx.facts.structural.fan_out_count.unwrap_or_default();
        Ok((fan_in > ctx.thresholds.max_fan_in_per_file
            && fan_out > ctx.thresholds.max_fan_out_per_file)
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    fan_in.saturating_add(fan_out),
                    ctx.thresholds
                        .max_fan_in_per_file
                        .saturating_add(ctx.thresholds.max_fan_out_per_file),
                    format!("file is a structural hub with fan-in {fan_in} and fan-out {fan_out}"),
                    None,
                    None,
                )
            }))
    }
}

impl QualityRule for CrossLayerDependencyRule {
    fn name(&self) -> &'static str {
        "cross_layer_dependency"
    }

    fn metric(&self, _ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        None
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(ctx
            .facts
            .structural
            .cross_layer
            .as_ref()
            .map(|facts| {
                explicit_violation(
                    ctx,
                    self.name(),
                    facts.edge_count,
                    0,
                    facts.message.clone(),
                    None,
                    None,
                )
            }))
    }
}

impl QualityRule for OrphanModuleRule {
    fn name(&self) -> &'static str {
        "orphan_module"
    }

    fn metric(&self, _ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        None
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(ctx
            .facts
            .structural
            .orphan_module
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    1,
                    0,
                    "file is isolated from the direct dependency graph".to_string(),
                    None,
                    None,
                )
            }))
    }
}
