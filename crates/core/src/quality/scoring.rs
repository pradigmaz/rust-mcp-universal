use crate::model::{
    FindingFamily, QualityRiskScoreBreakdown, QualityRiskScoreComponents, QualityRiskScoreWeights,
    QualitySeverity, RuleViolationFileHit,
};

impl Default for QualityRiskScoreWeights {
    fn default() -> Self {
        Self {
            violation_count: 1.0,
            severity: 1.0,
            fan_in: 1.0,
            fan_out: 1.0,
            size: 1.0,
            nesting: 1.0,
            function_length: 1.0,
            complexity: 1.25,
            layering: 1.35,
            git_risk: 1.15,
            test_risk: 1.25,
            duplication: 1.15,
        }
    }
}

pub(crate) fn compute_file_risk_score(
    components: QualityRiskScoreComponents,
    weights: QualityRiskScoreWeights,
) -> QualityRiskScoreBreakdown {
    let score = components.violation_count * weights.violation_count
        + components.severity * weights.severity
        + components.fan_in * weights.fan_in
        + components.fan_out * weights.fan_out
        + components.size * weights.size
        + components.nesting * weights.nesting
        + components.function_length * weights.function_length
        + components.complexity * weights.complexity
        + components.layering * weights.layering
        + components.git_risk * weights.git_risk
        + components.test_risk * weights.test_risk
        + components.duplication * weights.duplication;
    QualityRiskScoreBreakdown {
        score,
        components,
        weights,
    }
}

pub(crate) fn compute_hit_risk_score(hit: &RuleViolationFileHit) -> QualityRiskScoreBreakdown {
    let scored_violations = hit
        .violations
        .iter()
        .filter(|violation| violation.finding_family != Some(FindingFamily::SecuritySmells))
        .collect::<Vec<_>>();
    compute_file_risk_score(
        QualityRiskScoreComponents {
            violation_count: scored_violations.len() as f64,
            severity: scored_violations
                .iter()
                .map(|violation| severity_weight(violation.severity))
                .sum(),
            fan_in: metric_value(hit, "fan_in_count"),
            fan_out: metric_value(hit, "fan_out_count"),
            size: hit.non_empty_lines.unwrap_or(0) as f64 / 100.0,
            nesting: metric_value(hit, "max_nesting_depth"),
            function_length: metric_value(hit, "max_function_lines") / 50.0,
            complexity: complexity_component(hit),
            layering: layering_component(hit),
            git_risk: git_risk_component(hit),
            test_risk: test_risk_component(hit),
            duplication: duplication_component(hit),
        },
        QualityRiskScoreWeights::default(),
    )
}

fn complexity_component(hit: &RuleViolationFileHit) -> f64 {
    let cyclomatic = metric_value(hit, "max_cyclomatic_complexity") / 6.0;
    let cognitive = metric_value(hit, "max_cognitive_complexity") / 10.0;
    cyclomatic.max(cognitive)
}

fn duplication_component(hit: &RuleViolationFileHit) -> f64 {
    let density = metric_value(hit, "duplicate_density_bps") / 1_000.0;
    let blocks = metric_value(hit, "duplicate_block_count") / 2.0;
    let peers = metric_value(hit, "duplicate_peer_count") / 2.0;
    density.max(blocks + peers)
}

fn layering_component(hit: &RuleViolationFileHit) -> f64 {
    metric_value(hit, "layering_forbidden_edge_count")
        .max(metric_value(hit, "layering_out_of_direction_edge_count"))
        .max(metric_value(hit, "layering_unmatched_edge_count"))
}

fn git_risk_component(hit: &RuleViolationFileHit) -> f64 {
    let recent_commits = metric_value(hit, "git_recent_commit_count") / 6.0;
    let recent_churn = metric_value(hit, "git_recent_churn_lines") / 200.0;
    let primary_owner_share = metric_value(hit, "git_primary_author_share_bps") / 5_000.0;
    let cochange_neighbors = metric_value(hit, "git_cochange_neighbor_count") / 5.0;
    recent_commits
        .max(recent_churn)
        .max(primary_owner_share)
        .max(cochange_neighbors)
}

fn test_risk_component(hit: &RuleViolationFileHit) -> f64 {
    let mut score = 0.0_f64;
    if hit
        .violations
        .iter()
        .any(|violation| violation.rule_id == "hotspot_without_test_evidence")
    {
        score = score.max(2.0);
    }
    if hit
        .violations
        .iter()
        .any(|violation| violation.rule_id == "public_surface_without_tests")
    {
        score = score.max(1.5);
    }
    if hit
        .violations
        .iter()
        .any(|violation| violation.rule_id == "integration_entry_without_tests")
    {
        score = score.max(1.0);
    }
    score
}

fn metric_value(hit: &RuleViolationFileHit, metric_id: &str) -> f64 {
    hit.metrics
        .iter()
        .find(|metric| metric.metric_id == metric_id)
        .map(|metric| metric.metric_value.max(0) as f64)
        .unwrap_or(0.0)
}

fn severity_weight(severity: QualitySeverity) -> f64 {
    match severity {
        QualitySeverity::Low => 1.0,
        QualitySeverity::Medium => 2.0,
        QualitySeverity::High => 4.0,
        QualitySeverity::Critical => 8.0,
    }
}
