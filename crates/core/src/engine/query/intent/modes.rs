use crate::model::{AgentIntentMode, ModeResolutionSource};

use super::aliases::{
    API_ALIASES, CONFIG_ALIASES, DOMAIN_ALIASES, ENDPOINT_ALIASES, ENTRYPOINT_ALIASES,
    MODULE_ALIASES, ROUTER_ALIASES, RULE_ALIASES, RUNTIME_ALIASES, SCHEMA_ALIASES, SERVICE_ALIASES,
    TEST_ALIASES,
};
use super::roles::FileDomain;
use super::{ResolvedAgentIntent, SearchIntent};

impl SearchIntent {
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
}
