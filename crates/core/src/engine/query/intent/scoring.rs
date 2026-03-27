use super::SearchIntent;
use super::roles::{BackendLayer, FileDomain, FileRole};

impl SearchIntent {
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
