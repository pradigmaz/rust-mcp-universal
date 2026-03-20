const ANN_CANDIDATE_LIMIT_MULTIPLIER: usize = 12;
const ANN_MIN_PROBE: usize = 64;
const ANN_MAX_PROBE: usize = 1_024;
const ANN_ACCEPT_FLOOR_MIN: usize = 6;
const ANN_ACCEPT_FLOOR_MAX: usize = 24;

#[cfg(test)]
pub(super) fn ann_probe_limit(candidate_limit: usize) -> usize {
    ann_probe_limit_with_factor(candidate_limit, 1.0)
}

pub(super) fn ann_probe_limit_with_factor(candidate_limit: usize, probe_factor: f32) -> usize {
    let factor = if probe_factor.is_finite() {
        probe_factor.clamp(0.5, 3.0)
    } else {
        1.0
    };
    let base = candidate_limit
        .saturating_mul(ANN_CANDIDATE_LIMIT_MULTIPLIER)
        .clamp(ANN_MIN_PROBE, ANN_MAX_PROBE);
    let scaled = (base as f32 * factor).round();
    let scaled = if scaled <= 0.0 {
        ANN_MIN_PROBE as f32
    } else {
        scaled
    };
    usize::try_from(scaled as u64)
        .unwrap_or(ANN_MAX_PROBE)
        .clamp(ANN_MIN_PROBE, ANN_MAX_PROBE)
}

pub(super) fn ann_accept_floor(candidate_limit: usize) -> usize {
    candidate_limit
        .max(1)
        .clamp(ANN_ACCEPT_FLOOR_MIN, ANN_ACCEPT_FLOOR_MAX)
}
