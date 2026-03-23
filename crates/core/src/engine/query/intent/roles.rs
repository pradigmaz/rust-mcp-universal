use std::collections::BTreeSet;
use std::path::Path;

use super::aliases::{
    ANALYSIS_ALIASES, API_ALIASES, BACKEND_ALIASES, BATCH_ALIASES, COMPONENT_ALIASES,
    ENDPOINT_ALIASES, FRONTEND_ALIASES, HOOK_ALIASES, MIGRATION_ALIASES, PAGE_ALIASES,
    ROUTER_ALIASES, SCHEMA_ALIASES, SERVICE_ALIASES, SQL_ALIASES, VALIDATOR_ALIASES,
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
        let hook_like =
            path_view.matches_any(HOOK_ALIASES) || (is_ui_language && file_stem.starts_with("use"));
        let page_like = path_view.matches_any(PAGE_ALIASES) || file_stem == "page";
        let component_like = path_view.matches_any(COMPONENT_ALIASES);
        let service_like =
            path_view.matches_any(SERVICE_ALIASES) || file_stem.ends_with("_service");
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

fn has_support_extension(path: &str) -> bool {
    path.ends_with(".json")
        || path.ends_with(".toml")
        || path.ends_with(".yaml")
        || path.ends_with(".yml")
}
