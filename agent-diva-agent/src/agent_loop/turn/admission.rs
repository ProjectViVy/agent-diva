use agent_diva_core::channel::{
    ChannelEnvelopeV1, ChannelOrigin, ChannelPayloadV1, OwnerTurnIntent,
};
use agent_diva_core::planning::model::PlanPhase;
use agent_diva_core::planning::store::PlanningStore;
use agent_diva_core::planning::{ExecutionSession, ExecutionSessionStatus};
use agent_diva_tools::BackgroundTaskContext;
use tracing::info;

use super::super::AgentLoop;
use super::policy::TurnSnapshot;
use crate::mask::MaskFile;

const ADMISSION_CIRCUIT_TRIPPED: &str =
    "model/provider rejection storm tripped the circuit breaker; refusing new turn admission";
const ADMISSION_TURN_RATE_EXCEEDED: &str =
    "max_actions_per_hour exhausted; refusing new turn admission";

/// Side-effect-free result of inbound turn classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TurnMode {
    Agent,
    Plan,
    Ask,
}

impl TurnMode {
    fn from_envelope(envelope: &ChannelEnvelopeV1) -> Self {
        let ChannelPayloadV1::Message {
            context: Some(context),
            ..
        } = &envelope.payload
        else {
            return Self::Agent;
        };
        match context.intent {
            OwnerTurnIntent::Agent => Self::Agent,
            OwnerTurnIntent::Plan => Self::Plan,
            OwnerTurnIntent::Ask => Self::Ask,
        }
    }

    pub(crate) fn is_plan(self) -> bool {
        self == Self::Plan
    }

    pub(crate) fn is_read_only(self) -> bool {
        self == Self::Ask
    }

    fn allows_execution(self) -> bool {
        self == Self::Agent
    }
}

fn validate_execution_continuation(mode: TurnMode, requested: bool) -> anyhow::Result<()> {
    if requested && !mode.allows_execution() {
        anyhow::bail!("execution continuation is denied outside agent mode");
    }
    Ok(())
}

/// Side-effect-free result of inbound turn classification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TurnAdmission {
    pub session_key: String,
    pub mode: TurnMode,
    pub scheduled: bool,
}

impl TurnAdmission {
    pub(crate) fn classify(envelope: &ChannelEnvelopeV1) -> Self {
        Self {
            session_key: envelope.correlation.session_key.clone(),
            mode: TurnMode::from_envelope(envelope),
            scheduled: matches!(envelope.origin, ChannelOrigin::Runtime),
        }
    }
}

pub(crate) struct AdmittedTurn {
    pub active_mask: Option<MaskFile>,
    pub model: String,
    pub scheduled: bool,
    pub mode: TurnMode,
    pub execution_continuation: bool,
    pub session_key: String,
    pub active_execution: Option<ExecutionSession>,
    pub active_execution_id: Option<String>,
    pub snapshot: TurnSnapshot,
    pub plan_guard_active: bool,
    pub execution_plan_markdown: Option<String>,
    pub background_task_context: BackgroundTaskContext,
}

impl AgentLoop {
    /// Reject admission when the rejection circuit is tripped or the
    /// per-process turn rate limit (`max_actions_per_hour`) is exhausted.
    /// This is the offline-high-risk "reject" branch of S1c; the day/hour
    /// "queue" branch is deferred as a pending product decision.
    fn enforce_turn_admission(&self, session_key: &str) -> anyhow::Result<()> {
        if self.rejection_circuit.is_triggered() {
            anyhow::bail!(ADMISSION_CIRCUIT_TRIPPED);
        }
        if !self.turn_rate_limiter.try_record(self.max_actions_per_hour) {
            tracing::warn!(
                session_id = %session_key,
                max_actions_per_hour = self.max_actions_per_hour,
                "max_actions_per_hour exhausted; refusing new turn admission"
            );
            anyhow::bail!(ADMISSION_TURN_RATE_EXCEEDED);
        }
        Ok(())
    }

    pub(crate) async fn admit_turn(
        &mut self,
        envelope: &ChannelEnvelopeV1,
        trace_id: &str,
    ) -> Result<AdmittedTurn, Box<dyn std::error::Error + Send + Sync>> {
        let active_mask = self.load_active_mask();
        let model = self.effective_model_for_turn(active_mask.as_ref());
        let message_content = envelope
            .rendered_message_text()
            .ok_or_else(|| "typed channel turn requires a message payload".to_string())?;
        let preview = if message_content.chars().count() > 80 {
            format!(
                "{}...",
                message_content.chars().take(80).collect::<String>()
            )
        } else {
            message_content.clone()
        };
        info!(
            "Processing message from {}:{}: {} (model: {})",
            envelope.address.channel,
            envelope.address.sender_id.as_deref().unwrap_or("unknown"),
            preview,
            model
        );

        let admission = TurnAdmission::classify(envelope);
        self.enforce_turn_admission(&admission.session_key)?;
        let plan_mode = admission.mode.is_plan();
        let owner_execution_context = match &envelope.payload {
            ChannelPayloadV1::Message { context, .. }
                if matches!(envelope.origin, ChannelOrigin::OwnerFrontend) =>
            {
                context
                    .as_ref()
                    .and_then(|context| context.execution.as_ref())
            }
            _ => None,
        };
        let execution_continuation = owner_execution_context.is_some();
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
            Some(planning) if admission.mode.allows_execution() => {
                planning
                    .registry
                    .active_execution_for_session(&admission.session_key)
                    .await
            }
            _ => None,
        };
        if let Some(context) = owner_execution_context {
            validate_execution_continuation(admission.mode, execution_continuation)?;
            let execution = active_execution
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("no unique active execution for continuation"))?;
            let conflict = context.plan_id != execution.report_id.0
                || context.revision != execution.revision
                || context
                    .execution_id
                    .as_deref()
                    .is_some_and(|value| value != execution.id);
            if conflict {
                return Err(anyhow::anyhow!("approved execution continuation conflict").into());
            }
        }

        let active_execution_id = active_execution
            .as_ref()
            .map(|execution| execution.id.clone());
        let snapshot = TurnSnapshot::capture(
            admission.session_key.clone(),
            model.clone(),
            plan_mode,
            active_plan.as_ref(),
            admission.mode.is_read_only()
                || active_mask
                    .as_ref()
                    .is_some_and(crate::mask::ToolPolicy::is_read_only_mode),
            admission.scheduled,
            trace_id.to_string(),
        );
        let plan_guard_active = snapshot.plan_guard_active();
        let execution_plan_markdown = if admission.mode.allows_execution() {
            if let (Some(planning), Some(execution)) =
                (&self.tool_config.planning, active_execution.as_ref())
            {
                planning.registry.execution_markdown(&execution.id).await
            } else {
                None
            }
        } else {
            None
        };
        let background_task_context = BackgroundTaskContext {
            channel: Some(envelope.address.channel.clone()),
            chat_id: Some(envelope.address.chat_id.clone()),
            session_key: Some(admission.session_key.clone()),
            trace_id: Some(trace_id.to_string()),
            parent_run_id: None,
            token_budget_limit: self.session_token_budget_limit,
            mask_config: active_mask.as_ref().map(|mask| mask.frontmatter.clone()),
        };
        // Deferred tool activation is task-local. A new user turn starts with
        // a clean provider surface; same-turn runtime rebuilds reuse it.
        self.clear_active_deferred_tools(&admission.session_key);
        self.active_tool_surface.approval_policy_override =
            Self::approval_policy_for_envelope(envelope);
        self.rebuild_tools_for_turn(
            active_mask.as_ref(),
            snapshot.policy_phase.clone(),
            active_execution_id.clone(),
            Some(admission.session_key.clone()),
            Some(background_task_context.clone()),
        );

        Ok(AdmittedTurn {
            active_mask,
            model,
            scheduled: admission.scheduled,
            mode: admission.mode,
            execution_continuation,
            session_key: admission.session_key,
            active_execution,
            active_execution_id,
            snapshot,
            plan_guard_active,
            execution_plan_markdown,
            background_task_context,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::channel::{
        ChannelAddress, ChannelDirection, ContentPart, Correlation, OwnerTurnContextV1,
    };

    fn owner_envelope(intent: OwnerTurnIntent) -> ChannelEnvelopeV1 {
        ChannelEnvelopeV1::new(
            ChannelDirection::Ingress,
            ChannelAddress::new("gui", "chat"),
            Correlation::new("profile/session"),
            ChannelOrigin::OwnerFrontend,
            ChannelPayloadV1::Message {
                parts: vec![ContentPart::Text {
                    text: "turn".to_string(),
                }],
                subject: None,
                locale: None,
                context: Some(OwnerTurnContextV1 {
                    intent,
                    approval_policy: None,
                    execution: None,
                }),
            },
        )
    }

    fn runtime_envelope() -> ChannelEnvelopeV1 {
        ChannelEnvelopeV1::new(
            ChannelDirection::Ingress,
            ChannelAddress::new("cron", "job"),
            Correlation::new("runtime/cron/job"),
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Message {
                parts: vec![ContentPart::Text {
                    text: "tick".to_string(),
                }],
                subject: None,
                locale: None,
                context: None,
            },
        )
    }

    #[test]
    fn classifies_typed_intents_and_runtime_turns_without_side_effects() {
        let plan = owner_envelope(OwnerTurnIntent::Plan);
        assert_eq!(
            TurnAdmission::classify(&plan),
            TurnAdmission {
                session_key: "profile/session".into(),
                mode: TurnMode::Plan,
                scheduled: false,
            }
        );

        let ask = owner_envelope(OwnerTurnIntent::Ask);
        assert_eq!(TurnAdmission::classify(&ask).mode, TurnMode::Ask);
        assert!(!TurnMode::Ask.allows_execution());
        assert!(validate_execution_continuation(TurnMode::Ask, true).is_err());
        assert!(validate_execution_continuation(TurnMode::Plan, true).is_err());
        assert!(validate_execution_continuation(TurnMode::Agent, true).is_ok());
        let scheduled = runtime_envelope();
        assert!(TurnAdmission::classify(&scheduled).scheduled);
    }
}
