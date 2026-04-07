use anyhow::Result;

use super::{QualityRule, RuleContext, explicit_violation, metric_with_details};
use crate::model::{QualitySource, QualityViolationEntry};

struct RecentCommitCountRule;
struct RecentAuthorCountRule;
struct RecentChurnRule;
struct OwnershipConcentrationRule;
struct ChangeCouplingRule;

pub(super) fn rules() -> Vec<Box<dyn QualityRule>> {
    vec![
        Box::new(RecentCommitCountRule),
        Box::new(RecentAuthorCountRule),
        Box::new(RecentChurnRule),
        Box::new(OwnershipConcentrationRule),
        Box::new(ChangeCouplingRule),
    ]
}

impl QualityRule for RecentCommitCountRule {
    fn name(&self) -> &'static str {
        "high_git_churn"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "git_recent_commit_count",
            ctx.facts.git_risk.recent_commit_count,
            None,
            Some(QualitySource::Git),
        ))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok((ctx.facts.git_risk.recent_commit_count
            > ctx.effective_policy.git_risk.max_recent_commits_per_file)
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    ctx.facts.git_risk.recent_commit_count,
                    ctx.effective_policy.git_risk.max_recent_commits_per_file,
                    "recent commit count exceeds git-risk threshold".to_string(),
                    None,
                    Some(QualitySource::Git),
                )
            }))
    }
}

impl QualityRule for RecentAuthorCountRule {
    fn name(&self) -> &'static str {
        "high_git_churn"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "git_recent_author_count",
            ctx.facts.git_risk.recent_author_count,
            None,
            Some(QualitySource::Git),
        ))
    }

    fn evaluate(&self, _ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok(None)
    }
}

impl QualityRule for RecentChurnRule {
    fn name(&self) -> &'static str {
        "high_git_churn"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "git_recent_churn_lines",
            ctx.facts.git_risk.recent_churn_lines,
            None,
            Some(QualitySource::Git),
        ))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok((ctx.facts.git_risk.recent_churn_lines
            > ctx
                .effective_policy
                .git_risk
                .max_recent_churn_lines_per_file)
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    ctx.facts.git_risk.recent_churn_lines,
                    ctx.effective_policy
                        .git_risk
                        .max_recent_churn_lines_per_file,
                    "recent git churn exceeds risk threshold".to_string(),
                    None,
                    Some(QualitySource::Git),
                )
            }))
    }
}

impl QualityRule for OwnershipConcentrationRule {
    fn name(&self) -> &'static str {
        "ownership_concentration"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "git_primary_author_share_bps",
            ctx.facts.git_risk.primary_author_share_bps,
            None,
            Some(QualitySource::Git),
        ))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok((ctx.facts.git_risk.primary_author_share_bps
            > ctx.effective_policy.git_risk.max_primary_author_share_bps)
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    ctx.facts.git_risk.primary_author_share_bps,
                    ctx.effective_policy.git_risk.max_primary_author_share_bps,
                    "recent ownership concentration exceeds threshold".to_string(),
                    None,
                    Some(QualitySource::Git),
                )
            }))
    }
}

impl QualityRule for ChangeCouplingRule {
    fn name(&self) -> &'static str {
        "high_change_coupling"
    }

    fn metric(&self, ctx: &RuleContext<'_>) -> Option<crate::quality::QualityMetricEntry> {
        Some(metric_with_details(
            "git_cochange_neighbor_count",
            ctx.facts.git_risk.cochange_neighbor_count,
            None,
            Some(QualitySource::Git),
        ))
    }

    fn evaluate(&self, ctx: &RuleContext<'_>) -> Result<Option<QualityViolationEntry>> {
        Ok((ctx.facts.git_risk.cochange_neighbor_count
            > ctx
                .effective_policy
                .git_risk
                .max_cochange_neighbors_per_file)
            .then(|| {
                explicit_violation(
                    ctx,
                    self.name(),
                    ctx.facts.git_risk.cochange_neighbor_count,
                    ctx.effective_policy
                        .git_risk
                        .max_cochange_neighbors_per_file,
                    "recent co-change coupling exceeds threshold".to_string(),
                    None,
                    Some(QualitySource::Git),
                )
            }))
    }
}
