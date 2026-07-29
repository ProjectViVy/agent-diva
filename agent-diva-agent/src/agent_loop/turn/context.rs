use agent_diva_providers::Message;

/// Provider-ready context owned by the context preparation stage.
#[derive(Clone, Debug)]
pub(crate) struct PreparedTurnContext {
    pub messages: Vec<Message>,
    pub turn_messages_start: usize,
}

impl PreparedTurnContext {
    pub(crate) fn new(messages: Vec<Message>) -> Self {
        let turn_messages_start = messages.len();
        Self {
            messages,
            turn_messages_start,
        }
    }
}
