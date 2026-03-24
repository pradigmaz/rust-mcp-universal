use crate::model::{ContextMode, SearchHit};

#[path = "intent/aliases.rs"]
mod aliases;
#[path = "intent/roles.rs"]
mod roles;

use aliases::{
    API_ALIASES, BACKEND_ALIASES, BATCH_ALIASES, COMPONENT_ALIASES, DB_ALIASES, DEADLINE_ALIASES,
    ENDPOINT_ALIASES, FRONTEND_ALIASES, GRADING_ALIASES, HOOK_ALIASES, JOURNAL_ALIASES,
    MIGRATION_ALIASES, PAGE_ALIASES, ROUTER_ALIASES, SCHEMA_ALIASES, SERVICE_ALIASES, SQL_ALIASES,
    VALIDATOR_ALIASES, VISIBILITY_ALIASES,
};
use roles::{BackendLayer, FileDomain, FileRole, NormalizedText, collect_groups, count_matches};

#[derive(Debug, Clone)]
pub(super) struct SearchIntent {
    explicit_domain: Option<FileDomain>,
    prefers_backend: bool,
    prefers_frontend: bool,
    prefers_database: bool,
    wants_api_surface: bool,
    wants_service_layer: bool,
    wants_hook: bool,
    wants_page: bool,
    wants_component: bool,
    wants_migration: bool,
    code_first: bool,
    coverage_groups: Vec<&'static [&'static str]>,
    workload_groups: Vec<&'static [&'static str]>,
}

impl SearchIntent {
    pub(super) fn from_query(query: &str) -> Self {
        let normalized = NormalizedText::new(query, "");
        let explicit_backend = normalized.matches_any(BACKEND_ALIASES);
        let explicit_frontend = normalized.matches_any(FRONTEND_ALIASES);
        let explicit_database = normalized.matches_any(MIGRATION_ALIASES)
            || normalized.matches_any(SCHEMA_ALIASES)
            || normalized.matches_any(SQL_ALIASES)
            || normalized.matches_any(DB_ALIASES);
        let wants_api_surface = normalized.matches_any(API_ALIASES)
            || normalized.matches_any(ENDPOINT_ALIASES)
            || normalized.matches_any(ROUTER_ALIASES);
        let wants_service = normalized.matches_any(SERVICE_ALIASES);
        let wants_hook = normalized.matches_any(HOOK_ALIASES);
        let wants_page = normalized.matches_any(PAGE_ALIASES);
        let wants_component = normalized.matches_any(COMPONENT_ALIASES);
        let wants_batch = normalized.matches_any(BATCH_ALIASES);
        let wants_validator = normalized.matches_any(VALIDATOR_ALIASES);
        let wants_deadline = normalized.matches_any(DEADLINE_ALIASES);
        let wants_visibility = normalized.matches_any(VISIBILITY_ALIASES);
        let wants_migration = normalized.matches_any(MIGRATION_ALIASES);
        let wants_grading = normalized.matches_any(GRADING_ALIASES);
        let wants_journal = normalized.matches_any(JOURNAL_ALIASES);
        let has_workload =
            wants_batch || wants_deadline || wants_visibility || wants_grading || wants_journal;

        let explicit_domain = if explicit_database {
            Some(FileDomain::Database)
        } else if explicit_backend {
            Some(FileDomain::Backend)
        } else if explicit_frontend {
            Some(FileDomain::Frontend)
        } else {
            None
        };

        let prefers_backend = explicit_backend || wants_api_surface || wants_service || wants_batch;
        let prefers_frontend = explicit_frontend || wants_hook || wants_page || wants_component;
        let prefers_database = explicit_database;
        let wants_service_layer =
            wants_service || wants_batch || wants_validator || wants_deadline || wants_visibility;
        let code_first = prefers_backend || prefers_frontend || prefers_database || has_workload;

        Self {
            explicit_domain,
            prefers_backend,
            prefers_frontend,
            prefers_database,
            wants_api_surface,
            wants_service_layer,
            wants_hook,
            wants_page,
            wants_component,
            wants_migration,
            code_first,
            coverage_groups: collect_groups(
                &normalized,
                &[
                    BACKEND_ALIASES,
                    FRONTEND_ALIASES,
                    API_ALIASES,
                    ENDPOINT_ALIASES,
                    ROUTER_ALIASES,
                    SERVICE_ALIASES,
                    HOOK_ALIASES,
                    PAGE_ALIASES,
                    COMPONENT_ALIASES,
                    BATCH_ALIASES,
                    VALIDATOR_ALIASES,
                    MIGRATION_ALIASES,
                    SCHEMA_ALIASES,
                    SQL_ALIASES,
                    DB_ALIASES,
                    DEADLINE_ALIASES,
                    GRADING_ALIASES,
                    VISIBILITY_ALIASES,
                    JOURNAL_ALIASES,
                ],
            ),
            workload_groups: collect_groups(
                &normalized,
                &[
                    BATCH_ALIASES,
                    DEADLINE_ALIASES,
                    GRADING_ALIASES,
                    VISIBILITY_ALIASES,
                    JOURNAL_ALIASES,
                ],
            ),
        }
    }

    pub(super) fn apply_to_hits(&self, hits: &mut [SearchHit], context_mode: Option<ContextMode>) {
        for hit in hits {
            hit.score = (hit.score
                + self.score_hit(&hit.path, &hit.preview, &hit.language, context_mode))
            .max(0.0);
        }
    }

    pub(super) fn score_hit(
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
        let mut score = 0.0_f32;

        score += self.domain_score(role.domain);
        score += self.backend_layer_score(&role);
        score += self.frontend_role_score(&role);
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

    pub(super) fn lexical_candidate_limit(&self, requested_limit: usize) -> usize {
        let requested_limit = requested_limit.max(1);
        if self.code_first && self.coverage_groups.len() >= 2 {
            return requested_limit.saturating_mul(4).clamp(40, 320);
        }
        requested_limit.saturating_mul(2).min(200)
    }

    pub(super) fn pre_rerank_candidate_limit(
        &self,
        requested_limit: usize,
        semantic_enabled: bool,
    ) -> usize {
        let requested_limit = requested_limit.max(1);
        if semantic_enabled {
            return requested_limit.saturating_mul(3).min(240);
        }
        if self.code_first && self.coverage_groups.len() >= 2 {
            return requested_limit.saturating_mul(2).clamp(20, 80);
        }
        requested_limit
    }

    fn domain_score(&self, domain: FileDomain) -> f32 {
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

    fn backend_layer_score(&self, role: &FileRole) -> f32 {
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

        if self.prefers_backend_service_layer() {
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

    fn frontend_role_score(&self, role: &FileRole) -> f32 {
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

    fn database_score(&self, role: &FileRole) -> f32 {
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

    fn support_artifact_penalty(&self, role: &FileRole) -> f32 {
        if !self.code_first || !role.support_artifact_like {
            return 0.0;
        }
        let mut score = -0.100;
        if self.prefers_backend || self.prefers_frontend {
            score -= 0.040;
        }
        score
    }

    fn prefers_backend_service_layer(&self) -> bool {
        self.prefers_backend && self.wants_service_layer && !self.wants_api_surface
    }

    fn prefers_backend_api_surface(&self) -> bool {
        self.prefers_backend && self.wants_api_surface
    }
}
