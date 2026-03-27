use super::SearchIntent;

impl SearchIntent {
    pub(crate) fn lexical_candidate_limit(&self, requested_limit: usize) -> usize {
        let requested_limit = requested_limit.max(1);
        if self.needs_expanded_candidate_pool() {
            return requested_limit.saturating_mul(6).clamp(60, 360);
        }
        if self.code_first && self.coverage_groups.len() >= 2 {
            return requested_limit.saturating_mul(4).clamp(40, 320);
        }
        requested_limit.saturating_mul(2).min(200)
    }

    pub(crate) fn pre_rerank_candidate_limit(
        &self,
        requested_limit: usize,
        semantic_enabled: bool,
    ) -> usize {
        let requested_limit = requested_limit.max(1);
        if semantic_enabled && self.needs_expanded_candidate_pool() {
            return requested_limit.saturating_mul(4).clamp(32, 280);
        }
        if semantic_enabled {
            return requested_limit.saturating_mul(3).min(240);
        }
        if self.code_first && self.coverage_groups.len() >= 2 {
            return requested_limit.saturating_mul(2).clamp(20, 80);
        }
        requested_limit
    }

    fn needs_expanded_candidate_pool(&self) -> bool {
        (self.wants_tests && self.wants_auth_boundary)
            || (self.wants_api_surface && self.wants_service_layer)
            || self.expects_mod_runtime_surface()
            || (self.prefers_backend && self.prefers_frontend && self.coverage_groups.len() >= 3)
    }
}
