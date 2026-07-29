use agent_diva_providers::Message;

use crate::context::ContextBuilder;

use super::prompt;

/// Provider-ready context owned by the context preparation stage.
#[derive(Clone, Debug)]
pub(crate) struct PreparedTurnContext {
    pub messages: Vec<Message>,
    pub turn_messages_start: usize,
}

impl PreparedTurnContext {
    pub(crate) fn prepare(
        mut messages: Vec<Message>,
        plan_guard_active: bool,
        approved_plan_markdown: Option<&str>,
        system_prompt_override: Option<String>,
        scheduled: bool,
        current_turn_message: Message,
    ) -> Self {
        if plan_guard_active {
            messages.insert(1, prompt::plan_mode().system());
        }
        if let Some(markdown) = approved_plan_markdown {
            messages.insert(1, prompt::approved_plan(markdown).system());
        }
        ContextBuilder::sanitize_messages_for_provider(&mut messages);
        let turn_messages_start = messages.len();

        if let (Some(system_prompt), Some(first)) = (system_prompt_override, messages.first_mut()) {
            *first = Message::system(system_prompt);
        }
        if scheduled {
            let current_message = messages.pop();
            messages.push(prompt::scheduled_turn().system());
            if let Some(current_message) = current_message {
                messages.push(current_message);
            }
        }
        if let Some(last) = messages.last_mut() {
            *last = current_turn_message;
        } else {
            messages.push(current_turn_message);
        }

        Self {
            messages,
            turn_messages_start,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freezes_the_provider_prefix_boundary() {
        let context = PreparedTurnContext::prepare(
            vec![Message::system("system"), Message::user("current")],
            false,
            None,
            None,
            false,
            Message::user("current"),
        );
        assert_eq!(context.turn_messages_start, 2);
        assert_eq!(context.messages.len(), 2);
    }

    #[test]
    fn scheduled_prompt_precedes_the_rich_current_turn_message() {
        let context = PreparedTurnContext::prepare(
            vec![Message::system("system"), Message::user("placeholder")],
            false,
            None,
            None,
            true,
            Message::user("current"),
        );
        assert_eq!(context.turn_messages_start, 2);
        assert_eq!(context.messages[1].role, "system");
        assert!(context.messages[1]
            .content
            .as_text()
            .is_some_and(|text| text.contains("scheduled cron job")));
        assert_eq!(context.messages[2].content.as_text(), Some("current"));
    }
}
