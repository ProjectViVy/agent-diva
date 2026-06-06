use crate::context_budget::{
    compact_messages_to_budget, provider_error_indicates_context_overflow, CompactionMode,
    ContextBudgetPolicy, ContextBudgetReport,
};
use agent_diva_providers::{Message, ProviderError};

pub(crate) struct PreparedRequest {
    pub messages: Vec<Message>,
    pub report: ContextBudgetReport,
}

pub(crate) fn prepare_budgeted_messages(
    messages: &[Message],
    tool_defs: &[serde_json::Value],
    policy: &ContextBudgetPolicy,
    mode: CompactionMode,
) -> PreparedRequest {
    let (messages, report) = compact_messages_to_budget(messages, tool_defs, policy, mode);
    PreparedRequest { messages, report }
}

pub(crate) fn should_retry_context_overflow(
    policy: &ContextBudgetPolicy,
    error: &ProviderError,
    already_retried: bool,
) -> bool {
    policy.overflow_retry_enabled
        && !already_retried
        && provider_error_indicates_context_overflow(error)
}
