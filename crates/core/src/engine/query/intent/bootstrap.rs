use std::collections::HashMap;

use crate::model::{ContextMode, SearchHit};

#[path = "bootstrap_helpers.rs"]
mod helpers;

use super::roles::{FileDomain, FileRole};
use super::{SearchIntent, is_test_path};
use helpers::{
    module_root_bucket, promote_required_buckets, push_unique, role_bucket, root_bucket,
    should_defer_project_map_artifact,
};

const DIVERSIFY_WAVES: &[(usize, usize)] = &[(1, 1), (1, 2), (2, 2)];

impl SearchIntent {
    pub(crate) fn diversify_hits(
        &self,
        hits: &mut Vec<SearchHit>,
        context_mode: Option<ContextMode>,
    ) {
        let should_diversify = self.should_diversify_hits(context_mode)
            || self.should_diversify_from_hit_distribution(hits, context_mode);
        if !should_diversify || hits.len() < 3 {
            return;
        }

        let mut remaining = std::mem::take(hits);
        let mut selected = Vec::with_capacity(remaining.len());
        let mut role_counts = HashMap::<String, usize>::new();
        let mut root_counts = HashMap::<String, usize>::new();

        for (wave_idx, &(max_role, max_root)) in DIVERSIFY_WAVES.iter().enumerate() {
            let mut deferred = Vec::with_capacity(remaining.len());
            let mut made_progress = false;
            for hit in remaining {
                if wave_idx + 1 < DIVERSIFY_WAVES.len() && should_defer_project_map_artifact(&hit) {
                    deferred.push(hit);
                    continue;
                }
                let role_bucket = role_bucket(&hit);
                let root_bucket = root_bucket(&hit.path);
                let allow = role_counts.get(&role_bucket).copied().unwrap_or(0) < max_role
                    && root_counts.get(&root_bucket).copied().unwrap_or(0) < max_root;
                if allow {
                    *role_counts.entry(role_bucket).or_default() += 1;
                    *root_counts.entry(root_bucket).or_default() += 1;
                    selected.push(hit);
                    made_progress = true;
                } else {
                    deferred.push(hit);
                }
            }
            remaining = deferred;
            if remaining.is_empty() || !made_progress {
                break;
            }
        }

        selected.extend(remaining);
        promote_required_buckets(self, &mut selected);
        *hits = selected;
    }

    pub(crate) fn bootstrap_followups(&self, hits: &[SearchHit]) -> Vec<String> {
        let should_emit_followups = self.should_diversify_hits(None)
            || self.should_diversify_from_hit_distribution(hits, None);
        if !should_emit_followups || hits.is_empty() {
            return Vec::new();
        }

        let mut saw_backend = false;
        let mut saw_frontend = false;
        let mut saw_database = false;
        let mut saw_tests = false;

        for hit in hits.iter().take(8) {
            let role = FileRole::from_path_language(&hit.path, &hit.language);
            match role.domain {
                FileDomain::Backend => saw_backend = true,
                FileDomain::Frontend => saw_frontend = true,
                FileDomain::Database => saw_database = true,
                FileDomain::Docs | FileDomain::Other => {}
            }
            saw_tests |= is_test_path(&hit.path);
        }

        let mut followups = Vec::new();
        if saw_backend && saw_frontend {
            push_unique(
                &mut followups,
                "Follow one end-to-end flow from the surfaced frontend entrypoint into the backend API or service layer.",
            );
        }
        if self.wants_tests && !saw_tests {
            push_unique(
                &mut followups,
                "Inspect tests or contract fixtures covering the surfaced code paths.",
            );
        }
        if !saw_database
            && (self.wants_auth_boundary
                || self.wants_tests
                || self.coverage_groups.len() >= 3
                || (saw_backend && saw_frontend))
        {
            push_unique(
                &mut followups,
                "Inspect migrations, schemas, or persistence adapters adjacent to the surfaced code paths.",
            );
        }
        if self.wants_auth_boundary {
            push_unique(
                &mut followups,
                "Trace authentication and authorization checks across the surfaced request boundaries.",
            );
        }
        if self.expects_mod_runtime_surface() {
            push_unique(
                &mut followups,
                "Compare module entrypoints, mixins, and runtime handlers across the surfaced module roots.",
            );
            push_unique(
                &mut followups,
                "Inspect config stores, client hooks, and render modules adjacent to the surfaced module roots.",
            );
        }
        if self.prefers_backend && !saw_backend {
            push_unique(
                &mut followups,
                "Trace backend entrypoints, handlers, and service boundaries related to this query.",
            );
        }
        if self.prefers_frontend && !saw_frontend {
            push_unique(
                &mut followups,
                "Inspect frontend pages, hooks, or components connected to the surfaced code paths.",
            );
        }
        if followups.is_empty() {
            push_unique(
                &mut followups,
                "Inspect adjacent entrypoints, orchestration, and tests around the surfaced code paths.",
            );
        }

        followups.truncate(3);
        followups
    }

    fn should_diversify_hits(&self, context_mode: Option<ContextMode>) -> bool {
        if matches!(context_mode, Some(ContextMode::Design)) || self.token_count < 3 {
            return false;
        }
        if self.wants_architecture || self.wants_entrypoints {
            return true;
        }
        if self.prefers_backend && self.prefers_frontend {
            return true;
        }
        if (self.wants_auth_boundary || self.wants_tests) && self.token_count >= 4 {
            return true;
        }
        if self.expects_mod_runtime_surface() {
            return true;
        }
        self.wants_api_surface
            && self.wants_service_layer
            && self.coverage_groups.len() >= 2
            && self.token_count >= 5
    }

    fn should_diversify_from_hit_distribution(
        &self,
        hits: &[SearchHit],
        context_mode: Option<ContextMode>,
    ) -> bool {
        if matches!(context_mode, Some(ContextMode::Design)) || self.token_count < 6 {
            return false;
        }

        let mut root_counts = HashMap::<String, usize>::new();
        let mut role_counts = HashMap::<String, usize>::new();
        let mut code_like_hits = 0usize;
        let mut has_backend = false;
        let mut has_frontend = false;
        let mut has_database = false;
        let mut has_tests = false;
        let mut has_mod_runtime = false;

        for hit in hits.iter().take(8) {
            if should_defer_project_map_artifact(hit) {
                continue;
            }
            let role = FileRole::from_path_language(&hit.path, &hit.language);
            let module_root = module_root_bucket(&hit.path);
            if role.domain == FileDomain::Other && module_root.is_none() {
                continue;
            }
            code_like_hits += 1;
            *root_counts.entry(root_bucket(&hit.path)).or_default() += 1;
            *role_counts.entry(role_bucket(hit)).or_default() += 1;
            match role.domain {
                FileDomain::Backend => has_backend = true,
                FileDomain::Frontend => has_frontend = true,
                FileDomain::Database => has_database = true,
                FileDomain::Docs | FileDomain::Other => {}
            }
            has_tests |= is_test_path(&hit.path);
            has_mod_runtime |= module_root.is_some();
        }

        if code_like_hits < 4 {
            return false;
        }

        let dominant_root = root_counts.values().copied().max().unwrap_or(0);
        let dominant_role = role_counts.values().copied().max().unwrap_or(0);
        let cross_layer_signal = (has_backend && has_frontend)
            || has_tests
            || has_database
            || has_mod_runtime
            || role_counts.len() >= 3;

        cross_layer_signal
            && root_counts.len() >= 2
            && (dominant_root * 2 >= code_like_hits || dominant_role * 2 >= code_like_hits)
    }
}
