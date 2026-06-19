use crate::model::{AgentIntentMode, ModeResolutionSource};

#[path = "intent/aliases.rs"]
mod aliases;
#[path = "intent/bootstrap.rs"]
mod bootstrap;
#[path = "intent/limits.rs"]
mod limits;
#[path = "intent/modes.rs"]
mod modes;
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
use roles::is_test_path;
use roles::{FileDomain, NormalizedText, collect_groups};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_agent_modes_map_to_expected_intent_contracts() {
        let entrypoint = SearchIntent::from_agent_mode(AgentIntentMode::EntrypointMap);
        assert!(entrypoint.prefers_backend);
        assert!(entrypoint.wants_api_surface);
        assert!(entrypoint.wants_service_layer);
        assert!(entrypoint.wants_entrypoints);
        assert!(!entrypoint.wants_tests);
        assert!(!entrypoint.wants_mod_runtime);

        let test_map = SearchIntent::from_agent_mode(AgentIntentMode::TestMap);
        assert!(test_map.wants_tests);
        assert!(test_map.prefers_backend);
        assert!(test_map.wants_service_layer);
        assert!(!test_map.wants_api_surface);
        assert!(!test_map.wants_mod_runtime);

        let review_prep = SearchIntent::from_agent_mode(AgentIntentMode::ReviewPrep);
        assert!(review_prep.prefers_backend);
        assert!(review_prep.prefers_frontend);
        assert!(review_prep.prefers_database);
        assert!(review_prep.wants_api_surface);
        assert!(review_prep.wants_service_layer);
        assert!(review_prep.wants_architecture);
        assert!(review_prep.wants_entrypoints);
        assert!(review_prep.wants_auth_boundary);
        assert!(review_prep.wants_tests);
        assert!(review_prep.wants_mod_runtime);
        assert!(review_prep.wants_migration);

        let api_contract = SearchIntent::from_agent_mode(AgentIntentMode::ApiContractMap);
        assert!(api_contract.prefers_backend);
        assert!(api_contract.wants_api_surface);
        assert!(api_contract.wants_service_layer);
        assert!(api_contract.wants_entrypoints);
        assert!(!api_contract.wants_tests);
        assert!(!api_contract.wants_mod_runtime);

        let runtime_surface = SearchIntent::from_agent_mode(AgentIntentMode::RuntimeSurface);
        assert!(runtime_surface.prefers_backend);
        assert!(runtime_surface.prefers_frontend);
        assert!(runtime_surface.wants_hook);
        assert!(runtime_surface.wants_mod_runtime);
        assert!(!runtime_surface.wants_tests);

        let refactor_surface = SearchIntent::from_agent_mode(AgentIntentMode::RefactorSurface);
        assert!(refactor_surface.prefers_backend);
        assert!(refactor_surface.prefers_frontend);
        assert!(refactor_surface.wants_service_layer);
        assert!(refactor_surface.wants_architecture);
        assert!(refactor_surface.wants_tests);
        assert!(refactor_surface.wants_mod_runtime);
        assert!(!refactor_surface.wants_api_surface);
    }

    #[test]
    fn resolve_prefers_explicit_mode_then_inferred_then_default() {
        let (_, explicit) = SearchIntent::resolve(
            "auth boundary tests nearby backend frontend",
            Some(AgentIntentMode::RefactorSurface),
        );
        assert_eq!(explicit.mode, AgentIntentMode::RefactorSurface);
        assert_eq!(explicit.source, ModeResolutionSource::Explicit);

        let (_, inferred) =
            SearchIntent::resolve("auth boundary tests nearby backend frontend", None);
        assert_eq!(inferred.mode, AgentIntentMode::TestMap);
        assert_eq!(inferred.source, ModeResolutionSource::Inferred);

        let (_, fallback) = SearchIntent::resolve("mystery", None);
        assert_eq!(fallback.mode, AgentIntentMode::EntrypointMap);
        assert_eq!(fallback.source, ModeResolutionSource::Default);
    }
}
