use anyhow::{Context, Result};

use crate::model::ContextMode;
use crate::text_utils::is_low_priority_path;
use crate::vector_rank::SemanticRerankOutcome;

pub(super) use super::support_fusion::{FusionProfile, derive_fusion_profile, seed_fusion_profile};

pub(super) fn semantic_outcome_code(
    semantic_requested: bool,
    semantic_outcome: SemanticRerankOutcome,
) -> &'static str {
    if !semantic_requested {
        return "not_requested";
    }
    match semantic_outcome {
        SemanticRerankOutcome::AppliedRrfIndexed => "applied_indexed",
        SemanticRerankOutcome::AppliedRrfFallback => "applied_fallback",
        SemanticRerankOutcome::AppliedRrfMixed => "applied_mixed",
        SemanticRerankOutcome::ShortCircuitedLexical => "short_circuit_lexical",
        SemanticRerankOutcome::Failed => "failed",
        SemanticRerankOutcome::NotApplied => "not_applied",
    }
}

pub(super) fn db_limit_for(candidate_limit: usize) -> Result<i64> {
    i64::try_from(candidate_limit).with_context(|| {
        format!(
            "query `limit` value {candidate_limit} exceeds maximum supported value {}",
            i64::MAX
        )
    })
}

pub(super) fn is_low_signal_query(query: &str) -> bool {
    let tokens = query
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .filter(|token| !token.is_empty())
        .count();
    let trimmed = query.trim();
    tokens <= 1 && trimmed.chars().count() <= 2
}

pub(super) fn path_role_prior(
    path: &str,
    language: &str,
    context_mode: Option<ContextMode>,
) -> f32 {
    let normalized = path.replace('\\', "/");
    let is_code_source = matches!(
        language,
        "rust" | "python" | "go" | "java" | "javascript" | "typescript" | "tsx" | "jsx"
    );
    let is_src_path =
        normalized == "src" || normalized.starts_with("src/") || normalized.contains("/src/");
    let is_test_path = normalized.starts_with("tests/")
        || normalized.contains("/tests/")
        || normalized.contains("/test/")
        || normalized.contains("_tests/")
        || normalized.contains("/main_tests/")
        || normalized.contains("/rpc_tools_tests/")
        || normalized.ends_with("_test.rs")
        || normalized.ends_with("_tests.rs");
    let is_markdown = normalized.ends_with(".md") || normalized.ends_with(".mdx");
    let is_hidden_planning =
        normalized.starts_with(".codex-planning/") || normalized.contains("/.codex-planning/");
    let is_low_priority = is_low_priority_path(&normalized);
    let is_manifest_or_schema = normalized == "Cargo.toml"
        || normalized == "Cargo.lock"
        || normalized.starts_with("schemas/")
        || normalized.contains("/schemas/")
        || normalized.ends_with(".json")
        || normalized.ends_with(".toml");

    let mut prior = 0.0_f32;
    match context_mode.unwrap_or(ContextMode::Code) {
        ContextMode::Code => {
            if is_code_source && is_src_path {
                prior += 0.020;
            }
            if is_test_path {
                prior -= 0.016;
            }
            if is_markdown {
                prior -= 0.026;
            }
            if is_manifest_or_schema {
                prior -= 0.018;
            }
        }
        ContextMode::Design => {
            if is_markdown {
                prior += 0.022;
            }
            if is_manifest_or_schema {
                prior += 0.016;
            }
            if is_code_source && is_src_path {
                prior += 0.004;
            }
            if is_test_path {
                prior -= 0.012;
            }
        }
        ContextMode::Bugfix => {
            if is_code_source && is_src_path {
                prior += 0.014;
            }
            if is_test_path {
                prior += 0.012;
            }
            if is_markdown {
                prior -= 0.012;
            }
            if is_manifest_or_schema {
                prior -= 0.004;
            }
        }
    }
    if is_hidden_planning {
        prior -= 0.035;
    }
    if is_low_priority {
        prior -= 0.050;
    }

    prior.clamp(-0.090, 0.030)
}
