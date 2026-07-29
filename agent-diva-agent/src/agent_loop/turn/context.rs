use agent_diva_core::audit::{self, AuditEvent};
use agent_diva_core::bus::InboundMessage;
use agent_diva_core::memory::{PrefetchRequest, PrefetchStatus};
use agent_diva_core::planning::{
    ExecutionContextBoundary, ExecutionContextPolicy, ExecutionInitializationStatus,
    ExecutionSession,
};
use agent_diva_core::security::{check_security, SecurityContext, SecurityDecision};
use agent_diva_core::session::{align_chat_history, CompactTrigger};
use agent_diva_providers::{supports_vision_model, Message};
use tracing::{error, info, trace, warn};

use crate::compaction::ContextCompactor;
use crate::context::ContextBuilder;
use crate::context_budget::check_budget;
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
}

pub(crate) struct RuntimeTurnContext {
    pub message_content: String,
    pub current_turn_message: Message,
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
        active_mask: Option<&MaskFile>,
        scheduled: bool,
        trace_id: &str,
    ) -> Result<RuntimeTurnContext, Box<dyn std::error::Error>> {
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
                            snapshot.last_compacted = 0;
                            snapshot.compaction_history.clear();
                            let mut config = self.tool_config.budget.clone();
                            config.keep_recent_count = 0;
                            match ContextCompactor::compact(
                                &snapshot,
                                &config,
                                self.provider.clone(),
                                &self.model,
                                CompactTrigger::Manual,
                                &[],
                            )
                            .await
                            {
                                Ok(result)
                                    if !result.summary.summary.trim().is_empty()
                                        && result.summary.quality_score.unwrap_or_default()
                                            >= 0.6 =>
                                {
                                    Some(result.summary.summary)
                                }
                                Ok(_) => {
                                    let updated = planning
                                        .registry
                                        .update_execution_context(
                                            &execution.id,
                                            Some(boundary),
                                            None,
                                            ExecutionInitializationStatus::Blocked,
                                            Some(
                                                "Compact summary did not pass quality validation"
                                                    .to_string(),
                                            ),
                                        )
                                        .await?;
                                    persist_execution_context(planning, session_key, &updated)
                                        .await?;
                                    return Err(anyhow::anyhow!(
                                        "approved plan execution is blocked: Compact summary did not pass quality validation"
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
                                            Some("Compact summary generation failed".to_string()),
                                        )
                                        .await?;
                                    persist_execution_context(planning, session_key, &updated)
                                        .await?;
                                    return Err(anyhow::anyhow!(
                                        "approved plan execution is blocked: Compact summary generation failed"
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
            check_budget(&session.get_history(50), &self.tool_config.budget)
        };
        let (mut history, mut compaction_history, did_compact) = if budget_report.should_compact {
            info!(
                    "Compaction triggered — budget pressure {:.1}% ({} tokens used of ~{} history budget)",
                    budget_report.pressure_ratio * 100.0,
                    budget_report.history_estimated,
                    budget_report
                        .total_estimated
                        .saturating_sub(budget_report.system_estimated),
                );
            let compact_result = if let Some(session) = self.sessions.get(session_key) {
                ContextCompactor::compact(
                    session,
                    &self.tool_config.budget,
                    self.provider.clone(),
                    &self.model,
                    CompactTrigger::Auto,
                    &session.compaction_history,
                )
                .await
            } else {
                Err(anyhow::anyhow!("Session not found for compaction"))
            };
            match compact_result {
                Ok(result) => {
                    let session = self.sessions.get_or_create(session_key);
                    session.last_compacted = result.new_compacted_index;
                    session.compaction_history.push(result.summary);
                    (
                        session.get_history(50),
                        session.compaction_history.clone(),
                        true,
                    )
                }
                Err(error) => {
                    warn!("Compaction failed (non-blocking): {}", error);
                    let session = self.sessions.get_or_create(session_key);
                    (
                        session.get_history(50),
                        session.compaction_history.clone(),
                        false,
                    )
                }
            }
        } else {
            let session = self.sessions.get_or_create(session_key);
            (
                session.get_history(50),
                session.compaction_history.clone(),
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
                        compaction_history.clear();
                        if let Some(summary) = execution.compacted_context.as_ref() {
                            compaction_history.push(agent_diva_core::session::CompactSummary {
                                schema_version: 1,
                                compact_id: format!("execution:{}", execution.id),
                                created_at: execution.updated_at.to_rfc3339(),
                                trigger: CompactTrigger::Manual,
                                source_range: agent_diva_core::session::CompactionRange {
                                    start_index: 0,
                                    end_index: boundary.message_count,
                                },
                                kept_recent_count: 0,
                                pre_compact_message_count: boundary.message_count,
                                pre_compact_estimated_tokens: 0,
                                summary: summary.clone(),
                                quality_score: Some(1.0),
                                retry_count: 0,
                            });
                        }
                    }
                }
            }
        }

        let prepared = PreparedTurnContext::prepare(
            self.context.build_messages(
                history,
                message_content.clone(),
                Some(&message.channel),
                Some(&message.chat_id),
                &compaction_history,
            ),
            plan_guard_active,
            approved_plan_markdown,
            active_mask.map(|mask| self.context.build_system_prompt(Some(mask))),
            scheduled,
            current_turn_message.clone(),
        );
        let mut messages = prepared.messages;
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
                            messages.insert(1, Message::system(block));
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

        Ok(RuntimeTurnContext {
            message_content,
            current_turn_message,
            messages,
            turn_messages_start: prepared.turn_messages_start,
        })
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
