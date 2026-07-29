use agent_diva_core::session::TokenUsage;

/// Narrow input boundary for response/session finalization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FinalizationInput {
    pub content: String,
    pub reasoning: Option<String>,
    pub usage: Option<TokenUsage>,
}
