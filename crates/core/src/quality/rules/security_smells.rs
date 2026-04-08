use anyhow::Result;

use super::{QualityRule, RuleContext, metric, signal_violation};
use crate::model::{FindingFamily, QualitySource, QualityViolationEntry};
use crate::quality::SecuritySmellMatch;

struct ShellExecRule;
struct PathTraversalRule;
struct RawSqlRule;
struct UnsafeDeserializeRule;

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![
        Box::new(ShellExecRule),
        Box::new(PathTraversalRule),
        Box::new(RawSqlRule),
        Box::new(UnsafeDeserializeRule),
    ]
}

impl QualityRule for ShellExecRule {
    fn name(&self) -> &'static str {
        "security_smell_shell_exec"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        metric_for_match(
            "security_smell_shell_exec_count",
            &ctx.facts.security_smells.shell_exec,
        )
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(build_smell_violation(
            ctx,
            self.name(),
            &ctx.facts.security_smells.shell_exec,
            "shell execution path detected; manual review required",
        ))
    }
}

impl QualityRule for PathTraversalRule {
    fn name(&self) -> &'static str {
        "security_smell_path_traversal"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        metric_for_match(
            "security_smell_path_traversal_count",
            &ctx.facts.security_smells.path_traversal,
        )
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(build_smell_violation(
            ctx,
            self.name(),
            &ctx.facts.security_smells.path_traversal,
            "file-path handling looks unguarded; manual review required",
        ))
    }
}

impl QualityRule for RawSqlRule {
    fn name(&self) -> &'static str {
        "security_smell_raw_sql"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        metric_for_match(
            "security_smell_raw_sql_count",
            &ctx.facts.security_smells.raw_sql,
        )
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(build_smell_violation(
            ctx,
            self.name(),
            &ctx.facts.security_smells.raw_sql,
            "raw SQL interpolation smell detected; manual review required",
        ))
    }
}

impl QualityRule for UnsafeDeserializeRule {
    fn name(&self) -> &'static str {
        "security_smell_unsafe_deserialize"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        metric_for_match(
            "security_smell_unsafe_deserialize_count",
            &ctx.facts.security_smells.unsafe_deserialize,
        )
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(build_smell_violation(
            ctx,
            self.name(),
            &ctx.facts.security_smells.unsafe_deserialize,
            "unsafe deserialization smell detected; manual review required",
        ))
    }
}

fn metric_for_match(
    metric_id: &str,
    facts: &SecuritySmellMatch,
) -> Option<crate::quality::QualityMetricEntry> {
    (facts.match_count > 0).then(|| metric(metric_id, facts.match_count, facts.location.clone()))
}

fn build_smell_violation(
    ctx: &RuleContext<'_>,
    rule_id: &str,
    facts: &SecuritySmellMatch,
    message: &str,
) -> Option<QualityViolationEntry> {
    (facts.match_count > 0).then(|| {
        signal_violation(
            ctx,
            rule_id,
            facts.match_count,
            0,
            message.to_string(),
            facts.location.clone(),
            Some(QualitySource::Heuristic),
            FindingFamily::SecuritySmells,
            facts.confidence,
            facts.noise_reason.clone(),
            vec!["review input trust boundary and sanitization path".to_string()],
        )
    })
}
