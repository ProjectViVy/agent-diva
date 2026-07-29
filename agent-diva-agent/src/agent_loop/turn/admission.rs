use agent_diva_core::bus::InboundMessage;

/// Side-effect-free result of inbound turn classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TurnAdmission {
    pub session_key: String,
    pub plan_mode: bool,
    pub scheduled: bool,
}

impl TurnAdmission {
    pub(crate) fn classify(message: &InboundMessage) -> Self {
        Self {
            session_key: format!("{}:{}", message.channel, message.chat_id),
            plan_mode: message
                .metadata
                .get("exec_mode")
                .and_then(|value| value.as_str())
                .is_some_and(|mode| mode.eq_ignore_ascii_case("plan")),
            scheduled: message.sender_id == "cron" || message.metadata.contains_key("cron_job_id"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_plan_and_scheduled_turns_without_side_effects() {
        let plan =
            InboundMessage::new("gui", "user", "chat", "plan").with_metadata("exec_mode", "PLAN");
        assert_eq!(
            TurnAdmission::classify(&plan),
            TurnAdmission {
                session_key: "gui:chat".into(),
                plan_mode: true,
                scheduled: false,
            }
        );

        let scheduled = InboundMessage::new("gui", "cron", "chat", "tick");
        assert!(TurnAdmission::classify(&scheduled).scheduled);
    }
}
