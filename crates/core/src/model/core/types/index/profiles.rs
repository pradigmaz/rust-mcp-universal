use serde::{Deserialize, Serialize};

use crate::model::core::parse;

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
    "*.toml",
    "**/*.toml",
    "*.json",
    "**/*.json",
    "*.yaml",
    "**/*.yaml",
    "*.yml",
    "**/*.yml",
    "*.ini",
    "**/*.ini",
    "*.cfg",
    "**/*.cfg",
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
    "Cargo.lock",
    "**/Cargo.lock",
    ".gitignore",
    "**/.gitignore",
    ".editorconfig",
    "**/.editorconfig",
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
