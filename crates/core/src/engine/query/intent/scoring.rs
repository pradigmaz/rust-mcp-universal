use crate::model::{ContextMode, SearchHit};

use super::SearchIntent;
use super::roles::{
    BackendLayer, FileDomain, FileRole, NormalizedText, count_matches, is_test_path,
};

impl SearchIntent {
    pub(in crate::engine::query) fn apply_to_hits(
        &self,
        hits: &mut [SearchHit],
        context_mode: Option<ContextMode>,
    ) {
        for hit in hits {
            hit.score = (hit.score
                + self.score_hit(&hit.path, &hit.preview, &hit.language, context_mode))
            .max(0.0);
        }
    }

    pub(in crate::engine::query) fn score_hit(
        &self,
        path: &str,
        preview: &str,
        language: &str,
        context_mode: Option<ContextMode>,
    ) -> f32 {
        if self.coverage_groups.is_empty() || matches!(context_mode, Some(ContextMode::Design)) {
            return 0.0;
        }

        let role = FileRole::from_path_language(path, language);
        let haystack = NormalizedText::new(path, preview);
        let coverage = count_matches(&haystack, &self.coverage_groups);
        let workload_matches = count_matches(&haystack, &self.workload_groups);
        let plain_test_intent = self.wants_tests
            && self.coverage_groups.len() == 1
            && !self.prefers_backend
            && !self.prefers_frontend
            && !self.prefers_database
            && !self.wants_architecture
            && !self.wants_entrypoints
            && !self.wants_auth_boundary
            && !self.wants_service_layer
            && !self.wants_api_surface
            && !self.wants_mod_runtime;
        let mut score = 0.0_f32;

        score += self.domain_score(role.domain);
        score += self.backend_layer_score(&role);
        score += self.frontend_role_score(&role);
        score += self.mod_runtime_score(&role);
        score += self.database_score(&role);
        score += self.support_artifact_penalty(&role);

        if self.wants_hook && role.hook_like {
            score += 0.024;
        }
        if self.wants_page && role.page_like {
            score += 0.024;
        }
        if self.wants_component && role.component_like {
            score += 0.020;
        }
        if self.wants_migration && role.migration_like {
            score += 0.018;
        }
        if role.schema_like && self.prefers_database {
            score += 0.012;
        }
        if self.wants_entrypoints
            && role.domain == FileDomain::Backend
            && role.backend_layer == BackendLayer::ApiSurface
        {
            score += 0.060;
        }
        if self.wants_entrypoints && role.mod_entrypoint_like {
            score += 0.070;
        }
        if self.wants_auth_boundary && role.domain == FileDomain::Backend {
            score += 0.020;
        }
        if self.wants_tests && is_test_path(path) {
            score += 0.034;
            if plain_test_intent {
                score += 0.090;
            }
            if self.wants_auth_boundary {
                score += 0.030;
            }
        } else if plain_test_intent {
            score -= 0.050;
        }

        if coverage > 1 {
            score += ((coverage - 1) as f32 * 0.030).min(0.090);
        }
        if coverage >= 3 {
            score += 0.020;
        }
        if coverage == self.coverage_groups.len() {
            score += match coverage {
                0 | 1 => 0.0,
                2 => 0.060,
                _ => 0.160,
            };
        }
        if workload_matches > 0 {
            score += (workload_matches as f32 * 0.018).min(0.072);
        }
        if workload_matches == self.workload_groups.len() && workload_matches >= 2 {
            score += 0.080;
        }

        score.clamp(-0.220, 0.400)
    }

    pub(super) fn domain_score(&self, domain: FileDomain) -> f32 {
        match self.explicit_domain {
            Some(FileDomain::Backend) => match domain {
                FileDomain::Backend => 0.060,
                FileDomain::Frontend => -0.060,
                FileDomain::Database => -0.070,
                _ => 0.0,
            },
            Some(FileDomain::Frontend) => match domain {
                FileDomain::Frontend => 0.070,
                FileDomain::Backend => -0.080,
                FileDomain::Database => -0.055,
                _ => 0.0,
            },
            Some(FileDomain::Database) => match domain {
                FileDomain::Database => 0.054,
                FileDomain::Backend | FileDomain::Frontend => -0.030,
                _ => 0.0,
            },
            _ => 0.0,
        }
    }

    pub(super) fn backend_layer_score(&self, role: &FileRole) -> f32 {
        let mut score = 0.0_f32;
        if self.prefers_backend {
            if role.domain == FileDomain::Backend {
                score += 0.020;
            } else if role.domain == FileDomain::Frontend
                && self.explicit_domain != Some(FileDomain::Frontend)
            {
                score -= 0.016;
            }
        }

        if self.prefers_backend_mixed_layers() {
            match role.backend_layer {
                BackendLayer::ServiceWork => score += 0.070,
                BackendLayer::ApiSurface => score += 0.040,
                BackendLayer::Other => {}
            }
        } else if self.prefers_backend_service_layer() {
            match role.backend_layer {
                BackendLayer::ServiceWork => score += 0.095,
                BackendLayer::ApiSurface => score -= 0.090,
                BackendLayer::Other => {}
            }
        } else if self.prefers_backend_api_surface() {
            match role.backend_layer {
                BackendLayer::ApiSurface => score += 0.085,
                BackendLayer::ServiceWork => score -= 0.028,
                BackendLayer::Other => {}
            }
        }

        score
    }

    pub(super) fn frontend_role_score(&self, role: &FileRole) -> f32 {
        let mut score = 0.0_f32;
        if self.prefers_frontend {
            if role.domain == FileDomain::Frontend {
                score += 0.024;
                if role.hook_like || role.page_like || role.component_like {
                    score += 0.032;
                }
            } else if role.domain == FileDomain::Backend {
                score -= 0.024;
                if role.backend_layer == BackendLayer::ApiSurface {
                    score -= 0.080;
                }
            }
        }
        score
    }

    pub(super) fn mod_runtime_score(&self, role: &FileRole) -> f32 {
        if !self.wants_mod_runtime {
            return 0.0;
        }

        let foundational =
            role.mod_entrypoint_like || role.module_like || role.mixin_like || role.config_like;
        let mut score = 0.0_f32;
        if role.mod_entrypoint_like {
            score += 0.085;
        }
        if role.module_like {
            score += 0.065;
        }
        if role.mixin_like {
            score += 0.070;
        }
        if role.config_like {
            score += 0.048;
        }
        if role.runtime_like {
            score += if foundational { 0.030 } else { 0.010 };
        }
        if self.wants_hook && role.mixin_like {
            score += 0.040;
        }
        if self.wants_hook && role.hook_like && !role.mixin_like {
            score += 0.012;
        }
        if foundational && self.coverage_groups.len() >= 3 {
            score += 0.018;
        }
        if self.token_count >= 6 && role.runtime_like && !foundational {
            score -= 0.016;
        }
        if self.wants_entrypoints && role.runtime_like && !foundational {
            score -= 0.010;
        }
        score
    }

    pub(super) fn database_score(&self, role: &FileRole) -> f32 {
        if self.prefers_database {
            if role.domain == FileDomain::Database {
                0.018
            } else {
                0.0
            }
        } else if self.code_first && role.domain == FileDomain::Database {
            if self.prefers_backend || self.prefers_frontend {
                -0.100
            } else {
                -0.040
            }
        } else {
            0.0
        }
    }

    pub(super) fn support_artifact_penalty(&self, role: &FileRole) -> f32 {
        if !self.code_first || !role.support_artifact_like {
            return 0.0;
        }
        let mut score = -0.100;
        if self.prefers_backend || self.prefers_frontend {
            score -= 0.040;
        }
        score
    }

    pub(super) fn prefers_backend_service_layer(&self) -> bool {
        self.prefers_backend && self.wants_service_layer && !self.wants_api_surface
    }

    pub(super) fn prefers_backend_mixed_layers(&self) -> bool {
        self.prefers_backend && self.wants_service_layer && self.wants_api_surface
    }

    pub(super) fn prefers_backend_api_surface(&self) -> bool {
        self.prefers_backend && self.wants_api_surface && !self.wants_service_layer
    }
}
