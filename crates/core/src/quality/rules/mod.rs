use anyhow::Result;

use super::metrics::FileKind;
use super::{
    EffectiveQualityPolicy, IndexedQualityMetrics, QualityCandidateFacts, QualityMetricEntry,
    QualityThresholds,
};
use crate::model::{
    QualityLocation, QualitySource, QualityViolationEntry, SuppressedQualityViolationEntry,
};

mod basic;
mod complexity;
mod duplication;
mod file_hotspots;
mod git_risk;
mod layering;
mod structural;
mod test_risk;

#[derive(Debug, Clone)]
pub(crate) struct RuleEvaluationResult {
    pub(crate) metrics: Vec<QualityMetricEntry>,
    pub(crate) violations: Vec<QualityViolationEntry>,
    pub(crate) suppressed_violations: Vec<SuppressedQualityViolationEntry>,
    pub(crate) had_rule_errors: bool,
    pub(crate) last_error_rule_id: Option<String>,
}

pub(super) struct RuleContext<'a> {
    pub(super) facts: &'a QualityCandidateFacts,
    pub(super) indexed_metrics: &'a IndexedQualityMetrics,
    pub(super) thresholds: &'a QualityThresholds,
    pub(super) effective_policy: &'a EffectiveQualityPolicy,
    pub(super) file_kind: FileKind,
}

pub(super) trait QualityRule {
    fn name(&self) -> &'static str;
    fn metric(&self, ctx: &RuleContext<'_>) -> Option<QualityMetricEntry>;
    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>>;
}

pub(crate) fn evaluate_rules(
    facts: &QualityCandidateFacts,
    indexed_metrics: &IndexedQualityMetrics,
    policy: &EffectiveQualityPolicy,
) -> RuleEvaluationResult {
    let ctx = RuleContext {
        facts,
        indexed_metrics,
        thresholds: &policy.thresholds,
        effective_policy: policy,
        file_kind: facts.file_kind,
    };
    let mut out = RuleEvaluationResult {
        metrics: Vec::new(),
        violations: Vec::new(),
        suppressed_violations: Vec::new(),
        had_rule_errors: false,
        last_error_rule_id: None,
    };

    for rule in default_rules() {
        if let Some(metric) = rule.metric(&ctx) {
            out.metrics.push(metric);
        }
        match rule.evaluate(&ctx) {
            Ok(Some(violation)) => push_violation(&ctx, &mut out, violation),
            Ok(None) => {}
            Err(_) => {
                out.had_rule_errors = true;
                out.last_error_rule_id = Some(rule.name().to_string());
            }
        }
    }

    out.metrics
        .sort_by(|left, right| left.metric_id.cmp(&right.metric_id));
    out.violations
        .sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    out.suppressed_violations
        .sort_by(|left, right| left.violation.rule_id.cmp(&right.violation.rule_id));
    out
}

pub(super) fn metric(
    metric_id: &str,
    metric_value: i64,
    location: Option<QualityLocation>,
) -> QualityMetricEntry {
    metric_with_details(metric_id, metric_value, location, None)
}

pub(super) fn metric_with_details(
    metric_id: &str,
    metric_value: i64,
    location: Option<QualityLocation>,
    source: Option<QualitySource>,
) -> QualityMetricEntry {
    QualityMetricEntry {
        metric_id: metric_id.to_string(),
        metric_value,
        location,
        source,
    }
}

pub(super) fn threshold_violation(
    ctx: &RuleContext<'_>,
    rule_id: &str,
    actual_value: i64,
    threshold_value: i64,
    message: &str,
    location: Option<QualityLocation>,
) -> Option<QualityViolationEntry> {
    threshold_violation_with_source(
        ctx,
        rule_id,
        actual_value,
        threshold_value,
        message,
        location,
        None,
    )
}

pub(super) fn threshold_violation_with_source(
    ctx: &RuleContext<'_>,
    rule_id: &str,
    actual_value: i64,
    threshold_value: i64,
    message: &str,
    location: Option<QualityLocation>,
    source: Option<QualitySource>,
) -> Option<QualityViolationEntry> {
    (actual_value > threshold_value).then(|| QualityViolationEntry {
        rule_id: rule_id.to_string(),
        actual_value,
        threshold_value,
        message: message.to_string(),
        severity: ctx.effective_policy.metadata_for_rule(rule_id).severity,
        category: ctx.effective_policy.metadata_for_rule(rule_id).category,
        location,
        source,
    })
}

pub(super) fn explicit_violation(
    ctx: &RuleContext<'_>,
    rule_id: &str,
    actual_value: i64,
    threshold_value: i64,
    message: String,
    location: Option<QualityLocation>,
    source: Option<QualitySource>,
) -> QualityViolationEntry {
    QualityViolationEntry {
        rule_id: rule_id.to_string(),
        actual_value,
        threshold_value,
        message,
        severity: ctx.effective_policy.metadata_for_rule(rule_id).severity,
        category: ctx.effective_policy.metadata_for_rule(rule_id).category,
        location,
        source,
    }
}

fn push_violation(
    ctx: &RuleContext<'_>,
    out: &mut RuleEvaluationResult,
    violation: QualityViolationEntry,
) {
    let suppressions = ctx
        .effective_policy
        .suppressions_for_rule(&violation.rule_id);
    if suppressions.is_empty() {
        out.violations.push(violation);
        return;
    }
    out.suppressed_violations
        .push(SuppressedQualityViolationEntry {
            violation,
            suppressions,
        });
}

fn default_rules() -> Vec<Box<dyn QualityRule>> {
    let mut rules = basic::rules();
    rules.extend(file_hotspots::rules());
    rules.extend(complexity::rules());
    rules.extend(duplication::rules());
    rules.extend(structural::rules());
    rules.extend(layering::rules());
    rules.extend(git_risk::rules());
    rules.extend(test_risk::rules());
    rules
}

pub(crate) use file_hotspots::analyze_hotspots;
