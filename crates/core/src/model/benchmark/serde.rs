use super::defaults;

pub(super) fn default_qrel_relevance() -> f32 {
    defaults::qrel_relevance()
}

pub(super) fn default_query_benchmark_auto_index() -> bool {
    defaults::query_benchmark_auto_index()
}

pub(super) fn default_query_benchmark_runs() -> usize {
    defaults::query_benchmark_runs()
}

pub(super) fn default_query_benchmark_fail_fast() -> bool {
    defaults::query_benchmark_fail_fast()
}

pub(super) fn default_quality_max_drop_ratio() -> f32 {
    defaults::quality_max_drop_ratio()
}

pub(super) fn default_latency_p50_max_increase_ratio() -> f32 {
    defaults::latency_p50_max_increase_ratio()
}

pub(super) fn default_latency_p95_max_increase_ratio() -> f32 {
    defaults::latency_p95_max_increase_ratio()
}

pub(super) fn default_latency_p95_max_increase_ms() -> f32 {
    defaults::latency_p95_max_increase_ms()
}

pub(super) fn default_token_cost_max_increase_ratio() -> f32 {
    defaults::token_cost_max_increase_ratio()
}
