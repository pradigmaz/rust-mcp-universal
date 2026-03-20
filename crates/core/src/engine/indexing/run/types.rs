use std::collections::{HashMap, HashSet};

use crate::engine::storage;
use crate::utils::ProjectIgnoreMatcher;

#[derive(Debug, Default)]
pub(super) struct RunStats {
    pub(super) scanned: usize,
    pub(super) indexed: usize,
    pub(super) skipped: usize,
    pub(super) skipped_before_changed_since: usize,
    pub(super) added: usize,
    pub(super) changed: usize,
    pub(super) unchanged: usize,
    pub(super) deleted: usize,
    pub(super) embedding_cache_hits: usize,
    pub(super) embedding_cache_misses: usize,
}

#[derive(Debug, Default)]
pub(super) struct PassResult {
    pub(super) stats: RunStats,
    pub(super) present_paths: HashSet<String>,
    pub(super) failed_paths: HashSet<String>,
    pub(super) authoritative_deleted_paths: HashSet<String>,
    pub(super) failed_walk_prefixes: Vec<String>,
}

pub(super) struct PreparedIndexRun {
    pub(super) existing_files: HashMap<String, storage::ExistingFileState>,
    pub(super) selector: RunSelector,
    pub(super) ignore_matcher: ProjectIgnoreMatcher,
}

pub(crate) enum RunSelector {
    Full,
    Timestamp { changed_since_unix_ms: i64 },
    Commit(CommitSelector),
}

pub(crate) struct CommitSelector {
    pub(crate) candidate_paths: HashSet<String>,
    pub(crate) deleted_paths: HashSet<String>,
    pub(crate) resolved_merge_base_commit: String,
}
