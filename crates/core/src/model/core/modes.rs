use serde::{Deserialize, Serialize};

use super::parse;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SemanticFailMode {
    #[default]
    FailOpen,
    FailClosed,
}

impl SemanticFailMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FailOpen => "fail_open",
            Self::FailClosed => "fail_closed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        parse::semantic_fail_mode(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyMode {
    #[default]
    Off,
    Mask,
    Hash,
}

impl PrivacyMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Mask => "mask",
            Self::Hash => "hash",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        parse::privacy_mode(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContextMode {
    #[default]
    Code,
    Design,
    Bugfix,
}

impl ContextMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Code => "code",
            Self::Design => "design",
            Self::Bugfix => "bugfix",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        parse::context_mode(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentIntentMode {
    #[default]
    EntrypointMap,
    TestMap,
    ReviewPrep,
    ApiContractMap,
    RuntimeSurface,
    RefactorSurface,
}

impl AgentIntentMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EntrypointMap => "entrypoint_map",
            Self::TestMap => "test_map",
            Self::ReviewPrep => "review_prep",
            Self::ApiContractMap => "api_contract_map",
            Self::RuntimeSurface => "runtime_surface",
            Self::RefactorSurface => "refactor_surface",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        parse::agent_intent_mode(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapProfile {
    #[default]
    Fast,
    InvestigationSummary,
    Report,
    Full,
}

impl BootstrapProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::InvestigationSummary => "investigation_summary",
            Self::Report => "report",
            Self::Full => "full",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        parse::bootstrap_profile(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModeResolutionSource {
    Explicit,
    Inferred,
    #[default]
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DegradationReason {
    SemanticFailOpen,
    SemanticLowSignalSkip,
    ChunkPreviewFallback,
    BudgetTruncated,
    ProfileLimited,
    UnsupportedSourcesPresent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalBasis {
    Indexed,
    PreviewFallback,
    GraphDerived,
    Heuristic,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalFreshness {
    IndexSnapshot,
    LiveRead,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalStrength {
    Strong,
    Moderate,
    Weak,
    FallbackOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RolloutPhase {
    Shadow,
    Canary5,
    Canary25,
    #[default]
    Full100,
}

impl RolloutPhase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shadow => "shadow",
            Self::Canary5 => "canary_5",
            Self::Canary25 => "canary_25",
            Self::Full100 => "full_100",
        }
    }

    pub const fn sample_percent(self) -> u8 {
        match self {
            Self::Shadow => 0,
            Self::Canary5 => 5,
            Self::Canary25 => 25,
            Self::Full100 => 100,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        parse::rollout_phase(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MigrationMode {
    #[default]
    Auto,
    Off,
}

impl MigrationMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Off => "off",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        parse::migration_mode(value)
    }
}
