use agent_diva_core::audit::{self, AuditEvent};
use agent_diva_core::bus::InboundMessage;
use agent_diva_core::memory::{PrefetchRequest, PrefetchStatus};
use agent_diva_core::planning::{
    ExecutionContextBoundary, ExecutionContextPolicy, ExecutionInitializationStatus,
    ExecutionSession,
};
use agent_diva_core::security::{check_security, SecurityContext, SecurityDecision};
use agent_diva_core::session::{align_chat_history, CanonicalCheckpoint, CheckpointTrigger};
use agent_diva_providers::{supports_vision_model, DynamicContextTransport, Message};
use tracing::{error, info, trace, warn};

use crate::compaction::{CheckpointCompactor, CheckpointSnapshot};
use crate::context::ContextBuilder;
use crate::context_assembly::{
    estimate_messages, select_dynamic_sections, serialize_dynamic_sections, AssemblyDecision,
    AssemblyDecisionReason, BudgetLayer, ContextAssemblyError, ContextAssemblyReport,
    ContextBudgetPlan, ContextBudgetRegion, ContextSection, PromptSection, StablePrefixSnapshot,
};
use crate::context_budget::check_context_budget;
use crate::mask::MaskFile;

use super::super::loop_turn::{
    build_current_turn_message, derive_prefetch_intent, persist_execution_context,
    ProcessedInboundMedia,
};
use super::super::AgentLoop;
use super::prompt;

/// Provider-ready context owned by the context preparation stage.
#[derive(Clone, Debug)]
pub(crate) struct PreparedTurnContext {
    pub messages: Vec<Message>,
    pub turn_messages_start: usize,
    pub assembly_report: ContextAssemblyReport,
}

pub(crate) struct RuntimeTurnContext {
    pub message_content: String,
    pub current_turn_message: Message,
    pub messages: Vec<Message>,
    pub turn_messages_start: usize,
    pub dynamic_sections: Vec<PromptSection>,
    pub stable_prefix: StablePrefixSnapshot,
    pub assembly_report: ContextAssemblyReport,
}

impl PreparedTurnContext {
    #[cfg(test)]
    pub(crate) fn prepare(
        messages: Vec<Message>,
        dynamic_sections: &[PromptSection],
        transport: DynamicContextTransport,
        current_turn_message: Message,
    ) -> Result<Self, ContextAssemblyError> {
        Self::prepare_budgeted(
            messages,
            dynamic_sections,
            transport,
            current_turn_message,
            &crate::context_budget::BudgetConfig::default(),
        )
    }

    pub(crate) fn prepare_budgeted(
        mut messages: Vec<Message>,
        dynamic_sections: &[PromptSection],
        transport: DynamicContextTransport,
        current_turn_message: Message,
        budget: &crate::context_budget::BudgetConfig,
    ) -> Result<Self, ContextAssemblyError> {
        let base_tokens = estimate_messages(&messages).saturating_add(estimate_messages(
            std::slice::from_ref(&current_turn_message),
        ));
        let plan = ContextBudgetPlan::from_config(budget);
        let (selected_sections, assembly_report) =
            select_dynamic_sections(dynamic_sections, base_tokens, &plan);
        if let Some(dynamic) = serialize_dynamic_sections(&selected_sections, transport)? {
            messages.push(dynamic);
        }
        messages.push(current_turn_message);
        ContextBuilder::sanitize_messages_for_provider(&mut messages);
        let turn_messages_start = messages.len();

        Ok(Self {
            messages,
            turn_messages_start,
            assembly_report,
        })
    }
}

impl AgentLoop {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn prepare_runtime_context(
        &mut self,
        message: &InboundMessage,
        model: &str,
        execution_start: bool,
        session_key: &str,
        active_execution: &mut Option<ExecutionSession>,
        plan_guard_active: bool,
        approved_plan_markdown: Option<&str>,
        read_only: bool,
        active_mask: Option<&MaskFile>,
        scheduled: bool,
        trace_id: &str,
    ) -> Result<RuntimeTurnContext, Box<dyn std::error::Error>> {
        // A pending reactive update belongs to one turn only.  If the prior
        // turn was cancelled before finalize, discard it before starting a
        // fresh snapshot.
        self.pending_checkpoint_updates.remove(session_key);
        let processed_media = if message.media.is_empty() {
            ProcessedInboundMedia::default()
        } else {
            self.load_attachment_contents(&message.media).await?
        };
        let message_content = if processed_media.prompt_text.is_empty() {
            message.content.clone()
        } else {
            format!(
                "{}\n\n[Attachments]\n{}\n[/Attachments]",
                message.content, processed_media.prompt_text
            )
        };
        let security_context = SecurityContext {
            source_type: "channel".to_string(),
            workspace_id: Some(self.workspace.display().to_string()),
            run_id: None,
            channel_id: Some(message.channel.clone()),
            tool_name: None,
        };
        let message_content = match check_security(&message_content, &security_context) {
            SecurityDecision::Allow => message_content,
            SecurityDecision::Sanitize { redacted, .. } => redacted,
            SecurityDecision::Block { reason, .. }
            | SecurityDecision::Quarantine { reason, .. } => {
                audit::emit(AuditEvent::ChannelMessageBlocked {
                    channel_id: message.channel.clone(),
                    reason: reason.clone(),
                });
                return Err(
                    anyhow::anyhow!("Security policy blocked inbound message: {}", reason).into(),
                );
            }
        };
        if !processed_media.image_parts.is_empty() && !supports_vision_model(model) {
            return Err(anyhow::anyhow!(
                "Current model `{}` does not support vision input. Switch to a vision-capable model such as `gpt-4o` or `gpt-4.1`.",
                model
            )
            .into());
        }
        let current_turn_message =
            build_current_turn_message(&message_content, &processed_media.image_parts);

        if execution_start {
            if let (Some(planning), Some(execution)) =
                (&self.tool_config.planning, active_execution.as_ref())
            {
                if matches!(
                    execution.initialization_status,
                    ExecutionInitializationStatus::Pending | ExecutionInitializationStatus::Blocked
                ) {
                    let boundary_count = self
                        .sessions
                        .get(session_key)
                        .map_or(0, |session| session.messages.len());
                    let boundary = ExecutionContextBoundary {
                        message_count: boundary_count,
                        initialized_at: chrono::Utc::now(),
                    };
                    let compacted = match execution.context_policy {
                        ExecutionContextPolicy::Compact => {
                            let mut snapshot =
                                self.sessions.get(session_key).cloned().ok_or_else(|| {
                                    anyhow::anyhow!("execution transcript unavailable")
                                })?;
                            snapshot.messages.truncate(boundary_count);
                            let mut config = self.tool_config.budget.clone();
                            config.keep_recent_count = 0;
                            match CheckpointCompactor::compact_snapshot(
                                CheckpointSnapshot::new(None, snapshot.messages, Vec::new()),
                                &config,
                                self.provider.clone(),
                                &self.model,
                                CheckpointTrigger::Manual,
                            )
                            .await
                            {
                                Ok(Some(result))
                                    if result.checkpoint.quality_score.unwrap_or_default()
                                        >= 0.6 =>
                                {
                                    Some(result.checkpoint.body)
                                }
                                Ok(None) | Ok(Some(_)) => {
                                    let updated = planning
                                        .registry
                                        .update_execution_context(
                                            &execution.id,
                                            Some(boundary),
                                            None,
                                            ExecutionInitializationStatus::Blocked,
                                            Some(
                                                "Canonical checkpoint did not pass quality validation"
                                                    .to_string(),
                                            ),
                                        )
                                        .await?;
                                    persist_execution_context(planning, session_key, &updated)
                                        .await?;
                                    return Err(anyhow::anyhow!(
                                        "approved plan execution is blocked: Canonical checkpoint did not pass quality validation"
                                    )
                                    .into());
                                }
                                Err(error) => {
                                    warn!("execution context initialization failed: {error}");
                                    let updated = planning
                                        .registry
                                        .update_execution_context(
                                            &execution.id,
                                            Some(boundary),
                                            None,
                                            ExecutionInitializationStatus::Blocked,
                                            Some(
                                                "Canonical checkpoint generation failed"
                                                    .to_string(),
                                            ),
                                        )
                                        .await?;
                                    persist_execution_context(planning, session_key, &updated)
                                        .await?;
                                    return Err(anyhow::anyhow!(
                                        "approved plan execution is blocked: Canonical checkpoint generation failed"
                                    )
                                    .into());
                                }
                            }
                        }
                        ExecutionContextPolicy::Clear | ExecutionContextPolicy::Retain => None,
                    };
                    let updated = planning
                        .registry
                        .update_execution_context(
                            &execution.id,
                            Some(boundary),
                            compacted,
                            ExecutionInitializationStatus::Ready,
                            None,
                        )
                        .await?;
                    persist_execution_context(planning, session_key, &updated).await?;
                    *active_execution = Some(updated);
                }
            }
        }

        self.clear_session_cancellation(session_key);
        let budget_report = {
            let session = self.sessions.get_or_create(session_key);
            let checkpoint = session
                .canonical_checkpoint
                .as_ref()
                .map(|value| value.render_for_context());
            check_context_budget(
                &session.get_history(usize::MAX),
                checkpoint.as_deref(),
                &self.tool_config.budget,
            )
        };
        let (mut history, mut canonical_checkpoint, did_compact) = if budget_report.should_compact {
            info!(
                    "Compaction triggered — budget pressure {:.1}% ({} tokens used of ~{} history budget)",
                    budget_report.pressure_ratio * 100.0,
                    budget_report.history_estimated,
                    self.tool_config.budget.max_tokens.saturating_sub(
                        (self.tool_config.budget.max_tokens as f64
                            * self.tool_config.budget.system_budget_ratio) as usize
                    ),
                );
            let compact_result = if let Some(session) = self.sessions.get(session_key) {
                CheckpointCompactor::compact_snapshot(
                    CheckpointSnapshot::from_session(session, Vec::new()),
                    &self.tool_config.budget,
                    self.provider.clone(),
                    &self.model,
                    CheckpointTrigger::Auto,
                )
                .await
            } else {
                Err(anyhow::anyhow!("Session not found for compaction"))
            };
            match compact_result {
                Ok(Some(result)) => {
                    let session = self.sessions.get_or_create(session_key);
                    session.canonical_checkpoint = Some(result.checkpoint);
                    (
                        session.get_history(usize::MAX),
                        session.canonical_checkpoint.clone(),
                        true,
                    )
                }
                Ok(None) => {
                    let session = self.sessions.get_or_create(session_key);
                    (
                        session.get_history(usize::MAX),
                        session.canonical_checkpoint.clone(),
                        false,
                    )
                }
                Err(error) => {
                    warn!("Compaction failed (non-blocking): {}", error);
                    let session = self.sessions.get_or_create(session_key);
                    (
                        session.get_history(usize::MAX),
                        session.canonical_checkpoint.clone(),
                        false,
                    )
                }
            }
        } else {
            let session = self.sessions.get_or_create(session_key);
            (
                session.get_history(usize::MAX),
                session.canonical_checkpoint.clone(),
                false,
            )
        };
        if did_compact {
            if let Some(session) = self.sessions.get(session_key) {
                if let Err(error) = self.sessions.save(session) {
                    error!("Failed to persist compaction state: {}", error);
                }
            }
        }

        if let Some(execution) = active_execution.as_ref().filter(|execution| {
            execution.initialization_status == ExecutionInitializationStatus::Ready
        }) {
            if let Some(boundary) = execution.boundary.as_ref() {
                match execution.context_policy {
                    ExecutionContextPolicy::Retain => {}
                    ExecutionContextPolicy::Clear | ExecutionContextPolicy::Compact => {
                        let session = self.sessions.get_or_create(session_key);
                        history = align_chat_history(
                            session
                                .messages
                                .iter()
                                .skip(boundary.message_count)
                                .cloned()
                                .collect(),
                        );
                        canonical_checkpoint = None;
                        if let Some(summary) = execution.compacted_context.as_ref() {
                            canonical_checkpoint = Some(CanonicalCheckpoint::new(
                                format!("execution:{}", execution.id),
                                execution.updated_at.to_rfc3339(),
                                CheckpointTrigger::Manual,
                                boundary.message_count,
                                boundary.message_count,
                                0,
                                Some(1.0),
                                Vec::new(),
                                0,
                                summary,
                            ));
                        }
                    }
                }
            }
        }

        let mut dynamic_sections = Vec::new();
        match self
            .memory_provider
            .working_memory_block(agent_diva_core::memory::WorkingMemoryRequest {
                workspace_root: self.workspace.clone(),
                session_id: session_key.to_string(),
            })
            .await
        {
            Ok(response) => {
                if let Some(block) = response
                    .prompt_block
                    .filter(|value| !value.trim().is_empty())
                {
                    dynamic_sections.push(PromptSection::new(ContextSection::WorkingMemory, block));
                }
            }
            Err(error) => {
                warn!("Working memory block failed (non-fatal): {}", error);
            }
        }
        let prefetch_intent = derive_prefetch_intent(&message_content);
        if !prefetch_intent.is_empty() {
            match self
                .memory_provider
                .prefetch(PrefetchRequest {
                    workspace_root: self.workspace.clone(),
                    intent: prefetch_intent,
                    current_room: Some(message.channel.clone()),
                    user_message: Some(message_content.clone()),
                })
                .await
            {
                Ok(response) => match response.status {
                    PrefetchStatus::Failed { reason } => {
                        warn!("Prefetch recall failed (non-fatal): {}", reason);
                    }
                    _ => {
                        if let Some(block) = response.prompt_block {
                            dynamic_sections
                                .push(PromptSection::new(ContextSection::PrefetchRecall, block));
                            trace!(
                                trace_id = %trace_id,
                                step_name = "prefetch_injected",
                                "Prefetch recall injected into turn context"
                            );
                        } else {
                            trace!(
                                trace_id = %trace_id,
                                step_name = "prefetch_skipped",
                                "Prefetch skipped or empty"
                            );
                        }
                    }
                },
                Err(error) => warn!("Prefetch recall failed (non-fatal): {}", error),
            }
        }

        dynamic_sections.push(
            self.context
                .build_volatile_meta_section(Some(&message.channel), Some(&message.chat_id)),
        );
        if scheduled {
            dynamic_sections.push(PromptSection::new(
                ContextSection::VolatileMeta,
                prompt::scheduled_turn().content,
            ));
        }
        if let Some(markdown) = approved_plan_markdown {
            dynamic_sections.push(PromptSection::new(
                ContextSection::PlanGuard,
                prompt::approved_plan(markdown).content,
            ));
        }
        if plan_guard_active {
            dynamic_sections.push(PromptSection::new(
                ContextSection::PlanGuard,
                prompt::plan_mode().content,
            ));
        }
        if read_only {
            dynamic_sections.push(PromptSection::new(
                ContextSection::PlanGuard,
                prompt::ask_mode().content,
            ));
        }

        let stable_prefix = self
            .context
            .stable_prefix_snapshot_for_session(active_mask, session_key);
        let mut prepared = PreparedTurnContext::prepare_budgeted(
            self.context.build_prefix_messages_from_snapshot(
                history,
                canonical_checkpoint.as_ref(),
                &stable_prefix,
            ),
            &dynamic_sections,
            self.provider.dynamic_context_transport(),
            current_turn_message.clone(),
            &self.tool_config.budget,
        )?;
        if did_compact {
            prepared.assembly_report.compacted.push(AssemblyDecision {
                id: "history".to_string(),
                region: ContextBudgetRegion::CanonicalCheckpoint,
                layer: BudgetLayer::History,
                reason: AssemblyDecisionReason::MacroCompaction,
            });
        }

        if prepared.assembly_report.has_pressure_action() {
            warn!(
                trace_id = %trace_id,
                total_estimated = prepared.assembly_report.total_estimated,
                total_max = prepared.assembly_report.total_max,
                dropped = prepared.assembly_report.dropped.len(),
                compacted = prepared.assembly_report.compacted.len(),
                "context assembly applied budget pressure actions"
            );
        } else {
            trace!(
                trace_id = %trace_id,
                total_estimated = prepared.assembly_report.total_estimated,
                total_max = prepared.assembly_report.total_max,
                "context assembly report"
            );
        }

        Ok(RuntimeTurnContext {
            message_content,
            current_turn_message,
            messages: prepared.messages,
            turn_messages_start: prepared.turn_messages_start,
            dynamic_sections,
            stable_prefix,
            assembly_report: prepared.assembly_report,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freezes_the_provider_prefix_boundary() {
        let context = PreparedTurnContext::prepare(
            vec![Message::system("system")],
            &[],
            DynamicContextTransport::UserContextEnvelope,
            Message::user("current"),
        )
        .unwrap();
        assert_eq!(context.turn_messages_start, 2);
        assert_eq!(context.messages.len(), 2);
    }

    #[test]
    fn scheduled_prompt_precedes_the_rich_current_turn_message() {
        let sections = vec![PromptSection::new(
            ContextSection::VolatileMeta,
            prompt::scheduled_turn().content,
        )];
        let context = PreparedTurnContext::prepare(
            vec![Message::system("system")],
            &sections,
            DynamicContextTransport::UserContextEnvelope,
            Message::user("current"),
        )
        .unwrap();
        assert_eq!(context.turn_messages_start, 3);
        assert_eq!(context.messages[1].role, "user");
        assert!(context.messages[1]
            .content
            .as_text()
            .is_some_and(|text| text.contains("scheduled cron job")));
        assert_eq!(context.messages[2].content.as_text(), Some("current"));
    }

    #[test]
    fn working_memory_is_a_post_history_user_context_block() {
        let sections = vec![PromptSection::new(
            ContextSection::WorkingMemory,
            "## Working Memory\nin-flight state",
        )];
        let context = PreparedTurnContext::prepare(
            vec![Message::system("system"), Message::assistant("history")],
            &sections,
            DynamicContextTransport::UserContextEnvelope,
            Message::user("current"),
        )
        .unwrap();
        assert_eq!(context.messages.len(), 4);
        assert_eq!(context.messages[2].role, "user");
        assert!(context.messages[2]
            .content
            .as_text()
            .is_some_and(|text| text.contains("Working Memory")));
        assert_eq!(context.messages[3].content.as_text(), Some("current"));
    }

    #[test]
    fn plan_prompts_share_the_typed_post_prefix_envelope() {
        let sections = vec![
            PromptSection::new(
                ContextSection::PlanGuard,
                prompt::approved_plan("# Approved\n- execute").content,
            ),
            PromptSection::new(ContextSection::PlanGuard, prompt::plan_mode().content),
        ];
        let context = PreparedTurnContext::prepare(
            vec![Message::system("system")],
            &sections,
            DynamicContextTransport::UserContextEnvelope,
            Message::user("current"),
        )
        .unwrap();

        assert_eq!(context.messages.len(), 3);
        assert_eq!(context.messages[0].content.as_text(), Some("system"));
        let envelope = context.messages[1].content.as_text().unwrap();
        assert!(envelope.find("Approved").unwrap() < envelope.find("Plan mode").unwrap());
        assert_eq!(context.messages[2].content.as_text(), Some("current"));
    }

    #[test]
    fn empty_dynamic_sections_do_not_create_an_envelope() {
        let sections = vec![PromptSection::new(ContextSection::WorkingMemory, "   ")];
        let context = PreparedTurnContext::prepare(
            vec![Message::system("system")],
            &sections,
            DynamicContextTransport::UserContextEnvelope,
            Message::user("current"),
        )
        .unwrap();
        assert_eq!(context.messages.len(), 2);
    }
}

#[cfg(test)]
mod wave3_tests {
    use super::*;

    #[test]
    fn legacy_prefetch_failure_leaves_messages_untouched() {
        let context = PreparedTurnContext::prepare(
            vec![Message::system("system")],
            &[],
            DynamicContextTransport::UserContextEnvelope,
            Message::user("current"),
        )
        .unwrap();
        assert_eq!(context.messages.len(), 2);
    }

    #[test]
    fn typed_prefetch_follows_working_memory_inside_one_envelope() {
        let sections = vec![
            PromptSection::new(ContextSection::PrefetchRecall, "## Recalled Memory\nplan"),
            PromptSection::new(ContextSection::WorkingMemory, "## Working Memory\nstate"),
        ];
        let context = PreparedTurnContext::prepare(
            vec![Message::system("system")],
            &sections,
            DynamicContextTransport::UserContextEnvelope,
            Message::user("current"),
        )
        .unwrap();
        let envelope = context.messages[1].content.as_text().unwrap();
        assert!(
            envelope.find("Working Memory").unwrap() < envelope.find("Recalled Memory").unwrap()
        );
        assert_eq!(context.messages[2].content.as_text(), Some("current"));
    }

    #[test]
    fn no_injection_when_both_prefetch_and_working_memory_are_absent() {
        let context = PreparedTurnContext::prepare(
            vec![Message::system("system")],
            &[],
            DynamicContextTransport::UserContextEnvelope,
            Message::user("current"),
        )
        .unwrap();
        assert_eq!(context.messages.len(), 2);
    }
}
