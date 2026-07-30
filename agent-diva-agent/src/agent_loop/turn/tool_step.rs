use agent_diva_core::bus::{AgentEvent, InboundMessage};
use agent_diva_core::planning::model::PlanPhase;
use agent_diva_core::planning::policy::{allows_for_phase, ToolCapability};
use agent_diva_core::soul::SoulStateStore;
use agent_diva_providers::{Message, ToolCallRequest};
use agent_diva_tooling::ToolRegistry;
use agent_diva_tools::BackgroundTaskContext;
use serde_json::Value;
use std::collections::HashSet;
use tokio::sync::mpsc;
use tracing::{info, trace, warn};

use crate::mask::{MaskFile, ToolPolicy};
use crate::planning::builtin_tool_capability;

use super::super::loop_turn::{changed_soul_file, truncate_for_tool_summary};
use super::super::{policy_phase_for, AgentLoop};
use super::policy::TurnSnapshot;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolRunSummary {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

pub(crate) struct ToolOrchestrationContext<'a> {
    pub message: &'a InboundMessage,
    pub event_tx: Option<&'a mpsc::UnboundedSender<AgentEvent>>,
    pub session_key: &'a str,
    pub trace_id: &'a str,
    pub iteration: usize,
    pub plan_mode: bool,
    pub read_only: bool,
    pub plan_guard_active: bool,
    pub active_mask: Option<&'a MaskFile>,
    pub active_execution_id: Option<String>,
    pub background_task_context: BackgroundTaskContext,
    pub scheduled: bool,
}

pub(crate) struct ToolOrchestrationResult {
    pub summary: ToolRunSummary,
    pub stop_after_tool_call: bool,
}

impl ToolStepResult {
    fn error(error: impl Into<String>) -> Self {
        Self {
            output: error.into(),
            is_error: true,
        }
    }
}

impl AgentLoop {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn orchestrate_tool_call(
        &mut self,
        tool_call: &ToolCallRequest,
        context: &ToolOrchestrationContext<'_>,
        turn_snapshot: &mut TurnSnapshot,
        messages: &mut Vec<Message>,
        soul_files_changed: &mut HashSet<String>,
    ) -> Result<Option<ToolOrchestrationResult>, Box<dyn std::error::Error>> {
        self.drain_runtime_control_commands().await;
        if self.is_session_cancelled(context.session_key) {
            self.emit_error_event(
                context.message,
                context.event_tx,
                "Generation stopped by user.",
            );
            return Ok(None);
        }

        trace!(
            trace_id = %context.trace_id,
            loop_index = context.iteration,
            step_name = "tool_invoked",
            tool_name = %tool_call.name,
            "Tool invoked"
        );
        let args_str = serde_json::to_string(&tool_call.arguments).unwrap_or_default();
        let preview = if args_str.chars().count() > 200 {
            format!("{}...", args_str.chars().take(200).collect::<String>())
        } else {
            args_str
        };
        info!("Tool call: {}({})", tool_call.name, preview);

        let planning_before = self.snapshot_active_plan_runtime(context.session_key).await;
        turn_snapshot.refresh_policy(planning_before.as_ref());
        let policy = ToolStepPolicy {
            phase: turn_snapshot.policy_phase.clone(),
            cancelled: self.is_session_cancelled(context.session_key),
            plan_guard_active: context.plan_guard_active,
            persisted_plan_present: planning_before.is_some(),
            reviewer_read_only: context.read_only
                || context
                    .active_mask
                    .is_some_and(ToolPolicy::is_read_only_mode),
        };
        if !policy.may_enter_executor() {
            self.emit_error_event(
                context.message,
                context.event_tx,
                "Generation stopped by user.",
            );
            return Ok(None);
        }

        let event = AgentEvent::ToolCallStarted {
            name: tool_call.name.clone(),
            args_preview: preview,
            call_id: tool_call.id.clone(),
        };
        if let Some(tx) = context.event_tx {
            let _ = tx.send(event.clone());
        }
        let _ = self.bus.publish_event(
            context.message.channel.clone(),
            context.message.chat_id.clone(),
            event,
        );

        let (result, is_error) = match serde_json::to_value(&tool_call.arguments) {
            Ok(arguments) => {
                let result = policy
                    .execute(
                        &tool_call.name,
                        &arguments,
                        ToolExecutionContext {
                            registry: &self.tools,
                            channel: &context.message.channel,
                            chat_id: &context.message.chat_id,
                            session_key: context.session_key,
                            cron_trigger: context.message.channel == "cron" || context.scheduled,
                        },
                    )
                    .await;
                (result.output, result.is_error)
            }
            Err(error) => {
                warn!(
                    "Failed to serialize arguments for tool '{}' (call_id: {}): {}",
                    tool_call.name, tool_call.id, error
                );
                (
                    format!(
                        "Error: failed to serialize arguments for tool '{}': {}",
                        tool_call.name, error
                    ),
                    true,
                )
            }
        };

        if self.notify_on_soul_change && !is_error {
            if let Some(changed_file) =
                changed_soul_file(&tool_call.name, &tool_call.arguments, &result)
            {
                if changed_file == "BOOTSTRAP.md" {
                    let _ = SoulStateStore::new(&self.workspace).mark_bootstrap_completed();
                }
                soul_files_changed.insert(changed_file.to_string());
            }
        }
        trace!(
            trace_id = %context.trace_id,
            loop_index = context.iteration,
            step_name = "tool_completed",
            tool_name = %tool_call.name,
            "Tool completed"
        );

        let planning_after = if is_error {
            None
        } else {
            self.snapshot_active_plan_runtime(context.session_key).await
        };
        if !is_error {
            self.emit_chat_plan_update(
                context.message,
                context.event_tx,
                &tool_call.name,
                &serde_json::to_value(&tool_call.arguments).unwrap_or_default(),
                false,
            )
            .await;
        }
        let event = AgentEvent::ToolCallFinished {
            name: tool_call.name.clone(),
            is_error,
            result: result.clone(),
            call_id: tool_call.id.clone(),
        };
        if let Some(tx) = context.event_tx {
            let _ = tx.send(event.clone());
        }
        let _ = self.bus.publish_event(
            context.message.channel.clone(),
            context.message.chat_id.clone(),
            event,
        );

        let mut stop_after_tool_call = false;
        if !is_error {
            self.emit_planning_runtime_events(
                context.message,
                context.event_tx,
                &tool_call.name,
                planning_before.clone(),
                planning_after.clone(),
            )
            .await;
            if planning_before
                .as_ref()
                .map(|plan| (plan.phase.clone(), plan.revision))
                != planning_after
                    .as_ref()
                    .map(|plan| (plan.phase.clone(), plan.revision))
            {
                self.rebuild_tools_for_turn(
                    context.active_mask,
                    policy_phase_for(planning_after.as_ref(), context.plan_mode),
                    context.active_execution_id.clone(),
                    Some(context.background_task_context.clone()),
                );
            }
            stop_after_tool_call =
                matches!(tool_call.name.as_str(), "plan_submit" | "plan_transition")
                    && planning_after
                        .as_ref()
                        .is_some_and(|plan| plan.phase == PlanPhase::AwaitingApproval);
        }

        let summary = ToolRunSummary {
            name: tool_call.name.clone(),
            ok: !is_error,
            detail: truncate_for_tool_summary(&result, 120),
        };
        self.context.add_tool_result(
            messages,
            tool_call.id.clone(),
            tool_call.name.clone(),
            result,
        );
        Ok(Some(ToolOrchestrationResult {
            summary,
            stop_after_tool_call,
        }))
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

    #[test]
    fn ask_read_only_policy_denies_every_mutating_tool_family() {
        let policy = ToolStepPolicy {
            phase: None,
            cancelled: false,
            plan_guard_active: false,
            persisted_plan_present: true,
            reviewer_read_only: true,
        };
        for tool in [
            "exec",
            "write_file",
            "edit_file",
            "delete_file",
            "plan_submit",
            "todo_update",
            "cron",
            "spawn",
        ] {
            assert!(
                policy.denial_reason(tool).is_some(),
                "{tool} must be denied"
            );
        }
        for tool in [
            "read_file",
            "list_dir",
            "read_attachment",
            "web_search",
            "web_fetch",
        ] {
            assert!(
                policy.denial_reason(tool).is_none(),
                "{tool} must remain available"
            );
        }
    }
}
