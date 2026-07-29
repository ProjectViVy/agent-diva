use agent_diva_core::session::TokenUsage;

/// Data handed from model iteration to finalization.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct IterationOutcome {
    pub content: Option<String>,
    pub reasoning: Option<String>,
    pub token_usage: Option<TokenUsage>,
    pub stopped_for_plan_approval: bool,
}
