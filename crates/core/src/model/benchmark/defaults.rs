pub(super) fn qrel_relevance() -> f32 {
    1.0
}

pub(super) const fn query_benchmark_auto_index() -> bool {
    true
}

pub(super) const fn query_benchmark_runs() -> usize {
    5
}

pub(super) const fn query_benchmark_fail_fast() -> bool {
    true
}

pub(super) const fn quality_max_drop_ratio() -> f32 {
    0.02
}

pub(super) const fn latency_p50_max_increase_ratio() -> f32 {
    0.20
}

pub(super) const fn latency_p95_max_increase_ratio() -> f32 {
    0.12
}

pub(super) const fn latency_p95_max_increase_ms() -> f32 {
    30.0
}

pub(super) const fn token_cost_max_increase_ratio() -> f32 {
    0.05
}
