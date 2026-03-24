use anyhow::Result;

use super::{QualityRule, RuleContext, metric, threshold_violation};
use crate::model::QualityViolationEntry;
use crate::quality::metrics::FileKind;

struct SizeBytesRule;
struct NonEmptyLinesRule;
struct ImportCountRule;
struct MaxLineLengthRule;
struct SymbolCountRule;
struct RefCountRule;
struct ModuleDepCountRule;
struct GraphEdgeOutCountRule;

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![
        Box::new(SizeBytesRule),
        Box::new(NonEmptyLinesRule),
        Box::new(ImportCountRule),
        Box::new(MaxLineLengthRule),
        Box::new(SymbolCountRule),
        Box::new(RefCountRule),
        Box::new(ModuleDepCountRule),
        Box::new(GraphEdgeOutCountRule),
    ]
}

impl QualityRule for SizeBytesRule {
    fn name(&self) -> &'static str {
        "max_size_bytes"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric("size_bytes", ctx.facts.size_bytes, None))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(threshold_violation(
            ctx,
            self.name(),
            ctx.facts.size_bytes,
            ctx.thresholds.max_size_bytes,
            "file size exceeds the allowed threshold",
            None,
        ))
    }
}

impl QualityRule for NonEmptyLinesRule {
    fn name(&self) -> &'static str {
        "max_non_empty_lines"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        ctx.facts
            .non_empty_lines
            .map(|value| metric("non_empty_lines", value, None))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.facts.non_empty_lines else {
            return Ok(None);
        };
        let (rule_id, threshold) = match ctx.file_kind {
            FileKind::Default => (
                "max_non_empty_lines_default",
                ctx.thresholds.max_non_empty_lines_default,
            ),
            FileKind::Test => (
                "max_non_empty_lines_test",
                ctx.thresholds.max_non_empty_lines_test,
            ),
            FileKind::Config => (
                "max_non_empty_lines_config",
                ctx.thresholds.max_non_empty_lines_config,
            ),
        };
        Ok(threshold_violation(
            ctx,
            rule_id,
            actual,
            threshold,
            "non-empty line count exceeds the allowed threshold",
            None,
        ))
    }
}

impl QualityRule for ImportCountRule {
    fn name(&self) -> &'static str {
        "max_import_count"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        ctx.facts
            .import_count
            .map(|value| metric("import_count", value, ctx.facts.import_region.clone()))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.facts.import_count else {
            return Ok(None);
        };
        Ok(threshold_violation(
            ctx,
            self.name(),
            actual,
            ctx.thresholds.max_import_count,
            "import count exceeds the allowed threshold",
            ctx.facts.import_region.clone(),
        ))
    }
}

impl QualityRule for MaxLineLengthRule {
    fn name(&self) -> &'static str {
        "max_line_length"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        ctx.facts.max_line_length.map(|value| {
            metric(
                "max_line_length",
                value,
                ctx.facts.max_line_length_location.clone(),
            )
        })
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.facts.max_line_length else {
            return Ok(None);
        };
        Ok(threshold_violation(
            ctx,
            self.name(),
            actual,
            ctx.thresholds.max_line_length,
            "maximum line length exceeds the allowed threshold",
            ctx.facts.max_line_length_location.clone(),
        ))
    }
}

impl QualityRule for SymbolCountRule {
    fn name(&self) -> &'static str {
        "max_symbol_count_per_file"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        ctx.indexed_metrics
            .symbol_count
            .map(|value| metric("symbol_count", value, None))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.indexed_metrics.symbol_count else {
            return Ok(None);
        };
        Ok(threshold_violation(
            ctx,
            self.name(),
            actual,
            ctx.thresholds.max_symbol_count_per_file,
            "symbol count exceeds the allowed threshold",
            None,
        ))
    }
}

impl QualityRule for RefCountRule {
    fn name(&self) -> &'static str {
        "max_ref_count_per_file"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        ctx.indexed_metrics
            .ref_count
            .map(|value| metric("ref_count", value, None))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.indexed_metrics.ref_count else {
            return Ok(None);
        };
        Ok(threshold_violation(
            ctx,
            self.name(),
            actual,
            ctx.thresholds.max_ref_count_per_file,
            "reference count exceeds the allowed threshold",
            None,
        ))
    }
}

impl QualityRule for ModuleDepCountRule {
    fn name(&self) -> &'static str {
        "max_module_dep_count_per_file"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        ctx.indexed_metrics
            .module_dep_count
            .map(|value| metric("module_dep_count", value, None))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.indexed_metrics.module_dep_count else {
            return Ok(None);
        };
        Ok(threshold_violation(
            ctx,
            self.name(),
            actual,
            ctx.thresholds.max_module_dep_count_per_file,
            "module dependency count exceeds the allowed threshold",
            None,
        ))
    }
}

impl QualityRule for GraphEdgeOutCountRule {
    fn name(&self) -> &'static str {
        "max_graph_edge_out_count"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        ctx.indexed_metrics
            .graph_edge_out_count
            .map(|value| metric("graph_edge_out_count", value, None))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.indexed_metrics.graph_edge_out_count else {
            return Ok(None);
        };
        Ok(threshold_violation(
            ctx,
            self.name(),
            actual,
            ctx.thresholds.max_graph_edge_out_count,
            "graph outgoing edge count exceeds the allowed threshold",
            None,
        ))
    }
}
