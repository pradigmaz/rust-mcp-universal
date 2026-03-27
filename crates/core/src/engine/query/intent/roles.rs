use std::collections::BTreeSet;
use std::path::Path;

use super::aliases::{
    ANALYSIS_ALIASES, API_ALIASES, BACKEND_ALIASES, BATCH_ALIASES, COMPONENT_ALIASES,
    CONFIG_ALIASES, DOMAIN_ALIASES, ENDPOINT_ALIASES, FRONTEND_ALIASES, HOOK_ALIASES,
    MIGRATION_ALIASES, MIXIN_ALIASES, MODULE_ALIASES, ORCHESTRATION_ALIASES, PAGE_ALIASES,
    ROUTER_ALIASES, RULE_ALIASES, RUNTIME_ALIASES, SCHEMA_ALIASES, SERVICE_ALIASES, SQL_ALIASES,
    VALIDATOR_ALIASES,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileDomain {
    Backend,
    Frontend,
    Database,
    Docs,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BackendLayer {
    ApiSurface,
    ServiceWork,
    Other,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct FileRole {
    pub(super) domain: FileDomain,
    pub(super) backend_layer: BackendLayer,
    pub(super) hook_like: bool,
    pub(super) page_like: bool,
    pub(super) component_like: bool,
    pub(super) mixin_like: bool,
    pub(super) config_like: bool,
    pub(super) module_like: bool,
    pub(super) runtime_like: bool,
    pub(super) mod_entrypoint_like: bool,
    pub(super) migration_like: bool,
    pub(super) schema_like: bool,
    pub(super) support_artifact_like: bool,
}

impl FileRole {
    pub(super) fn from_path_language(path: &str, language: &str) -> Self {
        let normalized_path = path.replace('\\', "/");
        let lower_path = normalized_path.to_ascii_lowercase();
        let path_view = NormalizedText::new(&normalized_path, "");
        let file_stem = Path::new(path)
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();

        let is_ui_language = matches!(language, "javascript" | "typescript" | "tsx" | "jsx");
        let is_backend_language = matches!(language, "python" | "rust" | "go" | "java");
        let is_mod_container = lower_path.starts_with("mods/")
            || lower_path.contains("/mods/")
            || lower_path.starts_with("plugins/")
            || lower_path.contains("/plugins/");
        let hook_like =
            path_view.matches_any(HOOK_ALIASES) || (is_ui_language && file_stem.starts_with("use"));
        let page_like = path_view.matches_any(PAGE_ALIASES) || file_stem == "page";
        let component_like = path_view.matches_any(COMPONENT_ALIASES);
        let mixin_like = path_view.matches_any(MIXIN_ALIASES)
            || lower_path.contains("/mixin/")
            || lower_path.contains("/mixins/")
            || file_stem.starts_with("mixin");
        let config_like = path_view.matches_any(CONFIG_ALIASES)
            || lower_path.contains("/config/")
            || lower_path.contains("/configs/")
            || file_stem.contains("config")
            || file_stem.contains("settings");
        let module_like = path_view.matches_any(MODULE_ALIASES)
            || lower_path.contains("/module/")
            || lower_path.contains("/modules/")
            || file_stem.starts_with("module");
        let runtime_like = path_view.matches_any(RUNTIME_ALIASES)
            || lower_path.contains("/runtime/")
            || lower_path.contains("/network/")
            || lower_path.contains("/render/")
            || lower_path.contains("/client/");
        let mod_entrypoint_like = is_mod_container
            && (file_stem.ends_with("mod")
                || file_stem.ends_with("client")
                || file_stem.ends_with("plugin")
                || file_stem.contains("initializer")
                || file_stem.contains("bootstrap"));
        let service_like = path_view.matches_any(SERVICE_ALIASES)
            || path_view.matches_any(DOMAIN_ALIASES)
            || path_view.matches_any(ORCHESTRATION_ALIASES)
            || path_view.matches_any(RULE_ALIASES)
            || lower_path.contains("/services/")
            || lower_path.contains("/domain/")
            || lower_path.contains("/orchestration/")
            || file_stem.ends_with("_service");
        let batch_like = path_view.matches_any(BATCH_ALIASES) || file_stem.ends_with("_batch");
        let validator_like =
            path_view.matches_any(VALIDATOR_ALIASES) || file_stem.contains("validator");
        let endpoint_like =
            path_view.matches_any(ENDPOINT_ALIASES) || lower_path.contains("/endpoints/");
        let router_like = path_view.matches_any(ROUTER_ALIASES)
            || file_stem == "router"
            || file_stem.ends_with("_router")
            || file_stem.ends_with("_routes");
        let api_like = path_view.matches_any(API_ALIASES);
        let migration_like = path_view.matches_any(MIGRATION_ALIASES);
        let schema_like = path_view.matches_any(SCHEMA_ALIASES)
            || path_view.matches_any(SQL_ALIASES)
            || lower_path.ends_with(".sql");

        let docs_like = lower_path.starts_with("docs/")
            || lower_path.contains("/docs/")
            || lower_path.starts_with(".codex-planning/")
            || lower_path.contains("/.codex-planning/")
            || lower_path.ends_with(".md")
            || lower_path.ends_with(".mdx")
            || lower_path.ends_with(".txt");
        let support_artifact_like = lower_path.starts_with(".ai/")
            || lower_path.contains("/.ai/")
            || lower_path.starts_with(".codex-planning/")
            || lower_path.contains("/.codex-planning/")
            || (has_support_extension(&lower_path)
                && (path_view.matches_any(ANALYSIS_ALIASES)
                    || lower_path.contains("/memory/")
                    || lower_path.contains("/context/")
                    || lower_path.contains("/artifact")
                    || lower_path.contains("/report")
                    || lower_path.contains("/temp/")));

        let frontend_like =
            path_view.matches_any(FRONTEND_ALIASES) || hook_like || page_like || component_like;
        let backend_like = path_view.matches_any(BACKEND_ALIASES)
            || service_like
            || batch_like
            || validator_like
            || ((endpoint_like || router_like || api_like) && is_backend_language);
        let database_like = migration_like
            || schema_like
            || path_view.matches_any(&["alembic", "prisma", "migrations", "versions"]);

        let domain = if database_like {
            FileDomain::Database
        } else if frontend_like && is_ui_language {
            FileDomain::Frontend
        } else if backend_like {
            FileDomain::Backend
        } else if docs_like {
            FileDomain::Docs
        } else if frontend_like {
            FileDomain::Frontend
        } else {
            FileDomain::Other
        };

        let backend_layer = if domain != FileDomain::Backend {
            BackendLayer::Other
        } else if service_like || batch_like || validator_like {
            BackendLayer::ServiceWork
        } else if endpoint_like || router_like || api_like {
            BackendLayer::ApiSurface
        } else {
            BackendLayer::Other
        };

        Self {
            domain,
            backend_layer,
            hook_like,
            page_like,
            component_like,
            mixin_like,
            config_like,
            module_like,
            runtime_like,
            mod_entrypoint_like,
            migration_like,
            schema_like,
            support_artifact_like,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct NormalizedText {
    raw: String,
    tokens: BTreeSet<String>,
}

impl NormalizedText {
    pub(super) fn new(primary: &str, secondary: &str) -> Self {
        let raw = format!(
            "{} {}",
            primary.replace('\\', "/").to_ascii_lowercase(),
            secondary.to_ascii_lowercase()
        );
        let tokens = raw
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .filter(|token| !token.is_empty())
            .map(str::to_string)
            .collect();
        Self { raw, tokens }
    }

    pub(super) fn matches_any(&self, aliases: &[&str]) -> bool {
        aliases.iter().any(|alias| {
            self.tokens.contains(*alias) || (alias.len() >= 4 && self.raw.contains(alias))
        })
    }

    pub(super) fn token_count(&self) -> usize {
        self.tokens.len()
    }
}

pub(super) fn collect_groups(
    normalized: &NormalizedText,
    groups: &[&'static [&'static str]],
) -> Vec<&'static [&'static str]> {
    groups
        .iter()
        .copied()
        .filter(|aliases| normalized.matches_any(aliases))
        .collect()
}

pub(super) fn count_matches(
    haystack: &NormalizedText,
    groups: &[&'static [&'static str]],
) -> usize {
    groups
        .iter()
        .filter(|aliases| haystack.matches_any(aliases))
        .count()
}

pub(super) fn is_test_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    normalized.starts_with("tests/")
        || normalized.contains("/tests/")
        || normalized.contains("/test/")
        || normalized.contains("_tests/")
        || normalized.ends_with(".test.ts")
        || normalized.ends_with(".test.tsx")
        || normalized.ends_with(".test.js")
        || normalized.ends_with(".test.jsx")
        || normalized.ends_with(".test.py")
        || normalized.ends_with(".test.rs")
        || normalized.ends_with("_test.rs")
        || normalized.ends_with("_tests.rs")
        || normalized.ends_with("_spec.ts")
        || normalized.ends_with("_spec.tsx")
}

fn has_support_extension(path: &str) -> bool {
    path.ends_with(".json")
        || path.ends_with(".toml")
        || path.ends_with(".yaml")
        || path.ends_with(".yml")
}
