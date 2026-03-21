use anyhow::Result;
use rmu_core::{
    Engine, IndexProfile, IndexingOptions, PrivacyMode, sanitize_path_text,
    sanitize_value_for_privacy,
};

use crate::args::IndexCommandArgs;
use crate::output::{print_json, print_line};

use super::modes::{
    format_changed_since, parse_changed_since, parse_changed_since_commit, parse_index_profile,
};

pub(crate) fn run_index(engine: &Engine, json: bool, index_args: IndexCommandArgs) -> Result<()> {
    let options = to_indexing_options(&index_args)?;
    let effective_options = engine.resolve_indexing_options(&options);
    let summary = engine.index_path_with_options(&effective_options)?;

    if json {
        print_json(serde_json::to_string_pretty(&serde_json::json!({
            "profile": effective_options.profile.map(IndexProfile::as_str),
            "changed_since": options.changed_since.map(format_changed_since).transpose()?,
            "changed_since_commit": summary.changed_since_commit,
            "resolved_merge_base_commit": summary.resolved_merge_base_commit,
            "reindex": options.reindex,
            "include_paths": options.include_paths,
            "exclude_paths": options.exclude_paths,
            "scanned": summary.scanned,
            "indexed": summary.indexed,
            "skipped_binary_or_large": summary.skipped_binary_or_large,
            "skipped_before_changed_since": summary.skipped_before_changed_since,
            "added": summary.added,
            "changed": summary.changed,
            "unchanged": summary.unchanged,
            "deleted": summary.deleted,
            "lock_wait_ms": summary.lock_wait_ms,
            "embedding_cache_hits": summary.embedding_cache_hits,
            "embedding_cache_misses": summary.embedding_cache_misses
        })))?;
    } else {
        print_line(format!(
            "indexed={}, scanned={}, skipped={}, skipped_before_changed_since={}, added={}, changed={}, unchanged={}, deleted={}, lock_wait_ms={}, cache_hits={}, cache_misses={}, profile={}, changed_since={}, changed_since_commit={}, resolved_merge_base_commit={}, reindex={}, include_paths={}, exclude_paths={}",
            summary.indexed,
            summary.scanned,
            summary.skipped_binary_or_large,
            summary.skipped_before_changed_since,
            summary.added,
            summary.changed,
            summary.unchanged,
            summary.deleted,
            summary.lock_wait_ms,
            summary.embedding_cache_hits,
            summary.embedding_cache_misses,
            profile_label(effective_options.profile),
            changed_since_label(summary.changed_since)?,
            summary
                .changed_since_commit
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            summary
                .resolved_merge_base_commit
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            options.reindex,
            options.include_paths.join(","),
            options.exclude_paths.join(",")
        ));
    }

    Ok(())
}

pub(crate) fn run_semantic_index(
    engine: &Engine,
    json: bool,
    index_args: IndexCommandArgs,
) -> Result<()> {
    let options = to_indexing_options(&index_args)?;
    let effective_options = engine.resolve_indexing_options(&options);
    let summary = engine.index_path_with_options(&effective_options)?;
    let semantic_vectors_rebuilt = summary.changed > 0 || summary.added > 0;

    if json {
        print_json(serde_json::to_string_pretty(&serde_json::json!({
            "profile": effective_options.profile.map(IndexProfile::as_str),
            "changed_since": options.changed_since.map(format_changed_since).transpose()?,
            "changed_since_commit": summary.changed_since_commit,
            "resolved_merge_base_commit": summary.resolved_merge_base_commit,
            "reindex": options.reindex,
            "include_paths": options.include_paths,
            "exclude_paths": options.exclude_paths,
            "scanned": summary.scanned,
            "indexed": summary.indexed,
            "skipped_binary_or_large": summary.skipped_binary_or_large,
            "skipped_before_changed_since": summary.skipped_before_changed_since,
            "semantic_vectors_rebuilt": semantic_vectors_rebuilt,
            "added": summary.added,
            "changed": summary.changed,
            "unchanged": summary.unchanged,
            "deleted": summary.deleted,
            "lock_wait_ms": summary.lock_wait_ms,
            "embedding_cache_hits": summary.embedding_cache_hits,
            "embedding_cache_misses": summary.embedding_cache_misses
        })))?;
    } else {
        print_line(format!(
            "semantic index updated: indexed={}, scanned={}, skipped={}, skipped_before_changed_since={}, semantic_vectors_rebuilt={}, added={}, changed={}, unchanged={}, deleted={}, lock_wait_ms={}, cache_hits={}, cache_misses={}, profile={}, changed_since={}, changed_since_commit={}, resolved_merge_base_commit={}, reindex={}, include_paths={}, exclude_paths={}",
            summary.indexed,
            summary.scanned,
            summary.skipped_binary_or_large,
            summary.skipped_before_changed_since,
            semantic_vectors_rebuilt,
            summary.added,
            summary.changed,
            summary.unchanged,
            summary.deleted,
            summary.lock_wait_ms,
            summary.embedding_cache_hits,
            summary.embedding_cache_misses,
            profile_label(effective_options.profile),
            changed_since_label(summary.changed_since)?,
            summary
                .changed_since_commit
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            summary
                .resolved_merge_base_commit
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            options.reindex,
            options.include_paths.join(","),
            options.exclude_paths.join(",")
        ));
    }

    Ok(())
}

pub(crate) fn run_scope_preview(
    engine: &Engine,
    json: bool,
    privacy_mode: PrivacyMode,
    index_args: IndexCommandArgs,
) -> Result<()> {
    let options = to_indexing_options(&index_args)?;
    let preview = engine.scope_preview_with_options(&options)?;

    if json {
        let mut value = serde_json::to_value(&preview)?;
        sanitize_value_for_privacy(privacy_mode, &mut value);
        print_json(serde_json::to_string_pretty(&value))?;
    } else {
        print_line(format!(
            "scope preview: scanned_files={}, candidate_count={}, excluded_by_scope_count={}, ignored_count={}, skipped_before_changed_since_count={}, repair_backfill_count={}, deleted_count={}, profile={}, changed_since={}, changed_since_commit={}, resolved_merge_base_commit={}, reindex={}, include_paths={}, exclude_paths={}",
            preview.scanned_files,
            preview.candidate_count,
            preview.excluded_by_scope_count,
            preview.ignored_count,
            preview.skipped_before_changed_since_count,
            preview.repair_backfill_count,
            preview.deleted_count,
            profile_label(preview.profile),
            changed_since_label(preview.changed_since)?,
            preview
                .changed_since_commit
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            preview
                .resolved_merge_base_commit
                .clone()
                .unwrap_or_else(|| "none".to_string()),
            preview.reindex,
            preview.include_paths.join(","),
            preview.exclude_paths.join(",")
        ));
        print_preview_bucket(
            "candidate_paths",
            &preview.candidate_paths,
            privacy_mode,
            20,
        );
        print_preview_bucket(
            "excluded_by_scope_paths",
            &preview.excluded_by_scope_paths,
            privacy_mode,
            20,
        );
        print_preview_bucket("ignored_paths", &preview.ignored_paths, privacy_mode, 20);
        print_preview_bucket(
            "skipped_before_changed_since_paths",
            &preview.skipped_before_changed_since_paths,
            privacy_mode,
            20,
        );
        print_preview_bucket(
            "repair_backfill_paths",
            &preview.repair_backfill_paths,
            privacy_mode,
            20,
        );
        print_preview_bucket("deleted_paths", &preview.deleted_paths, privacy_mode, 20);
    }

    Ok(())
}

fn to_indexing_options(args: &IndexCommandArgs) -> Result<IndexingOptions> {
    Ok(IndexingOptions {
        profile: args
            .profile
            .as_deref()
            .map(parse_index_profile)
            .transpose()?,
        changed_since: args
            .changed_since
            .as_deref()
            .map(parse_changed_since)
            .transpose()?,
        changed_since_commit: args
            .changed_since_commit
            .as_deref()
            .map(parse_changed_since_commit)
            .transpose()?,
        include_paths: args.include_paths.clone(),
        exclude_paths: args.exclude_paths.clone(),
        reindex: args.reindex,
    })
}

fn profile_label(profile: Option<IndexProfile>) -> &'static str {
    profile.map(IndexProfile::as_str).unwrap_or("none")
}

fn changed_since_label(changed_since: Option<time::OffsetDateTime>) -> Result<String> {
    changed_since
        .map(format_changed_since)
        .transpose()
        .map(|value| value.unwrap_or_else(|| "none".to_string()))
}

fn print_preview_bucket(label: &str, paths: &[String], privacy_mode: PrivacyMode, limit: usize) {
    if paths.is_empty() {
        print_line(format!("{label}=none"));
        return;
    }

    let shown = paths
        .iter()
        .take(limit)
        .map(|path| sanitize_path_text(privacy_mode, path))
        .collect::<Vec<_>>();
    let remaining = paths.len().saturating_sub(shown.len());
    let suffix = if remaining == 0 {
        String::new()
    } else {
        format!(" … +{remaining} more")
    };
    print_line(format!("{label}={}", shown.join(", ")));
    if !suffix.is_empty() {
        print_line(format!("{label}_truncated={suffix}"));
    }
}

#[cfg(test)]
mod tests {
    use super::to_indexing_options;
    use crate::args::IndexCommandArgs;

    #[test]
    fn to_indexing_options_preserves_changed_since_commit() {
        let options = to_indexing_options(&IndexCommandArgs {
            changed_since_commit: Some("HEAD~1".to_string()),
            ..IndexCommandArgs::default()
        })
        .expect("options");

        assert_eq!(options.changed_since_commit.as_deref(), Some("HEAD~1"));
        assert!(options.changed_since.is_none());
    }
}
