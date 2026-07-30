use agent_diva_providers::Message;

/// Stable identity for an LLM-facing runtime prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RuntimePromptId {
    PlanMode,
    AskMode,
    ApprovedPlan,
    ScheduledTurn,
    SessionTitleSystem,
    SessionTitleUser,
}

/// An auditable prompt contract. Callers log only `id` and `version`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RuntimePrompt {
    pub id: RuntimePromptId,
    pub version: u16,
    pub content: String,
}

impl RuntimePrompt {
    pub(crate) fn system(self) -> Message {
        Message::system(self.content)
    }

    pub(crate) fn user(self) -> Message {
        Message::user(self.content)
    }
}

pub(crate) fn plan_mode() -> RuntimePrompt {
    RuntimePrompt {
        id: RuntimePromptId::PlanMode,
        version: 2,
        content: r#"You are in Plan mode until the user leaves it. Explore with read-only tools only. Do not modify files, run mutating shell commands, call planning or TODO tools, or begin implementation.

When you have enough information for a decision-complete plan, end the turn with exactly one line-oriented XML block. The tags must be alone on their lines and must not be translated:
<proposed_plan>
# Short title
## Goal
...
## Scope
...
## Implementation Steps
...
## Risks and Assumptions
...
## Verification
...
</proposed_plan>

Put any preface outside the tags. Emit at most one <proposed_plan> block per turn. A revision must be a complete replacement. Do not ask whether to implement; the user uses the approval UI."#.to_string(),
    }
}

pub(crate) fn ask_mode() -> RuntimePrompt {
    RuntimePrompt {
        id: RuntimePromptId::AskMode,
        version: 1,
        content: "You are in Ask mode. Analyze and answer using read-only inspection only. Never execute shell commands, write, edit, delete, schedule, spawn, mutate plans or TODOs, or perform any other persistent side effect. Existing approvals and execution sessions do not authorize mutations in this turn.".to_string(),
    }
}

pub(crate) fn approved_plan(markdown: &str) -> RuntimePrompt {
    RuntimePrompt {
        id: RuntimePromptId::ApprovedPlan,
        version: 1,
        content: format!(
            "You are implementing the approved plan below. It remains authoritative throughout execution. Do not re-plan; execute it and report the results.\n\n{markdown}"
        ),
    }
}

pub(crate) fn scheduled_turn() -> RuntimePrompt {
    RuntimePrompt {
        id: RuntimePromptId::ScheduledTurn,
        version: 1,
        content: "This turn was triggered automatically by a scheduled cron job, not by real-time user input. Do not schedule new reminders or jobs unless prior task design explicitly requires it.".to_string(),
    }
}

pub(crate) fn session_title_system() -> RuntimePrompt {
    RuntimePrompt {
        id: RuntimePromptId::SessionTitleSystem,
        version: 1,
        content:
            "Write a short conversation title. Output only the title without quotes or explanation."
                .to_string(),
    }
}

pub(crate) fn session_title_user(user: &str, assistant: &str) -> RuntimePrompt {
    RuntimePrompt {
        id: RuntimePromptId::SessionTitleUser,
        version: 1,
        content: format!(
            "Generate a concise conversation title of at most 12 words.\n\nFirst user message:\n{user}\n\nFirst assistant message:\n{assistant}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_contracts_have_stable_identity_and_roles() {
        let plan = plan_mode();
        assert_eq!(plan.id, RuntimePromptId::PlanMode);
        assert_eq!(plan.version, 2);
        assert!(plan.content.contains("## Goal"));
        assert!(plan.content.contains("<proposed_plan>"));
        let ask = ask_mode();
        assert_eq!(ask.id, RuntimePromptId::AskMode);
        assert!(ask.content.contains("read-only"));

        let title = session_title_user("<user>", "<assistant>");
        assert!(title.content.contains("<user>"));
        assert!(title.content.contains("<assistant>"));
        assert_eq!(title.user().role, "user");
        assert_eq!(session_title_system().system().role, "system");
    }
}
