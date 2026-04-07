use anyhow::{Context, Result};

use crate::model::{AgentIntentMode, ContextMode};
use crate::query_profile::{QueryProfile, derive_query_profile};
use crate::vector_rank::SemanticRerankOutcome;

#[derive(Debug, Clone, Copy)]
pub(super) struct FusionProfile {
    pub(super) lexical_weight: f32,
    pub(super) semantic_file_weight: f32,
    pub(super) semantic_chunk_weight: f32,
    pub(super) graph_weight: f32,
    pub(super) probe_factor: f32,
}

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

pub(super) fn derive_fusion_profile(
    query: &str,
    context_mode: Option<ContextMode>,
    agent_intent_mode: Option<AgentIntentMode>,
) -> FusionProfile {
    if let Some(mode) = agent_intent_mode {
        return match mode {
            AgentIntentMode::EntrypointMap => FusionProfile {
                lexical_weight: 0.46,
                semantic_file_weight: 0.18,
                semantic_chunk_weight: 0.16,
                graph_weight: 0.20,
                probe_factor: 1.05,
            },
            AgentIntentMode::TestMap => FusionProfile {
                lexical_weight: 0.32,
                semantic_file_weight: 0.20,
                semantic_chunk_weight: 0.22,
                graph_weight: 0.26,
                probe_factor: 1.10,
            },
            AgentIntentMode::ReviewPrep => FusionProfile {
                lexical_weight: 0.28,
                semantic_file_weight: 0.24,
                semantic_chunk_weight: 0.24,
                graph_weight: 0.24,
                probe_factor: 1.20,
            },
            AgentIntentMode::ApiContractMap => FusionProfile {
                lexical_weight: 0.38,
                semantic_file_weight: 0.20,
                semantic_chunk_weight: 0.18,
                graph_weight: 0.24,
                probe_factor: 1.10,
            },
            AgentIntentMode::RuntimeSurface => FusionProfile {
                lexical_weight: 0.30,
                semantic_file_weight: 0.22,
                semantic_chunk_weight: 0.20,
                graph_weight: 0.28,
                probe_factor: 1.20,
            },
            AgentIntentMode::RefactorSurface => FusionProfile {
                lexical_weight: 0.26,
                semantic_file_weight: 0.24,
                semantic_chunk_weight: 0.22,
                graph_weight: 0.28,
                probe_factor: 1.22,
            },
        };
    }

    if let Some(mode) = context_mode {
        return match mode {
            ContextMode::Code => FusionProfile {
                lexical_weight: 0.52,
                semantic_file_weight: 0.18,
                semantic_chunk_weight: 0.18,
                graph_weight: 0.12,
                probe_factor: 0.95,
            },
            ContextMode::Design => FusionProfile {
                lexical_weight: 0.24,
                semantic_file_weight: 0.24,
                semantic_chunk_weight: 0.32,
                graph_weight: 0.20,
                probe_factor: 1.30,
            },
            ContextMode::Bugfix => FusionProfile {
                lexical_weight: 0.34,
                semantic_file_weight: 0.22,
                semantic_chunk_weight: 0.28,
                graph_weight: 0.16,
                probe_factor: 1.15,
            },
        };
    }

    match derive_query_profile(query) {
        QueryProfile::Precise => FusionProfile {
            lexical_weight: 0.60,
            semantic_file_weight: 0.22,
            semantic_chunk_weight: 0.10,
            graph_weight: 0.04,
            probe_factor: 0.90,
        },
        QueryProfile::Balanced => FusionProfile {
            lexical_weight: 0.40,
            semantic_file_weight: 0.26,
            semantic_chunk_weight: 0.20,
            graph_weight: 0.14,
            probe_factor: 1.00,
        },
        QueryProfile::Exploratory => FusionProfile {
            lexical_weight: 0.28,
            semantic_file_weight: 0.24,
            semantic_chunk_weight: 0.30,
            graph_weight: 0.18,
            probe_factor: 1.25,
        },
        QueryProfile::Bugfix => FusionProfile {
            lexical_weight: 0.34,
            semantic_file_weight: 0.22,
            semantic_chunk_weight: 0.28,
            graph_weight: 0.16,
            probe_factor: 1.15,
        },
    }
}

pub(super) fn seed_fusion_profile(profile: FusionProfile) -> FusionProfile {
    FusionProfile {
        graph_weight: 0.0,
        ..profile
    }
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

    prior.clamp(-0.060, 0.030)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_profile(
        profile: FusionProfile,
        lexical_weight: f32,
        semantic_file_weight: f32,
        semantic_chunk_weight: f32,
        graph_weight: f32,
        probe_factor: f32,
    ) {
        assert!((profile.lexical_weight - lexical_weight).abs() < f32::EPSILON);
        assert!((profile.semantic_file_weight - semantic_file_weight).abs() < f32::EPSILON);
        assert!((profile.semantic_chunk_weight - semantic_chunk_weight).abs() < f32::EPSILON);
        assert!((profile.graph_weight - graph_weight).abs() < f32::EPSILON);
        assert!((profile.probe_factor - probe_factor).abs() < f32::EPSILON);
    }

    #[test]
    fn explicit_agent_modes_use_first_class_fusion_profiles() {
        assert_profile(
            derive_fusion_profile("", None, Some(AgentIntentMode::EntrypointMap)),
            0.46,
            0.18,
            0.16,
            0.20,
            1.05,
        );
        assert_profile(
            derive_fusion_profile("", None, Some(AgentIntentMode::TestMap)),
            0.32,
            0.20,
            0.22,
            0.26,
            1.10,
        );
        assert_profile(
            derive_fusion_profile("", None, Some(AgentIntentMode::ReviewPrep)),
            0.28,
            0.24,
            0.24,
            0.24,
            1.20,
        );
        assert_profile(
            derive_fusion_profile("", None, Some(AgentIntentMode::ApiContractMap)),
            0.38,
            0.20,
            0.18,
            0.24,
            1.10,
        );
        assert_profile(
            derive_fusion_profile("", None, Some(AgentIntentMode::RuntimeSurface)),
            0.30,
            0.22,
            0.20,
            0.28,
            1.20,
        );
        assert_profile(
            derive_fusion_profile("", None, Some(AgentIntentMode::RefactorSurface)),
            0.26,
            0.24,
            0.22,
            0.28,
            1.22,
        );
    }

    #[test]
    fn seed_fusion_profile_keeps_weights_but_drops_graph_component() {
        let profile = derive_fusion_profile("", None, Some(AgentIntentMode::RuntimeSurface));
        let seeded = seed_fusion_profile(profile);
        assert_eq!(seeded.graph_weight, 0.0);
        assert_eq!(seeded.lexical_weight, profile.lexical_weight);
        assert_eq!(seeded.semantic_file_weight, profile.semantic_file_weight);
        assert_eq!(seeded.semantic_chunk_weight, profile.semantic_chunk_weight);
        assert_eq!(seeded.probe_factor, profile.probe_factor);
    }
}
