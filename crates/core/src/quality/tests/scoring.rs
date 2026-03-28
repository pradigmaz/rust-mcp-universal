use crate::model::{QualityRiskScoreComponents, QualityRiskScoreWeights};
use crate::quality::scoring::compute_file_risk_score;

#[test]
fn scoring_skeleton_is_deterministic() {
    let breakdown = compute_file_risk_score(
        QualityRiskScoreComponents {
            violation_count: 2.0,
            severity: 3.0,
            fan_in: 4.0,
            fan_out: 5.0,
            size: 6.0,
            nesting: 7.0,
            function_length: 8.0,
            complexity: 9.0,
            duplication: 10.0,
        },
        QualityRiskScoreWeights::default(),
    );
    assert_eq!(breakdown.score, 57.75);
}
