use anyhow::Result;

use crate::model::{AgentBootstrap, PrivacyMode, SemanticFailMode};

use super::super::Engine;

impl Engine {
    pub fn agent_bootstrap(
        &self,
        query: Option<&str>,
        limit: usize,
        semantic: bool,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<AgentBootstrap> {
        self.agent_bootstrap_with_mode(
            query,
            limit,
            semantic,
            SemanticFailMode::FailOpen,
            PrivacyMode::Off,
            max_chars,
            max_tokens,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "public compatibility for MCP callers"
    )]
    pub fn agent_bootstrap_with_mode(
        &self,
        query: Option<&str>,
        limit: usize,
        semantic: bool,
        semantic_fail_mode: SemanticFailMode,
        privacy_mode: PrivacyMode,
        max_chars: usize,
        max_tokens: usize,
    ) -> Result<AgentBootstrap> {
        self.agent_bootstrap_with_auto_index_and_mode(
            query,
            limit,
            semantic,
            semantic_fail_mode,
            privacy_mode,
            max_chars,
            max_tokens,
            true,
        )
    }

    pub fn agent_bootstrap_with_auto_index(
        &self,
        query: Option<&str>,
        limit: usize,
        semantic: bool,
        max_chars: usize,
        max_tokens: usize,
        auto_index: bool,
    ) -> Result<AgentBootstrap> {
        self.agent_bootstrap_with_auto_index_and_mode(
            query,
            limit,
            semantic,
            SemanticFailMode::FailOpen,
            PrivacyMode::Off,
            max_chars,
            max_tokens,
            auto_index,
        )
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "public compatibility for MCP callers"
    )]
    pub fn agent_bootstrap_with_auto_index_and_mode(
        &self,
        query: Option<&str>,
        limit: usize,
        semantic: bool,
        semantic_fail_mode: SemanticFailMode,
        privacy_mode: PrivacyMode,
        max_chars: usize,
        max_tokens: usize,
        auto_index: bool,
    ) -> Result<AgentBootstrap> {
        self.agent_bootstrap_with_auto_index_and_options(
            query,
            limit,
            semantic,
            semantic_fail_mode,
            privacy_mode,
            max_chars,
            max_tokens,
            auto_index,
            None,
            Default::default(),
        )
    }
}
