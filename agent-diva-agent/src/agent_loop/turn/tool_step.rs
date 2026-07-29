use agent_diva_core::planning::model::PlanPhase;
use agent_diva_core::planning::policy::{allows_for_phase, ToolCapability};
use agent_diva_tooling::ToolRegistry;
use serde_json::Value;

use crate::mask::ToolPolicy;
use crate::planning::builtin_tool_capability;

/// Policy context required immediately before the single tool execution seam.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolStepPolicy {
    pub phase: Option<PlanPhase>,
    pub cancelled: bool,
    pub plan_guard_active: bool,
    pub persisted_plan_present: bool,
    pub reviewer_read_only: bool,
}

impl ToolStepPolicy {
    pub(crate) fn may_enter_executor(&self) -> bool {
        !self.cancelled
    }

    pub(crate) fn denial_reason(&self, tool_name: &str) -> Option<String> {
        if self.reviewer_read_only && !ToolPolicy::is_read_only_tool(tool_name) {
            return Some(format!(
                "Error: tool '{tool_name}' is disabled in reviewer read-only mode"
            ));
        }

        let capability = builtin_tool_capability(tool_name);
        let denied_by_phase = self
            .phase
            .as_ref()
            .is_some_and(|phase| !allows_for_phase(phase, capability));
        let denied_without_plan = self.plan_guard_active
            && !self.persisted_plan_present
            && !matches!(
                capability,
                ToolCapability::Inspect | ToolCapability::PlanningRecord
            );
        (denied_by_phase || denied_without_plan).then(|| {
            format!("Error: tool '{tool_name}' is denied by the active plan capability policy.")
        })
    }

    pub(crate) async fn execute(
        &self,
        tool_name: &str,
        arguments: &Value,
        context: ToolExecutionContext<'_>,
    ) -> ToolStepResult {
        if let Some(error) = self.denial_reason(tool_name) {
            return ToolStepResult::error(error);
        }

        let mut parameters = arguments.clone();
        if tool_name == "cron" {
            if let Some(object) = parameters.as_object_mut() {
                object.insert(
                    "context_channel".into(),
                    Value::String(context.channel.into()),
                );
                object.insert(
                    "context_chat_id".into(),
                    Value::String(context.chat_id.into()),
                );
                if context.cron_trigger {
                    object.insert("_in_cron_context".into(), Value::Bool(true));
                }
            }
        }
        if tool_name == "exec" {
            if let Some(object) = parameters.as_object_mut() {
                object.insert(
                    "_context_channel".into(),
                    Value::String(context.channel.into()),
                );
                object.insert(
                    "_context_chat_id".into(),
                    Value::String(context.chat_id.into()),
                );
                object.insert(
                    "_context_session_key".into(),
                    Value::String(context.session_key.into()),
                );
            }
        }
        if context.cron_trigger && tool_name == "cron" {
            return ToolStepResult::error(
                "Error: cron tool is disabled during cron-triggered execution to prevent recursive scheduling",
            );
        }

        match context.registry.execute(tool_name, parameters).await {
            Ok(output) => ToolStepResult {
                output,
                is_error: false,
            },
            Err(error) => ToolStepResult::error(format!("Error: {error}")),
        }
    }
}

pub(crate) struct ToolExecutionContext<'a> {
    pub registry: &'a ToolRegistry,
    pub channel: &'a str,
    pub chat_id: &'a str,
    pub session_key: &'a str,
    pub cron_trigger: bool,
}

pub(crate) struct ToolStepResult {
    pub output: String,
    pub is_error: bool,
}

impl ToolStepResult {
    fn error(error: impl Into<String>) -> Self {
        Self {
            output: error.into(),
            is_error: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_denies_executor_entry() {
        assert!(!ToolStepPolicy {
            phase: Some(PlanPhase::Execute),
            cancelled: true,
            plan_guard_active: false,
            persisted_plan_present: true,
            reviewer_read_only: false,
        }
        .may_enter_executor());
        assert!(ToolStepPolicy {
            phase: None,
            cancelled: false,
            plan_guard_active: false,
            persisted_plan_present: false,
            reviewer_read_only: false,
        }
        .may_enter_executor());
    }

    #[test]
    fn policy_denies_mutation_before_executor_entry() {
        let policy = ToolStepPolicy {
            phase: Some(PlanPhase::Plan),
            cancelled: false,
            plan_guard_active: true,
            persisted_plan_present: false,
            reviewer_read_only: false,
        };
        assert!(policy.denial_reason("exec").is_some());
        assert!(policy.denial_reason("read_file").is_none());
    }
}
