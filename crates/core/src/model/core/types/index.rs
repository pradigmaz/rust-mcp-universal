use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::super::{parse, serde_glue};

const RUST_MONOREPO_INCLUDES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain",
    "rust-toolchain.toml",
    ".cargo",
    "crates",
    "src",
    "tests",
    "examples",
    "benches",
];
const MIXED_INCLUDES: &[&str] = &[
    "*.rs",
    "**/*.rs",
    "*.py",
    "**/*.py",
    "*.js",
    "**/*.js",
    "*.jsx",
    "**/*.jsx",
    "*.mjs",
    "**/*.mjs",
    "*.cjs",
    "**/*.cjs",
    "*.ts",
    "**/*.ts",
    "*.tsx",
    "**/*.tsx",
    "*.go",
    "**/*.go",
    "*.java",
    "**/*.java",
    "*.c",
    "**/*.c",
    "*.h",
    "**/*.h",
    "*.cpp",
    "**/*.cpp",
    "*.cc",
    "**/*.cc",
    "*.cxx",
    "**/*.cxx",
    "*.hpp",
    "**/*.hpp",
    "*.hh",
    "**/*.hh",
    "*.cs",
    "**/*.cs",
    "*.php",
    "**/*.php",
    "*.rb",
    "**/*.rb",
    "*.kt",
    "**/*.kt",
    "*.kts",
    "**/*.kts",
    "*.swift",
    "**/*.swift",
    "*.scala",
    "**/*.scala",
    "*.sc",
    "**/*.sc",
    "*.lua",
    "**/*.lua",
    "*.sh",
    "**/*.sh",
    "*.bash",
    "**/*.bash",
    "*.zsh",
    "**/*.zsh",
    "*.ps1",
    "**/*.ps1",
    "*.sql",
    "**/*.sql",
    "*.html",
    "**/*.html",
    "*.css",
    "**/*.css",
    "*.scss",
    "**/*.scss",
    "*.sass",
    "**/*.sass",
    "*.less",
    "**/*.less",
    "*.vue",
    "**/*.vue",
    "*.svelte",
    "**/*.svelte",
];
const MIXED_EXCLUDES: &[&str] = &[
    "dist",
    "**/dist/**",
    "build",
    "**/build/**",
    "coverage",
    "**/coverage/**",
    ".cache",
    "**/.cache/**",
    ".turbo",
    "**/.turbo/**",
    ".next",
    "**/.next/**",
    ".nuxt",
    "**/.nuxt/**",
    ".svelte-kit",
    "**/.svelte-kit/**",
    "out",
    "**/out/**",
    ".codex",
    "**/.codex/**",
    ".codex-planning",
    "**/.codex-planning/**",
    "semgrep.err",
    "**/semgrep.err",
    "semgrep.json",
    "**/semgrep.json",
    "semgrep.out",
    "**/semgrep.out",
];
const DOCS_HEAVY_INCLUDES: &[&str] = &[
    "docs",
    "schemas",
    "README.md",
    "*.md",
    "**/*.md",
    "*.mdx",
    "**/*.mdx",
    "*.rst",
    "**/*.rst",
    "*.txt",
    "**/*.txt",
    "*.toml",
    "**/*.toml",
    "*.json",
    "**/*.json",
];
const DOCS_HEAVY_EXCLUDES: &[&str] = &[
    "crates/**",
    "src/**",
    "tests/**",
    "examples/**",
    "benches/**",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStatus {
    pub project_root: String,
    pub db_path: String,
    pub files: usize,
    pub symbols: usize,
    pub module_deps: usize,
    pub refs: usize,
    pub semantic_vectors: usize,
    pub file_chunks: usize,
    pub chunk_embeddings: usize,
    pub semantic_model: String,
    pub last_index_lock_wait_ms: u64,
    pub last_embedding_cache_hits: usize,
    pub last_embedding_cache_misses: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IndexProfile {
    RustMonorepo,
    Mixed,
    DocsHeavy,
}

impl IndexProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RustMonorepo => "rust-monorepo",
            Self::Mixed => "mixed",
            Self::DocsHeavy => "docs-heavy",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        parse::index_profile(value)
    }

    pub(crate) const fn include_paths(self) -> &'static [&'static str] {
        match self {
            Self::RustMonorepo => RUST_MONOREPO_INCLUDES,
            Self::Mixed => MIXED_INCLUDES,
            Self::DocsHeavy => DOCS_HEAVY_INCLUDES,
        }
    }

    pub(crate) const fn exclude_paths(self) -> &'static [&'static str] {
        match self {
            Self::RustMonorepo => &[],
            Self::Mixed => MIXED_EXCLUDES,
            Self::DocsHeavy => DOCS_HEAVY_EXCLUDES,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndexingOptions {
    #[serde(default)]
    pub profile: Option<IndexProfile>,
    #[serde(
        default,
        serialize_with = "serde_glue::serialize_optional_offset_datetime_rfc3339",
        deserialize_with = "serde_glue::deserialize_optional_offset_datetime_rfc3339"
    )]
    pub changed_since: Option<OffsetDateTime>,
    #[serde(default)]
    pub changed_since_commit: Option<String>,
    #[serde(default)]
    pub include_paths: Vec<String>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    #[serde(default)]
    pub reindex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopePreviewResult {
    #[serde(default)]
    pub profile: Option<IndexProfile>,
    #[serde(
        default,
        serialize_with = "serde_glue::serialize_optional_offset_datetime_rfc3339",
        deserialize_with = "serde_glue::deserialize_optional_offset_datetime_rfc3339"
    )]
    pub changed_since: Option<OffsetDateTime>,
    #[serde(default)]
    pub changed_since_commit: Option<String>,
    #[serde(default)]
    pub resolved_merge_base_commit: Option<String>,
    #[serde(default)]
    pub reindex: bool,
    #[serde(default)]
    pub include_paths: Vec<String>,
    #[serde(default)]
    pub exclude_paths: Vec<String>,
    pub scanned_files: usize,
    pub candidate_count: usize,
    pub excluded_by_scope_count: usize,
    pub ignored_count: usize,
    pub skipped_before_changed_since_count: usize,
    pub repair_backfill_count: usize,
    pub deleted_count: usize,
    #[serde(default)]
    pub candidate_paths: Vec<String>,
    #[serde(default)]
    pub excluded_by_scope_paths: Vec<String>,
    #[serde(default)]
    pub ignored_paths: Vec<String>,
    #[serde(default)]
    pub skipped_before_changed_since_paths: Vec<String>,
    #[serde(default)]
    pub repair_backfill_paths: Vec<String>,
    #[serde(default)]
    pub deleted_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteIndexResult {
    pub db_path: String,
    pub removed_count: usize,
    pub removed_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct DbMaintenanceOptions {
    #[serde(default)]
    pub integrity_check: bool,
    #[serde(default)]
    pub checkpoint: bool,
    #[serde(default)]
    pub vacuum: bool,
    #[serde(default)]
    pub analyze: bool,
    #[serde(default)]
    pub stats: bool,
    #[serde(default)]
    pub prune: bool,
}

impl DbMaintenanceOptions {
    pub fn normalized(self) -> Self {
        if self.integrity_check
            || self.checkpoint
            || self.vacuum
            || self.analyze
            || self.stats
            || self.prune
        {
            self
        } else {
            Self {
                integrity_check: true,
                checkpoint: true,
                vacuum: true,
                analyze: true,
                stats: true,
                prune: true,
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbCheckpointResult {
    pub busy: i64,
    pub wal_pages: i64,
    pub checkpointed_pages: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMaintenanceStats {
    pub page_size: i64,
    pub page_count: i64,
    pub freelist_count: i64,
    pub approx_free_bytes: u64,
    pub db_size_bytes: u64,
    pub wal_size_bytes: u64,
    pub shm_size_bytes: u64,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DbPruneResult {
    pub removed_databases: usize,
    pub removed_sidecars: usize,
    pub removed_bytes: u64,
    pub removed_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DbMaintenanceResult {
    pub db_path: String,
    pub options: DbMaintenanceOptions,
    pub integrity_ok: Option<bool>,
    pub integrity_message: Option<String>,
    pub checkpoint: Option<DbCheckpointResult>,
    pub vacuum_ran: bool,
    pub analyze_ran: bool,
    pub stats: Option<DbMaintenanceStats>,
    pub prune: Option<DbPruneResult>,
}
