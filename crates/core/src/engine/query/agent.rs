use anyhow::Result;

use crate::model::{
    AgentBootstrap, AgentBootstrapIncludeOptions, AgentIntentMode, PrivacyMode, SemanticFailMode,
};

use super::super::Engine;
use super::agent_bootstrap::{AgentBootstrapRequest, build_agent_bootstrap};

impl Engine {
    #[expect(
        clippy::too_many_arguments,
        reason = "public compatibility for MCP callers"
    )]
    pub fn agent_bootstrap_with_auto_index_and_options(
        &self,
        query: Option<&str>,
        limit: usize,
        semantic: bool,
        semantic_fail_mode: SemanticFailMode,
        privacy_mode: PrivacyMode,
        max_chars: usize,
        max_tokens: usize,
        auto_index: bool,
        agent_intent_mode: Option<AgentIntentMode>,
        include: AgentBootstrapIncludeOptions,
    ) -> Result<AgentBootstrap> {
        build_agent_bootstrap(
            self,
            AgentBootstrapRequest {
                query,
                limit,
                semantic,
                semantic_fail_mode,
                privacy_mode,
                max_chars,
                max_tokens,
                auto_index,
                agent_intent_mode,
                include,
            },
        )
    }
}
