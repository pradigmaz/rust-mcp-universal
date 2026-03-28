use crate::model::{
    QualityRiskScoreBreakdown, QualityRiskScoreComponents, QualityRiskScoreWeights,
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
        + components.duplication * weights.duplication;
    QualityRiskScoreBreakdown {
        score,
        components,
        weights,
    }
}

pub(crate) fn compute_hit_risk_score(hit: &RuleViolationFileHit) -> QualityRiskScoreBreakdown {
    compute_file_risk_score(
        QualityRiskScoreComponents {
            violation_count: hit.violations.len() as f64,
            severity: hit
                .violations
                .iter()
                .map(|violation| severity_weight(violation.severity))
                .sum(),
            fan_in: metric_value(hit, "fan_in_count"),
            fan_out: metric_value(hit, "fan_out_count"),
            size: hit.non_empty_lines.unwrap_or(0) as f64 / 100.0,
            nesting: metric_value(hit, "max_nesting_depth"),
            function_length: metric_value(hit, "max_function_lines") / 50.0,
            complexity: complexity_component(hit),
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
