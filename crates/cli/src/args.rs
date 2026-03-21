use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "rmu")]
#[command(about = "Rust MCP Universal CLI")]
pub(crate) struct App {
    #[arg(long, default_value = ".", global = true)]
    pub(crate) project_path: PathBuf,
    #[arg(long, global = true)]
    pub(crate) db_path: Option<PathBuf>,
    #[arg(long, short = 'j', default_value_t = false, global = true)]
    pub(crate) json: bool,
    #[arg(long, default_value = "off", global = true)]
    pub(crate) privacy_mode: String,
    #[arg(long, default_value_t = true, global = true)]
    pub(crate) vector_layer_enabled: bool,
    #[arg(long, default_value = "full_100", global = true)]
    pub(crate) rollout_phase: String,
    #[arg(long, default_value = "auto", global = true)]
    pub(crate) migration_mode: String,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    Index(IndexCommandArgs),
    SemanticIndex(IndexCommandArgs),
    ScopePreview(IndexCommandArgs),
    InstallIgnoreRules {
        #[arg(long, default_value = "git-info-exclude")]
        target: String,
    },
    DeleteIndex {
        #[arg(long, default_value_t = false)]
        yes: bool,
    },
    DbMaintenance {
        #[arg(long, default_value_t = false)]
        integrity_check: bool,
        #[arg(long, default_value_t = false)]
        checkpoint: bool,
        #[arg(long, default_value_t = false)]
        vacuum: bool,
        #[arg(long, default_value_t = false)]
        analyze: bool,
        #[arg(long, default_value_t = false)]
        stats: bool,
        #[arg(long, default_value_t = false)]
        prune: bool,
    },
    Status,
    Search {
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        detailed: bool,
        #[arg(long, default_value_t = false)]
        semantic: bool,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
        #[arg(long, default_value = "fail_open")]
        semantic_fail_mode: String,
    },
    SemanticSearch {
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
        #[arg(long, default_value = "fail_open")]
        semantic_fail_mode: String,
    },
    SymbolLookup {
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
    },
    SymbolReferences {
        #[arg(long)]
        name: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
    },
    RelatedFiles {
        #[arg(long)]
        path: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
    },
    CallPath {
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long, default_value_t = 6)]
        max_hops: usize,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
    },
    Context {
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        semantic: bool,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
        #[arg(long, default_value = "fail_open")]
        semantic_fail_mode: String,
        #[arg(long, default_value_t = 12_000)]
        max_chars: usize,
        #[arg(long, default_value_t = 3_000)]
        max_tokens: usize,
    },
    ContextPack {
        #[arg(long)]
        query: String,
        #[arg(long)]
        mode: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        semantic: bool,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
        #[arg(long, default_value = "fail_open")]
        semantic_fail_mode: String,
        #[arg(long, default_value_t = 12_000)]
        max_chars: usize,
        #[arg(long, default_value_t = 3_000)]
        max_tokens: usize,
    },
    Report {
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        semantic: bool,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
        #[arg(long, default_value = "fail_open")]
        semantic_fail_mode: String,
        #[arg(long, default_value_t = 12_000)]
        max_chars: usize,
        #[arg(long, default_value_t = 3_000)]
        max_tokens: usize,
    },
    QueryBenchmark {
        #[arg(long)]
        dataset: PathBuf,
        #[arg(long, default_value_t = 10)]
        k: usize,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        semantic: bool,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
        #[arg(long, default_value = "fail_open")]
        semantic_fail_mode: String,
        #[arg(long, default_value_t = 12_000)]
        max_chars: usize,
        #[arg(long, default_value_t = 3_000)]
        max_tokens: usize,
        #[arg(long)]
        baseline: Option<PathBuf>,
        #[arg(long)]
        thresholds: Option<PathBuf>,
        #[arg(long, default_value_t = 1)]
        runs: usize,
        #[arg(long, default_value_t = false)]
        enforce_gates: bool,
    },
    Brief,
    Agent {
        #[arg(long)]
        query: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = false)]
        semantic: bool,
        #[arg(long, default_value_t = false)]
        auto_index: bool,
        #[arg(long, default_value = "fail_open")]
        semantic_fail_mode: String,
        #[arg(long, default_value_t = 12_000)]
        max_chars: usize,
        #[arg(long, default_value_t = 3_000)]
        max_tokens: usize,
    },
}

#[derive(Args, Debug, Clone, Default)]
pub(crate) struct IndexCommandArgs {
    #[arg(long)]
    pub(crate) profile: Option<String>,
    #[arg(long)]
    pub(crate) changed_since: Option<String>,
    #[arg(long)]
    pub(crate) changed_since_commit: Option<String>,
    #[arg(long = "include")]
    pub(crate) include_paths: Vec<String>,
    #[arg(long = "exclude")]
    pub(crate) exclude_paths: Vec<String>,
    #[arg(long, default_value_t = false)]
    pub(crate) reindex: bool,
}
