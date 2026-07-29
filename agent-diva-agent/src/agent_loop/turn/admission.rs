use agent_diva_core::bus::InboundMessage;
use agent_diva_core::planning::model::PlanPhase;
use agent_diva_core::planning::store::PlanningStore;
use agent_diva_core::planning::{ExecutionSession, ExecutionSessionStatus};
use agent_diva_tools::BackgroundTaskContext;
use tracing::info;

use super::super::AgentLoop;
use super::policy::TurnSnapshot;
use crate::mask::MaskFile;

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

pub(crate) struct AdmittedTurn {
    pub active_mask: Option<MaskFile>,
    pub model: String,
    pub scheduled: bool,
    pub plan_mode: bool,
    pub execution_start: bool,
    pub session_key: String,
    pub active_execution: Option<ExecutionSession>,
    pub active_execution_id: Option<String>,
    pub snapshot: TurnSnapshot,
    pub plan_guard_active: bool,
    pub approved_plan_markdown: Option<String>,
    pub background_task_context: BackgroundTaskContext,
}

impl AgentLoop {
    pub(crate) async fn admit_turn(
        &mut self,
        message: &InboundMessage,
        trace_id: &str,
    ) -> Result<AdmittedTurn, Box<dyn std::error::Error>> {
        let active_mask = self.load_active_mask();
        let model = self.effective_model_for_turn(active_mask.as_ref());
        self.subagent_manager
            .set_current_mask(active_mask.as_ref().map(|mask| mask.frontmatter.clone()))
            .await;

        let preview = if message.content.chars().count() > 80 {
            format!(
                "{}...",
                message.content.chars().take(80).collect::<String>()
            )
        } else {
            message.content.clone()
        };
        info!(
            "Processing message from {}:{}: {} (model: {})",
            message.channel, message.sender_id, preview, model
        );

        let admission = TurnAdmission::classify(message);
        let mut execution_start = message
            .metadata
            .get("execution_start")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let active_plan = self
            .snapshot_active_plan_runtime(&admission.session_key)
            .await;

        if let Some(planning) = &self.tool_config.planning {
            if planning
                .registry
                .active_execution_for_session(&admission.session_key)
                .await
                .is_none()
            {
                if let Ok(plan_id) = planning.store.get_active_plan().await {
                    if let Ok(plan) = planning.store.get_plan(&plan_id).await {
                        if plan.phase == PlanPhase::Execute {
                            if let Some(context) = planning
                                .store
                                .get_execution_context_for_session(&plan_id, &admission.session_key)
                                .await?
                            {
                                if let Some(markdown) = plan.strategy {
                                    planning
                                        .registry
                                        .restore_execution(
                                            &admission.session_key,
                                            ExecutionSession {
                                                id: context.execution_id,
                                                report_id: context.plan_id,
                                                revision: context.revision,
                                                context_policy: context.context_policy,
                                                status: ExecutionSessionStatus::Executing,
                                                compacted_context: context.compacted_context,
                                                boundary: context.boundary,
                                                initialization_status: context
                                                    .initialization_status,
                                                initialization_error: context.initialization_error,
                                                created_at: context.created_at,
                                                updated_at: context.updated_at,
                                            },
                                            markdown,
                                        )
                                        .await?;
                                }
                            }
                        }
                    }
                }
            }
        }

        let active_execution = match &self.tool_config.planning {
            Some(planning) if !admission.plan_mode => {
                planning
                    .registry
                    .active_execution_for_session(&admission.session_key)
                    .await
            }
            _ => None,
        };
        if execution_start {
            let execution = active_execution
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no unique active execution for continuation"))?;
            let conflict = message
                .metadata
                .get("plan_id")
                .and_then(|value| value.as_str())
                .is_some_and(|value| value != execution.report_id.0)
                || message
                    .metadata
                    .get("plan_revision")
                    .and_then(|value| value.as_i64())
                    .is_some_and(|value| value != execution.revision)
                || message
                    .metadata
                    .get("execution_id")
                    .and_then(|value| value.as_str())
                    .is_some_and(|value| value != execution.id);
            if conflict {
                return Err(anyhow::anyhow!("approved execution continuation conflict").into());
            }
        }
        if message
            .metadata
            .get("legacy_execution_start")
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
            && active_execution.is_some()
            && !admission.plan_mode
        {
            execution_start = execution_start
                || message
                    .metadata
                    .get("approved_plan_markdown")
                    .and_then(|value| value.as_str())
                    .is_some()
                || message.content.contains("Carry out the approved plan")
                || message.content.contains("开始执行已批准");
        }

        let active_execution_id = active_execution
            .as_ref()
            .map(|execution| execution.id.clone());
        let snapshot = TurnSnapshot::capture(
            admission.session_key.clone(),
            model.clone(),
            admission.plan_mode,
            active_plan.as_ref(),
            trace_id.to_string(),
        );
        let plan_guard_active = snapshot.plan_guard_active();
        let approved_plan_markdown = if let Some(markdown) = message
            .metadata
            .get("approved_plan_markdown")
            .and_then(|value| value.as_str())
        {
            Some(markdown.to_string())
        } else if let (Some(planning), Some(execution)) =
            (&self.tool_config.planning, active_execution.as_ref())
        {
            planning.registry.execution_markdown(&execution.id).await
        } else {
            None
        };
        let background_task_context = BackgroundTaskContext {
            channel: Some(message.channel.clone()),
            chat_id: Some(message.chat_id.clone()),
            session_key: Some(admission.session_key.clone()),
            trace_id: Some(trace_id.to_string()),
            parent_run_id: message
                .metadata
                .get("run_id")
                .or_else(|| message.metadata.get("parent_run_id"))
                .and_then(|value| value.as_str())
                .map(str::to_string),
            token_budget_limit: self.session_token_budget_limit,
        };
        self.rebuild_tools_for_turn(
            active_mask.as_ref(),
            snapshot.policy_phase.clone(),
            active_execution_id.clone(),
            Some(background_task_context.clone()),
        );

        Ok(AdmittedTurn {
            active_mask,
            model,
            scheduled: admission.scheduled,
            plan_mode: admission.plan_mode,
            execution_start,
            session_key: admission.session_key,
            active_execution,
            active_execution_id,
            snapshot,
            plan_guard_active,
            approved_plan_markdown,
            background_task_context,
        })
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
