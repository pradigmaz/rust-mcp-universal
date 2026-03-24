use anyhow::Result;

use super::{QualityRule, RuleContext, metric_with_details, threshold_violation_with_source};
use crate::model::QualityViolationEntry;
use crate::quality::{HotspotFacts, ObservedMetric};

#[path = "file_hotspots_common.rs"]
mod file_hotspots_common;
#[path = "file_hotspots_javascript.rs"]
mod file_hotspots_javascript;
#[path = "file_hotspots_python.rs"]
mod file_hotspots_python;
#[path = "file_hotspots_rust.rs"]
mod file_hotspots_rust;

struct FunctionLinesRule;
struct NestingDepthRule;
struct ParametersPerFunctionRule;
struct ExportCountRule;
struct ClassMemberCountRule;
struct TodoCountRule;

pub(crate) fn analyze_hotspots(rel_path: &str, language: &str, full_text: &str) -> HotspotFacts {
    let mut facts = match language {
        "javascript" | "jsx" | "mjs" | "cjs" | "typescript" | "tsx" => {
            file_hotspots_javascript::analyze(rel_path, language, full_text)
        }
        "python" => file_hotspots_python::analyze(full_text),
        "rust" => file_hotspots_rust::analyze(full_text),
        _ => HotspotFacts::default(),
    };
    facts.max_todo_count_per_file = file_hotspots_common::todo_metric(full_text);
    facts
}

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![
        Box::new(FunctionLinesRule),
        Box::new(NestingDepthRule),
        Box::new(ParametersPerFunctionRule),
        Box::new(ExportCountRule),
        Box::new(ClassMemberCountRule),
        Box::new(TodoCountRule),
    ]
}

impl QualityRule for FunctionLinesRule {
    fn name(&self) -> &'static str {
        "max_function_lines"
    }
    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        hotspot_metric(self.name(), ctx.facts.hotspots.max_function_lines.as_ref())
    }
    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(hotspot_violation(
            ctx,
            self.name(),
            ctx.facts.hotspots.max_function_lines.as_ref(),
            ctx.thresholds.max_function_lines,
            "function or method length exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for NestingDepthRule {
    fn name(&self) -> &'static str {
        "max_nesting_depth"
    }
    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        hotspot_metric(self.name(), ctx.facts.hotspots.max_nesting_depth.as_ref())
    }
    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(hotspot_violation(
            ctx,
            self.name(),
            ctx.facts.hotspots.max_nesting_depth.as_ref(),
            ctx.thresholds.max_nesting_depth,
            "nesting depth exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for ParametersPerFunctionRule {
    fn name(&self) -> &'static str {
        "max_parameters_per_function"
    }
    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        hotspot_metric(
            self.name(),
            ctx.facts.hotspots.max_parameters_per_function.as_ref(),
        )
    }
    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(hotspot_violation(
            ctx,
            self.name(),
            ctx.facts.hotspots.max_parameters_per_function.as_ref(),
            ctx.thresholds.max_parameters_per_function,
            "function parameter count exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for ExportCountRule {
    fn name(&self) -> &'static str {
        "max_export_count_per_file"
    }
    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        hotspot_metric(
            self.name(),
            ctx.facts.hotspots.max_export_count_per_file.as_ref(),
        )
    }
    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(hotspot_violation(
            ctx,
            self.name(),
            ctx.facts.hotspots.max_export_count_per_file.as_ref(),
            ctx.thresholds.max_export_count_per_file,
            "export count exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for ClassMemberCountRule {
    fn name(&self) -> &'static str {
        "max_class_member_count"
    }
    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        hotspot_metric(
            self.name(),
            ctx.facts.hotspots.max_class_member_count.as_ref(),
        )
    }
    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(hotspot_violation(
            ctx,
            self.name(),
            ctx.facts.hotspots.max_class_member_count.as_ref(),
            ctx.thresholds.max_class_member_count,
            "class member count exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for TodoCountRule {
    fn name(&self) -> &'static str {
        "max_todo_count_per_file"
    }
    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        hotspot_metric(
            self.name(),
            ctx.facts.hotspots.max_todo_count_per_file.as_ref(),
        )
    }
    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(hotspot_violation(
            ctx,
            self.name(),
            ctx.facts.hotspots.max_todo_count_per_file.as_ref(),
            ctx.thresholds.max_todo_count_per_file,
            "TODO-style marker count exceeds the allowed threshold",
        ))
    }
}

fn hotspot_metric(
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

fn hotspot_violation(
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
