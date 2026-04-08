use anyhow::Result;

use super::{QualityRule, RuleContext, metric, signal_violation};
use crate::model::{FindingFamily, QualitySource, QualityViolationEntry};

struct DeadCodeRule;

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![Box::new(DeadCodeRule)]
}

impl QualityRule for DeadCodeRule {
    fn name(&self) -> &'static str {
        "dead_code_unused_export_candidate"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        (ctx.facts.dead_code.exported_symbol_count > 0).then(|| {
            metric(
                "dead_code_exported_symbol_count",
                ctx.facts.dead_code.exported_symbol_count,
                ctx.facts.dead_code.location.clone(),
            )
        })
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        if !ctx.facts.dead_code.candidate {
            return Ok(None);
        }
        let fan_in = ctx.facts.structural.fan_in_count.unwrap_or_default();
        let fan_out = ctx.facts.structural.fan_out_count.unwrap_or_default();
        let symbol_count = ctx.indexed_metrics.symbol_count.unwrap_or_default();
        let isolated = ctx.facts.structural.orphan_module || (fan_in == 0 && fan_out <= 1);
        if !isolated || symbol_count > 24 {
            return Ok(None);
        }

        Ok(Some(signal_violation(
            ctx,
            self.name(),
            1,
            0,
            format!(
                "file looks like a dead-code candidate: {} exported symbols, fan-in {fan_in}, fan-out {fan_out}, symbol_count {symbol_count}",
                ctx.facts.dead_code.exported_symbol_count
            ),
            ctx.facts.dead_code.location.clone(),
            Some(QualitySource::Heuristic),
            FindingFamily::DeadCode,
            ctx.facts.dead_code.confidence,
            ctx.facts.dead_code.noise_reason.clone(),
            vec![
                "confirm runtime registration and reflection use before removal".to_string(),
                "prefer warning-first cleanup instead of direct deletion".to_string(),
            ],
        )))
    }
}
