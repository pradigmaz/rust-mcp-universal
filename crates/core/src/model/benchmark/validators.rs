use super::super::core::{PrivacyMode, SemanticFailMode};
use super::defaults;
use super::types::{
    QueryBenchmarkComparisonOptions, QueryBenchmarkGateThresholds, QueryBenchmarkOptions,
};

impl QueryBenchmarkOptions {
    pub const fn new(
        k: usize,
        limit: usize,
        semantic: bool,
        max_chars: usize,
        max_tokens: usize,
    ) -> Self {
        Self {
            k,
            limit,
            semantic,
            semantic_fail_mode: SemanticFailMode::FailOpen,
            privacy_mode: PrivacyMode::Off,
            max_chars,
            max_tokens,
            auto_index: defaults::query_benchmark_auto_index(),
        }
    }

    pub const fn with_auto_index(mut self, auto_index: bool) -> Self {
        self.auto_index = auto_index;
        self
    }

    pub const fn with_semantic_fail_mode(mut self, semantic_fail_mode: SemanticFailMode) -> Self {
        self.semantic_fail_mode = semantic_fail_mode;
        self
    }

    pub const fn with_privacy_mode(mut self, privacy_mode: PrivacyMode) -> Self {
        self.privacy_mode = privacy_mode;
        self
    }
}

impl QueryBenchmarkComparisonOptions {
    pub const fn new(baseline: QueryBenchmarkOptions, candidate: QueryBenchmarkOptions) -> Self {
        Self {
            baseline,
            candidate,
            runs: defaults::query_benchmark_runs(),
            gate_thresholds: QueryBenchmarkGateThresholds::new(),
            fail_fast: defaults::query_benchmark_fail_fast(),
        }
    }

    pub const fn with_runs(mut self, runs: usize) -> Self {
        self.runs = runs;
        self
    }

    pub const fn with_gate_thresholds(
        mut self,
        gate_thresholds: QueryBenchmarkGateThresholds,
    ) -> Self {
        self.gate_thresholds = gate_thresholds;
        self
    }

    pub const fn with_fail_fast(mut self, fail_fast: bool) -> Self {
        self.fail_fast = fail_fast;
        self
    }
}

impl Default for QueryBenchmarkGateThresholds {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryBenchmarkGateThresholds {
    pub const fn new() -> Self {
        Self {
            quality_max_drop_ratio: defaults::quality_max_drop_ratio(),
            latency_p50_max_increase_ratio: defaults::latency_p50_max_increase_ratio(),
            latency_p95_max_increase_ratio: defaults::latency_p95_max_increase_ratio(),
            latency_p95_max_increase_ms: defaults::latency_p95_max_increase_ms(),
            token_cost_max_increase_ratio: defaults::token_cost_max_increase_ratio(),
        }
    }
}
