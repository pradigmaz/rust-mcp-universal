use anyhow::Result;

use super::metrics::{
    FileKind, MAX_GRAPH_EDGE_OUT_COUNT, MAX_IMPORT_COUNT, MAX_LINE_LENGTH,
    MAX_MODULE_DEP_COUNT_PER_FILE, MAX_NON_EMPTY_LINES_CONFIG, MAX_NON_EMPTY_LINES_DEFAULT,
    MAX_NON_EMPTY_LINES_TEST, MAX_REF_COUNT_PER_FILE, MAX_SIZE_BYTES, MAX_SYMBOL_COUNT_PER_FILE,
};
use super::{IndexedQualityMetrics, QualityCandidateFacts, QualityMetricEntry};
use crate::model::QualityViolationEntry;

#[derive(Debug, Clone)]
pub(crate) struct RuleEvaluationResult {
    pub(crate) metrics: Vec<QualityMetricEntry>,
    pub(crate) violations: Vec<QualityViolationEntry>,
    pub(crate) had_rule_errors: bool,
    pub(crate) last_error_rule_id: Option<String>,
}

pub(crate) fn evaluate_rules(
    facts: &QualityCandidateFacts,
    indexed_metrics: &IndexedQualityMetrics,
) -> RuleEvaluationResult {
    let ctx = RuleContext {
        facts,
        indexed_metrics,
    };
    let mut out = RuleEvaluationResult {
        metrics: Vec::new(),
        violations: Vec::new(),
        had_rule_errors: false,
        last_error_rule_id: None,
    };

    for rule in default_rules() {
        if let Some(metric) = rule.metric(&ctx) {
            out.metrics.push(metric);
        }
        match rule.evaluate(&ctx) {
            Ok(Some(violation)) => out.violations.push(violation),
            Ok(None) => {}
            Err(_) => {
                out.had_rule_errors = true;
                out.last_error_rule_id = Some(rule.name().to_string());
            }
        }
    }

    out.metrics.sort_by(|left, right| left.metric_id.cmp(&right.metric_id));
    out.violations
        .sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    out
}

struct RuleContext<'a> {
    facts: &'a QualityCandidateFacts,
    indexed_metrics: &'a IndexedQualityMetrics,
}

trait QualityRule {
    fn name(&self) -> &'static str;
    fn metric(&self, ctx: &RuleContext<'_>) -> Option<QualityMetricEntry>;
    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>>;
}

struct SizeBytesRule;
struct NonEmptyLinesRule;
struct ImportCountRule;
struct MaxLineLengthRule;
struct SymbolCountRule;
struct RefCountRule;
struct ModuleDepCountRule;
struct GraphEdgeOutCountRule;

fn default_rules() -> Vec<Box<dyn QualityRule>> {
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

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<QualityMetricEntry> {
        Some(metric("size_bytes", ctx.facts.size_bytes))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(threshold_violation(
            self.name(),
            ctx.facts.size_bytes,
            MAX_SIZE_BYTES,
            "file size exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for NonEmptyLinesRule {
    fn name(&self) -> &'static str {
        "max_non_empty_lines"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<QualityMetricEntry> {
        ctx.facts
            .non_empty_lines
            .map(|value| metric("non_empty_lines", value))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.facts.non_empty_lines else {
            return Ok(None);
        };
        let (rule_id, threshold) = match ctx.facts.file_kind {
            FileKind::Default => ("max_non_empty_lines_default", MAX_NON_EMPTY_LINES_DEFAULT),
            FileKind::Test => ("max_non_empty_lines_test", MAX_NON_EMPTY_LINES_TEST),
            FileKind::Config => ("max_non_empty_lines_config", MAX_NON_EMPTY_LINES_CONFIG),
        };
        Ok(threshold_violation(
            rule_id,
            actual,
            threshold,
            "non-empty line count exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for ImportCountRule {
    fn name(&self) -> &'static str {
        "max_import_count"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<QualityMetricEntry> {
        ctx.facts.import_count.map(|value| metric("import_count", value))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.facts.import_count else {
            return Ok(None);
        };
        Ok(threshold_violation(
            self.name(),
            actual,
            MAX_IMPORT_COUNT,
            "import count exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for MaxLineLengthRule {
    fn name(&self) -> &'static str {
        "max_line_length"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<QualityMetricEntry> {
        ctx.facts
            .max_line_length
            .map(|value| metric("max_line_length", value))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.facts.max_line_length else {
            return Ok(None);
        };
        Ok(threshold_violation(
            self.name(),
            actual,
            MAX_LINE_LENGTH,
            "maximum line length exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for SymbolCountRule {
    fn name(&self) -> &'static str {
        "max_symbol_count_per_file"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<QualityMetricEntry> {
        ctx.indexed_metrics
            .symbol_count
            .map(|value| metric("symbol_count", value))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.indexed_metrics.symbol_count else {
            return Ok(None);
        };
        Ok(threshold_violation(
            self.name(),
            actual,
            MAX_SYMBOL_COUNT_PER_FILE,
            "symbol count exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for RefCountRule {
    fn name(&self) -> &'static str {
        "max_ref_count_per_file"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<QualityMetricEntry> {
        ctx.indexed_metrics
            .ref_count
            .map(|value| metric("ref_count", value))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.indexed_metrics.ref_count else {
            return Ok(None);
        };
        Ok(threshold_violation(
            self.name(),
            actual,
            MAX_REF_COUNT_PER_FILE,
            "reference count exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for ModuleDepCountRule {
    fn name(&self) -> &'static str {
        "max_module_dep_count_per_file"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<QualityMetricEntry> {
        ctx.indexed_metrics
            .module_dep_count
            .map(|value| metric("module_dep_count", value))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.indexed_metrics.module_dep_count else {
            return Ok(None);
        };
        Ok(threshold_violation(
            self.name(),
            actual,
            MAX_MODULE_DEP_COUNT_PER_FILE,
            "module dependency count exceeds the allowed threshold",
        ))
    }
}

impl QualityRule for GraphEdgeOutCountRule {
    fn name(&self) -> &'static str {
        "max_graph_edge_out_count"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<QualityMetricEntry> {
        ctx.indexed_metrics
            .graph_edge_out_count
            .map(|value| metric("graph_edge_out_count", value))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        let Some(actual) = ctx.indexed_metrics.graph_edge_out_count else {
            return Ok(None);
        };
        Ok(threshold_violation(
            self.name(),
            actual,
            MAX_GRAPH_EDGE_OUT_COUNT,
            "graph outgoing edge count exceeds the allowed threshold",
        ))
    }
}

fn metric(metric_id: &str, metric_value: i64) -> QualityMetricEntry {
    QualityMetricEntry {
        metric_id: metric_id.to_string(),
        metric_value,
    }
}

fn threshold_violation(
    rule_id: &str,
    actual_value: i64,
    threshold_value: i64,
    message: &str,
) -> Option<QualityViolationEntry> {
    (actual_value > threshold_value).then(|| QualityViolationEntry {
        rule_id: rule_id.to_string(),
        actual_value,
        threshold_value,
        message: message.to_string(),
    })
}
