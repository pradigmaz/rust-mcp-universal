use crate::model::SearchHit;

#[path = "bootstrap_mod.rs"]
mod bootstrap_mod;

use super::super::roles::{BackendLayer, FileDomain, FileRole};
use super::super::{SearchIntent, is_test_path};
use bootstrap_mod::rebalance_mod_runtime_prefix;

const GENERIC_SEGMENTS: &[&str] = &["src", "app", "lib", "internal", "packages", "crates", "cmd"];
const CONTAINER_SEGMENTS: &[&str] = &["mods", "plugins"];

pub(super) fn role_bucket(hit: &SearchHit) -> String {
    let role = FileRole::from_path_language(&hit.path, &hit.language);
    if is_test_path(&hit.path) {
        return "tests".to_string();
    }
    if role.support_artifact_like {
        return "support".to_string();
    }
    if module_root_bucket(&hit.path).is_some() {
        if role.mod_entrypoint_like {
            return "mod_entrypoint".to_string();
        }
        if role.mixin_like {
            return "mod_mixin".to_string();
        }
        if role.module_like {
            return "mod_module".to_string();
        }
        if role.config_like {
            return "mod_config".to_string();
        }
        if role.runtime_like {
            return "mod_runtime".to_string();
        }
    }
    match role.domain {
        FileDomain::Backend => match role.backend_layer {
            BackendLayer::ApiSurface => "backend_api".to_string(),
            BackendLayer::ServiceWork => "backend_service".to_string(),
            BackendLayer::Other => "backend_other".to_string(),
        },
        FileDomain::Frontend => {
            if role.hook_like {
                "frontend_hook".to_string()
            } else if role.page_like {
                "frontend_page".to_string()
            } else if role.component_like {
                "frontend_component".to_string()
            } else {
                "frontend_other".to_string()
            }
        }
        FileDomain::Database => {
            if role.migration_like {
                "database_migration".to_string()
            } else if role.schema_like {
                "database_schema".to_string()
            } else {
                "database_other".to_string()
            }
        }
        FileDomain::Docs => "docs".to_string(),
        FileDomain::Other => "other".to_string(),
    }
}

pub(super) fn should_defer_project_map_artifact(hit: &SearchHit) -> bool {
    let role = FileRole::from_path_language(&hit.path, &hit.language);
    role.support_artifact_like || role.domain == FileDomain::Docs
}

pub(super) fn root_bucket(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    let segments = normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let Some(mut root) = segments.first().copied() else {
        return "root".to_string();
    };
    if GENERIC_SEGMENTS.contains(&root) {
        root = segments
            .iter()
            .copied()
            .find(|segment| !GENERIC_SEGMENTS.contains(segment))
            .unwrap_or(root);
    }
    if CONTAINER_SEGMENTS.contains(&root) {
        if let Some(module_root) = segments
            .iter()
            .copied()
            .skip(1)
            .find(|segment| !GENERIC_SEGMENTS.contains(segment))
        {
            return format!(
                "{}/{}",
                root.to_ascii_lowercase(),
                module_root.to_ascii_lowercase()
            );
        }
    }
    root.to_ascii_lowercase()
}

pub(super) fn push_unique(target: &mut Vec<String>, value: &str) {
    if target.iter().any(|existing| existing == value) {
        return;
    }
    target.push(value.to_string());
}

pub(super) fn promote_required_buckets(intent: &SearchIntent, hits: &mut Vec<SearchHit>) {
    if hits.len() < 3 {
        return;
    }

    let window = hits.len().min(5);

    if intent.wants_tests && !hits.iter().take(window).any(|hit| is_test_path(&hit.path)) {
        promote_first_match(hits, window.saturating_sub(2), |hit| {
            is_test_path(&hit.path)
        });
    }

    if intent.wants_api_surface
        && intent.wants_service_layer
        && !hits.iter().take(window).any(is_api_hit)
    {
        promote_first_match(hits, 1, is_api_hit);
    }

    if intent.wants_service_layer && !hits.iter().take(window).any(is_service_hit) {
        promote_first_match(hits, window.saturating_sub(1), is_service_hit);
    }

    if intent.expects_mod_runtime_surface()
        && let Some(missing_module_root) = missing_module_root_in_window(hits, window)
    {
        promote_first_match(hits, 1, |hit| {
            module_root_bucket(&hit.path).as_deref() == Some(missing_module_root.as_str())
        });
    }

    if intent.expects_mod_runtime_surface()
        && !hits.iter().take(window).any(is_mod_foundational_hit)
    {
        promote_first_match(hits, 2, is_mod_foundational_hit);
    }

    if intent.expects_mod_runtime_surface() {
        rebalance_mod_runtime_prefix(hits, window.max(6).saturating_add(4));
    }
}

fn promote_first_match(
    hits: &mut Vec<SearchHit>,
    target_idx: usize,
    predicate: impl Fn(&SearchHit) -> bool,
) {
    let Some(found_idx) = hits.iter().position(predicate) else {
        return;
    };
    let target_idx = target_idx.min(hits.len().saturating_sub(1));
    if found_idx <= target_idx {
        return;
    }
    let hit = hits.remove(found_idx);
    hits.insert(target_idx, hit);
}

fn is_api_hit(hit: &SearchHit) -> bool {
    let role = FileRole::from_path_language(&hit.path, &hit.language);
    role.domain == FileDomain::Backend && role.backend_layer == BackendLayer::ApiSurface
}

fn is_service_hit(hit: &SearchHit) -> bool {
    let role = FileRole::from_path_language(&hit.path, &hit.language);
    role.domain == FileDomain::Backend && role.backend_layer == BackendLayer::ServiceWork
}

fn missing_module_root_in_window(hits: &[SearchHit], window: usize) -> Option<String> {
    let visible = hits
        .iter()
        .take(window)
        .filter_map(|hit| module_root_bucket(&hit.path))
        .collect::<Vec<_>>();
    if visible.is_empty() {
        return None;
    }
    hits.iter()
        .skip(window)
        .filter_map(|hit| module_root_bucket(&hit.path))
        .find(|bucket| {
            !visible
                .iter()
                .any(|visible_bucket| visible_bucket == bucket)
        })
}

pub(super) fn module_root_bucket(path: &str) -> Option<String> {
    let normalized = path.replace('\\', "/");
    let mut segments = normalized.split('/').filter(|segment| !segment.is_empty());
    let container = segments.next()?;
    if !CONTAINER_SEGMENTS.contains(&container) {
        return None;
    }
    let module = segments.next()?;
    Some(format!(
        "{}/{}",
        container.to_ascii_lowercase(),
        module.to_ascii_lowercase()
    ))
}

fn is_mod_foundational_hit(hit: &SearchHit) -> bool {
    let role = FileRole::from_path_language(&hit.path, &hit.language);
    module_root_bucket(&hit.path).is_some()
        && (role.mod_entrypoint_like || role.mixin_like || role.module_like || role.config_like)
}
