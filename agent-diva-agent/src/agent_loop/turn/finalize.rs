use agent_diva_core::bus::{AgentEvent, InboundMessage, OutboundMessage};
use agent_diva_core::planning::{
    normalize_report_markdown, report_validation_issues, resolve_plan_report_body,
    strip_proposed_plan_block, PlanRevisionAuthor,
};
use agent_diva_core::session::TokenUsage;
use agent_diva_providers::Message;
use tokio::sync::mpsc;
use tracing::{error, info, trace, warn};

use super::super::super::consolidation;
use super::super::loop_turn::{fallback_session_title, save_turn, should_generate_session_title};
use super::super::AgentLoop;
use super::iteration::IterationOutcome;

/// Narrow input boundary for response/session finalization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FinalizationInput {
    pub content: String,
    pub reasoning: Option<String>,
    pub usage: Option<TokenUsage>,
}

impl From<IterationOutcome> for FinalizationInput {
    fn from(outcome: IterationOutcome) -> Self {
        Self {
            content: outcome.content.unwrap_or_default(),
            reasoning: outcome.reasoning,
            usage: outcome.token_usage,
        }
    }
}

/// Owned values needed after model iteration has committed to a final response.
pub(crate) struct FinalizationContext {
    pub message: InboundMessage,
    pub messages: Vec<Message>,
    pub session_key: String,
    pub message_content: String,
    pub turn_messages_start: usize,
    pub system_turn: bool,
    pub model: String,
    pub trace_id: String,
}

pub(crate) struct FinalizationPreparation<'a> {
    pub message: &'a InboundMessage,
    pub event_tx: Option<&'a mpsc::UnboundedSender<AgentEvent>>,
    pub session_key: &'a str,
    pub plan_mode: bool,
    pub rendered_content: String,
}

impl AgentLoop {
    pub(crate) async fn prepare_finalization(
        &mut self,
        preparation: FinalizationPreparation<'_>,
        outcome: IterationOutcome,
    ) -> FinalizationInput {
        let FinalizationPreparation {
            message,
            event_tx,
            session_key,
            plan_mode,
            mut rendered_content,
        } = preparation;

        if plan_mode {
            if let Some(planning) = &self.tool_config.planning {
                if let Some(extracted) = resolve_plan_report_body(&rendered_content) {
                    let markdown = normalize_report_markdown(&extracted.markdown);
                    let title = markdown
                        .lines()
                        .find_map(|line| line.trim().strip_prefix("# "))
                        .unwrap_or("Plan report");
                    let soft_issues = report_validation_issues(&markdown);
                    match planning
                        .registry
                        .create_report(session_key, title, &markdown, PlanRevisionAuthor::Agent)
                        .await
                    {
                        Ok(report) => {
                            if extracted.tagged {
                                rendered_content = strip_proposed_plan_block(&rendered_content);
                            } else {
                                rendered_content.clear();
                            }
                            if rendered_content.trim().is_empty() {
                                rendered_content =
                                    "已生成计划报告，请在下方审批卡片中查看并批准。".to_string();
                            }
                            if !soft_issues.is_empty() {
                                let missing: Vec<String> =
                                    soft_issues.iter().map(ToString::to_string).collect();
                                rendered_content.push_str(&format!(
                                    "\n\n> 计划已提交审批，但章节仍不完整（{}）。可直接批准，或点编辑继续完善。",
                                    missing.join("；")
                                ));
                            }
                            self.emit_agent_event(
                                message,
                                event_tx,
                                AgentEvent::PlanReportReadyForApproval { report },
                            );
                        }
                        Err(error) => warn!(%error, "failed to persist plan report"),
                    }
                }
            }
        }

        // Preserve the pre-existing user-visible response contract: report
        // demultiplexing affects the durable report/event surface only.
        FinalizationInput::from(outcome)
    }

    /// Persist and publish one completed turn after all policy-sensitive work.
    pub(crate) async fn finalize_turn(
        &mut self,
        context: FinalizationContext,
        finalization: FinalizationInput,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
    ) -> Result<Option<OutboundMessage>, Box<dyn std::error::Error>> {
        let FinalizationContext {
            message,
            messages,
            session_key,
            message_content,
            turn_messages_start,
            system_turn,
            model,
            trace_id,
        } = context;

        trace!(
            trace_id = %trace_id,
            step_name = "response_generated",
            "Response generated"
        );
        let preview = if finalization.content.chars().count() > 120 {
            format!(
                "{}...",
                finalization.content.chars().take(120).collect::<String>()
            )
        } else {
            finalization.content.clone()
        };
        info!(
            "Response to {}:{}: {}",
            message.channel, message.sender_id, preview
        );

        let event = AgentEvent::FinalResponse {
            content: finalization.content.clone(),
        };
        if let Some(tx) = event_tx {
            let _ = tx.send(event.clone());
        }
        let _ = self
            .bus
            .publish_event(message.channel.clone(), message.chat_id.clone(), event);

        {
            let session = self.sessions.get_or_create(&session_key);
            save_turn(
                session,
                &messages,
                turn_messages_start,
                if system_turn { "system" } else { "user" },
                &message_content,
                &finalization.content,
                finalization.usage.clone(),
            );
        }

        {
            let session = self.sessions.get_or_create(&session_key);
            if consolidation::should_consolidate(session, self.memory_window) {
                if let Err(error) = consolidation::consolidate(
                    session,
                    &self.provider,
                    &model,
                    &self.workspace,
                    &*self.memory_provider,
                    self.memory_window,
                )
                .await
                {
                    error!("Memory consolidation failed: {}", error);
                }
            }
        }

        let title_update = if let Some(session) = self.sessions.get(&session_key) {
            if should_generate_session_title(session) {
                let fallback = fallback_session_title(session);
                let generated = self.generate_session_title_with_llm(session, &model).await;
                generated
                    .clone()
                    .or(fallback)
                    .map(|title| (title, generated.is_some()))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((title, generated)) = title_update {
            let session = self.sessions.get_or_create(&session_key);
            session.set_conversation_title(Some(title));
            session.set_title_generated(generated);
            if !session.title_manually_set() {
                session.set_title_manually_set(false);
            }
        }

        if let Some(session) = self.sessions.get(&session_key) {
            if let Err(error) = self.sessions.save(session) {
                error!("Failed to save session: {}", error);
            }
        }

        let reply_to = message
            .metadata
            .get("message_id")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        trace!(
            trace_id = %trace_id,
            step_name = "msg_sent_to_channel",
            "Returning response to channel/manager"
        );
        trace!(
            trace_id = %trace_id,
            step_name = "msg_sent_to_manager",
            "Returning response to manager"
        );

        Ok(Some(OutboundMessage {
            channel: message.channel,
            chat_id: message.chat_id,
            content: finalization.content,
            reply_to,
            media: vec![],
            reasoning_content: finalization.reasoning,
            metadata: message.metadata,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_iteration_output_at_the_persistence_boundary() {
        let finalization = FinalizationInput::from(IterationOutcome {
            content: Some("answer".into()),
            reasoning: Some("reasoning".into()),
            ..Default::default()
        });
        assert_eq!(finalization.content, "answer");
        assert_eq!(finalization.reasoning.as_deref(), Some("reasoning"));
        assert!(finalization.usage.is_none());
    }
}
