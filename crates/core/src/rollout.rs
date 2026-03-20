use serde::{Deserialize, Serialize};

use crate::model::RolloutPhase;
use crate::utils::hash_bytes;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutDecision {
    pub phase: String,
    pub sample_percent: u8,
    pub bucket: u8,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollbackLevel {
    None,
    Fast,
    Full,
}

impl RollbackLevel {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Fast => "fast",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RollbackSignals {
    pub quality_regression: bool,
    pub latency_regression: bool,
    pub token_cost_regression: bool,
    pub privacy_violation: bool,
    pub error_spike: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackRecommendation {
    pub level: RollbackLevel,
    pub reasons: Vec<String>,
    pub fast_actions: Vec<String>,
    pub full_actions: Vec<String>,
}

pub fn rollout_bucket_percent(key: &str) -> u8 {
    let digest = hash_bytes(key.as_bytes());
    let head = digest.get(..8).unwrap_or(digest.as_str());
    let value = u32::from_str_radix(head, 16).unwrap_or(0);
    (value % 100) as u8
}

pub fn decide_semantic_rollout(
    requested_semantic: bool,
    vector_layer_enabled: bool,
    rollout_phase: RolloutPhase,
    key: &str,
) -> RolloutDecision {
    let bucket = rollout_bucket_percent(key);
    let sample_percent = rollout_phase.sample_percent();
    let enabled = requested_semantic && vector_layer_enabled && bucket < sample_percent;
    RolloutDecision {
        phase: rollout_phase.as_str().to_string(),
        sample_percent,
        bucket,
        enabled,
    }
}

pub fn stable_cycles_observed(run_passes: &[bool]) -> usize {
    run_passes.iter().take_while(|pass| **pass).count()
}

pub fn recommend_rollback(signals: &RollbackSignals) -> RollbackRecommendation {
    let mut reasons = Vec::new();
    if signals.quality_regression {
        reasons.push("quality regression beyond gate".to_string());
    }
    if signals.latency_regression {
        reasons.push("latency regression beyond gate".to_string());
    }
    if signals.token_cost_regression {
        reasons.push("token-cost regression beyond gate".to_string());
    }
    if signals.privacy_violation {
        reasons.push("privacy violation detected".to_string());
    }
    if signals.error_spike {
        reasons.push("error spike detected".to_string());
    }

    let level = if signals.privacy_violation || signals.error_spike {
        RollbackLevel::Full
    } else if signals.quality_regression
        || signals.latency_regression
        || signals.token_cost_regression
    {
        RollbackLevel::Fast
    } else {
        RollbackLevel::None
    };

    RollbackRecommendation {
        level,
        reasons,
        fast_actions: vec![
            "set vector_layer_enabled=false".to_string(),
            "set rollout_phase=shadow".to_string(),
            "set semantic_fail_mode=fail_open".to_string(),
        ],
        full_actions: vec![
            "restore latest migration backup (index.db/-wal/-shm)".to_string(),
            "roll back to previous binary build".to_string(),
            "rebuild index from scratch after rollback".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RollbackLevel, RollbackSignals, decide_semantic_rollout, recommend_rollback,
        stable_cycles_observed,
    };
    use crate::model::RolloutPhase;

    #[test]
    fn semantic_rollout_respects_phase_sampling() {
        let shadow = decide_semantic_rollout(true, true, RolloutPhase::Shadow, "abc");
        assert!(!shadow.enabled);
        assert_eq!(shadow.sample_percent, 0);

        let full = decide_semantic_rollout(true, true, RolloutPhase::Full100, "abc");
        assert!(full.enabled);
        assert_eq!(full.sample_percent, 100);
    }

    #[test]
    fn stable_cycle_counter_stops_on_first_failure() {
        assert_eq!(stable_cycles_observed(&[true, true, false, true]), 2);
        assert_eq!(stable_cycles_observed(&[false, true]), 0);
        assert_eq!(stable_cycles_observed(&[]), 0);
    }

    #[test]
    fn rollback_recommendation_prefers_full_for_privacy_or_error_spike() {
        let full = recommend_rollback(&RollbackSignals {
            quality_regression: true,
            latency_regression: false,
            token_cost_regression: false,
            privacy_violation: true,
            error_spike: false,
        });
        assert_eq!(full.level, RollbackLevel::Full);

        let fast = recommend_rollback(&RollbackSignals {
            quality_regression: true,
            latency_regression: false,
            token_cost_regression: false,
            privacy_violation: false,
            error_spike: false,
        });
        assert_eq!(fast.level, RollbackLevel::Fast);

        let none = recommend_rollback(&RollbackSignals::default());
        assert_eq!(none.level, RollbackLevel::None);
    }
}
