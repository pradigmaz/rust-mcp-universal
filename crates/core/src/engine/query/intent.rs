use crate::model::{AgentIntentMode, ContextMode, ModeResolutionSource, SearchHit};

#[path = "intent/aliases.rs"]
mod aliases;
#[path = "intent/bootstrap.rs"]
mod bootstrap;
#[path = "intent/limits.rs"]
mod limits;
#[path = "intent/roles.rs"]
mod roles;
#[path = "intent/scoring.rs"]
mod scoring;

use aliases::{
    API_ALIASES, ARCHITECTURE_ALIASES, AUTH_ALIASES, BACKEND_ALIASES, BATCH_ALIASES,
    COMPONENT_ALIASES, CONFIG_ALIASES, DB_ALIASES, DEADLINE_ALIASES, DOMAIN_ALIASES,
    ENDPOINT_ALIASES, ENTRYPOINT_ALIASES, FRONTEND_ALIASES, GRADING_ALIASES, HOOK_ALIASES,
    JOURNAL_ALIASES, MIGRATION_ALIASES, MIXIN_ALIASES, MODULE_ALIASES, ORCHESTRATION_ALIASES,
    PAGE_ALIASES, ROUTER_ALIASES, RULE_ALIASES, RUNTIME_ALIASES, SCHEMA_ALIASES, SERVICE_ALIASES,
    SQL_ALIASES, TEST_ALIASES, VALIDATOR_ALIASES, VISIBILITY_ALIASES,
};
use roles::{
    BackendLayer, FileDomain, FileRole, NormalizedText, collect_groups, count_matches, is_test_path,
};

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
    wants_mod_runtime: bool,
    wants_migration: bool,
    wants_architecture: bool,
    wants_entrypoints: bool,
    wants_auth_boundary: bool,
    wants_tests: bool,
    code_first: bool,
    token_count: usize,
    coverage_groups: Vec<&'static [&'static str]>,
    workload_groups: Vec<&'static [&'static str]>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedAgentIntent {
    pub(crate) mode: AgentIntentMode,
    pub(crate) source: ModeResolutionSource,
}

impl SearchIntent {
    pub(super) fn from_query(query: &str) -> Self {
        let normalized = NormalizedText::new(query, "");
        let token_count = normalized.token_count();
        let explicit_backend = normalized.matches_any(BACKEND_ALIASES);
        let explicit_frontend = normalized.matches_any(FRONTEND_ALIASES);
        let explicit_database = normalized.matches_any(MIGRATION_ALIASES)
            || normalized.matches_any(SCHEMA_ALIASES)
            || normalized.matches_any(SQL_ALIASES)
            || normalized.matches_any(DB_ALIASES);
        let wants_architecture = normalized.matches_any(ARCHITECTURE_ALIASES);
        let wants_entrypoints = normalized.matches_any(ENTRYPOINT_ALIASES);
        let wants_auth_boundary = normalized.matches_any(AUTH_ALIASES);
        let wants_tests = normalized.matches_any(TEST_ALIASES);
        let wants_api_surface = normalized.matches_any(API_ALIASES)
            || normalized.matches_any(ENDPOINT_ALIASES)
            || normalized.matches_any(ROUTER_ALIASES);
        let wants_service = normalized.matches_any(SERVICE_ALIASES)
            || normalized.matches_any(DOMAIN_ALIASES)
            || normalized.matches_any(ORCHESTRATION_ALIASES)
            || normalized.matches_any(RULE_ALIASES);
        let wants_hook = normalized.matches_any(HOOK_ALIASES);
        let wants_page = normalized.matches_any(PAGE_ALIASES);
        let wants_component = normalized.matches_any(COMPONENT_ALIASES);
        let wants_mod_runtime = normalized.matches_any(MODULE_ALIASES)
            || normalized.matches_any(MIXIN_ALIASES)
            || normalized.matches_any(CONFIG_ALIASES)
            || normalized.matches_any(RUNTIME_ALIASES);
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
        let prefers_frontend = explicit_frontend
            || wants_page
            || wants_component
            || (wants_hook && !wants_mod_runtime);
        let prefers_database = explicit_database;
        let wants_service_layer =
            wants_service || wants_batch || wants_validator || wants_deadline || wants_visibility;
        let code_first = prefers_backend
            || prefers_frontend
            || prefers_database
            || wants_architecture
            || wants_entrypoints
            || wants_auth_boundary
            || wants_tests
            || wants_mod_runtime
            || has_workload;

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
            wants_mod_runtime,
            wants_migration,
            wants_architecture,
            wants_entrypoints,
            wants_auth_boundary,
            wants_tests,
            code_first,
            token_count,
            coverage_groups: collect_groups(
                &normalized,
                &[
                    BACKEND_ALIASES,
                    FRONTEND_ALIASES,
                    API_ALIASES,
                    ENDPOINT_ALIASES,
                    ROUTER_ALIASES,
                    SERVICE_ALIASES,
                    DOMAIN_ALIASES,
                    ORCHESTRATION_ALIASES,
                    RULE_ALIASES,
                    MODULE_ALIASES,
                    MIXIN_ALIASES,
                    CONFIG_ALIASES,
                    RUNTIME_ALIASES,
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

    pub(crate) fn from_agent_mode(mode: AgentIntentMode) -> Self {
        match mode {
            AgentIntentMode::EntrypointMap => Self {
                explicit_domain: Some(FileDomain::Backend),
                prefers_backend: true,
                prefers_frontend: false,
                prefers_database: false,
                wants_api_surface: true,
                wants_service_layer: true,
                wants_hook: false,
                wants_page: false,
                wants_component: false,
                wants_mod_runtime: false,
                wants_migration: false,
                wants_architecture: true,
                wants_entrypoints: true,
                wants_auth_boundary: false,
                wants_tests: false,
                code_first: true,
                token_count: 6,
                coverage_groups: vec![
                    ENTRYPOINT_ALIASES,
                    API_ALIASES,
                    ENDPOINT_ALIASES,
                    ROUTER_ALIASES,
                    SERVICE_ALIASES,
                ],
                workload_groups: Vec::new(),
            },
            AgentIntentMode::TestMap => Self {
                explicit_domain: None,
                prefers_backend: true,
                prefers_frontend: false,
                prefers_database: false,
                wants_api_surface: false,
                wants_service_layer: true,
                wants_hook: false,
                wants_page: false,
                wants_component: false,
                wants_mod_runtime: false,
                wants_migration: false,
                wants_architecture: false,
                wants_entrypoints: false,
                wants_auth_boundary: false,
                wants_tests: true,
                code_first: true,
                token_count: 6,
                coverage_groups: vec![TEST_ALIASES, ENTRYPOINT_ALIASES, SERVICE_ALIASES],
                workload_groups: Vec::new(),
            },
            AgentIntentMode::ReviewPrep => Self {
                explicit_domain: None,
                prefers_backend: true,
                prefers_frontend: true,
                prefers_database: true,
                wants_api_surface: true,
                wants_service_layer: true,
                wants_hook: false,
                wants_page: false,
                wants_component: false,
                wants_mod_runtime: true,
                wants_migration: true,
                wants_architecture: true,
                wants_entrypoints: true,
                wants_auth_boundary: true,
                wants_tests: true,
                code_first: true,
                token_count: 8,
                coverage_groups: vec![
                    ENTRYPOINT_ALIASES,
                    API_ALIASES,
                    SERVICE_ALIASES,
                    TEST_ALIASES,
                    SCHEMA_ALIASES,
                ],
                workload_groups: vec![RULE_ALIASES],
            },
            AgentIntentMode::ApiContractMap => Self {
                explicit_domain: Some(FileDomain::Backend),
                prefers_backend: true,
                prefers_frontend: false,
                prefers_database: false,
                wants_api_surface: true,
                wants_service_layer: true,
                wants_hook: false,
                wants_page: false,
                wants_component: false,
                wants_mod_runtime: false,
                wants_migration: false,
                wants_architecture: false,
                wants_entrypoints: true,
                wants_auth_boundary: false,
                wants_tests: false,
                code_first: true,
                token_count: 6,
                coverage_groups: vec![
                    API_ALIASES,
                    ENDPOINT_ALIASES,
                    ROUTER_ALIASES,
                    SERVICE_ALIASES,
                ],
                workload_groups: Vec::new(),
            },
            AgentIntentMode::RuntimeSurface => Self {
                explicit_domain: None,
                prefers_backend: true,
                prefers_frontend: true,
                prefers_database: false,
                wants_api_surface: false,
                wants_service_layer: false,
                wants_hook: true,
                wants_page: false,
                wants_component: false,
                wants_mod_runtime: true,
                wants_migration: false,
                wants_architecture: true,
                wants_entrypoints: true,
                wants_auth_boundary: false,
                wants_tests: false,
                code_first: true,
                token_count: 6,
                coverage_groups: vec![
                    MODULE_ALIASES,
                    CONFIG_ALIASES,
                    RUNTIME_ALIASES,
                    ENTRYPOINT_ALIASES,
                ],
                workload_groups: Vec::new(),
            },
            AgentIntentMode::RefactorSurface => Self {
                explicit_domain: None,
                prefers_backend: true,
                prefers_frontend: true,
                prefers_database: false,
                wants_api_surface: false,
                wants_service_layer: true,
                wants_hook: false,
                wants_page: false,
                wants_component: false,
                wants_mod_runtime: true,
                wants_migration: false,
                wants_architecture: true,
                wants_entrypoints: false,
                wants_auth_boundary: false,
                wants_tests: true,
                code_first: true,
                token_count: 7,
                coverage_groups: vec![
                    SERVICE_ALIASES,
                    DOMAIN_ALIASES,
                    MODULE_ALIASES,
                    TEST_ALIASES,
                ],
                workload_groups: vec![RULE_ALIASES],
            },
        }
    }

    pub(crate) fn resolve(
        query: &str,
        explicit_mode: Option<AgentIntentMode>,
    ) -> (Self, ResolvedAgentIntent) {
        if let Some(mode) = explicit_mode {
            return (
                Self::from_agent_mode(mode),
                ResolvedAgentIntent {
                    mode,
                    source: ModeResolutionSource::Explicit,
                },
            );
        }

        let inferred = Self::from_query(query);
        let mode = inferred
            .inferred_agent_mode()
            .unwrap_or(AgentIntentMode::EntrypointMap);
        let source = if inferred.inferred_agent_mode().is_some() {
            ModeResolutionSource::Inferred
        } else {
            ModeResolutionSource::Default
        };
        (inferred, ResolvedAgentIntent { mode, source })
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

    pub(crate) fn expects_test_surface(&self) -> bool {
        self.wants_tests
    }

    pub(crate) fn expects_service_surface(&self) -> bool {
        self.wants_service_layer
    }

    pub(crate) fn expects_mod_runtime_surface(&self) -> bool {
        self.wants_mod_runtime
    }

    fn inferred_agent_mode(&self) -> Option<AgentIntentMode> {
        if self.wants_tests {
            return Some(AgentIntentMode::TestMap);
        }
        if self.wants_mod_runtime {
            return Some(AgentIntentMode::RuntimeSurface);
        }
        if self.wants_api_surface {
            return Some(AgentIntentMode::ApiContractMap);
        }
        if self.wants_entrypoints {
            return Some(AgentIntentMode::EntrypointMap);
        }
        if self.wants_architecture && (self.prefers_backend || self.prefers_frontend) {
            return Some(AgentIntentMode::RefactorSurface);
        }
        if self.prefers_backend && self.prefers_frontend {
            return Some(AgentIntentMode::ReviewPrep);
        }
        None
    }
}
