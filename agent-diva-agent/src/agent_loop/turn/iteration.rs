use agent_diva_core::bus::{AgentEvent, InboundMessage};
use agent_diva_core::session::{CompactTrigger, TokenUsage};
use agent_diva_providers::retry::{RetryAttempt, RetryListener};
use agent_diva_providers::{
    LLMResponse, LLMStreamEvent, Message, ProviderEventStream, ToolChoiceMode,
};
use futures::StreamExt;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::super::loop_turn::{is_context_overflow_error, replace_current_turn_message};
use super::super::AgentLoop;
use super::prompt;
use crate::compaction::ContextCompactor;

const INTERNAL_PROTOCOL_MARKERS: &[&str] = &[
    "<｜DSML｜tool_calls>",
    "<｜DSML｜invoke",
    "<｜DSML｜parameter",
    "<｜｜DSML｜｜tool_calls>",
    "<｜｜DSML｜｜invoke",
    "<｜｜DSML｜｜parameter",
    "<tool_calls>",
    "<function_calls>",
];

pub(crate) fn contains_internal_protocol(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    INTERNAL_PROTOCOL_MARKERS
        .iter()
        .any(|marker| lower.contains(&marker.to_ascii_lowercase()))
}

/// Withholds a short suffix so protocol markers split across chunks cannot leak.
#[derive(Default)]
pub(crate) struct InternalProtocolGuard {
    pending: String,
    detected: bool,
}

impl InternalProtocolGuard {
    pub(crate) fn push(&mut self, delta: String) -> Option<String> {
        if self.detected {
            return None;
        }
        self.pending.push_str(&delta);
        if contains_internal_protocol(&self.pending) {
            self.detected = true;
            self.pending.clear();
            return None;
        }

        const RETAINED_CHARS: usize = 32;
        let chars: Vec<char> = self.pending.chars().collect();
        if chars.len() <= RETAINED_CHARS {
            return None;
        }
        let split_at = chars.len() - RETAINED_CHARS;
        let safe: String = chars[..split_at].iter().collect();
        self.pending = chars[split_at..].iter().collect();
        Some(safe)
    }

    pub(crate) fn detected(&self) -> bool {
        self.detected
    }

    pub(crate) fn finish(&mut self) -> Option<String> {
        if self.detected || self.pending.is_empty() {
            return None;
        }
        Some(std::mem::take(&mut self.pending))
    }
}

pub(crate) struct StreamedModelStep {
    pub response: LLMResponse,
    pub protocol_leak_detected: bool,
}

impl AgentLoop {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn start_model_stream(
        &mut self,
        messages: &mut Vec<Message>,
        tool_definitions: &[Value],
        summary_only: bool,
        session_key: &str,
        model: &str,
        message: &InboundMessage,
        message_content: &str,
        approved_plan_markdown: Option<&str>,
        read_only: bool,
        scheduled: bool,
        current_turn_message: &Message,
    ) -> Result<ProviderEventStream, Box<dyn std::error::Error>> {
        let mut reactive_retry_attempted = false;
        loop {
            self.enforce_session_token_budget(session_key)?;
            let tools = (!tool_definitions.is_empty()).then(|| tool_definitions.to_vec());
            crate::context::ContextBuilder::sanitize_messages_for_provider(messages);
            let stream_result = {
                let bus = self.bus.clone();
                let channel = message.channel.clone();
                let chat_id = message.chat_id.clone();
                let listener: RetryListener = Arc::new(move |r: RetryAttempt| {
                    let _ = bus.publish_event(
                        channel.clone(),
                        chat_id.clone(),
                        AgentEvent::ProviderRetry {
                            model: r.model,
                            attempt: r.attempt,
                            max_retries: r.max_retries,
                            delay_ms: r.delay_ms,
                            reason: r.reason,
                        },
                    );
                });
                self.provider.set_retry_listener(Some(listener));
                let result = self
                    .provider
                    .chat_stream(
                        messages.clone(),
                        tools,
                        if summary_only {
                            ToolChoiceMode::Disabled
                        } else if tool_definitions.is_empty() {
                            ToolChoiceMode::Unspecified
                        } else {
                            ToolChoiceMode::Auto
                        },
                        Some(model.to_string()),
                        4096,
                        0.7,
                    )
                    .await;
                self.provider.set_retry_listener(None);
                result
            };
            match stream_result {
                Ok(stream) => return Ok(stream),
                Err(error) if !reactive_retry_attempted && is_context_overflow_error(&error) => {
                    warn!(
                        "Context overflow detected from provider: {}. Triggering reactive compaction...",
                        error
                    );
                    reactive_retry_attempted = true;
                    let compact_result = if let Some(session) = self.sessions.get(session_key) {
                        ContextCompactor::compact(
                            session,
                            &self.tool_config.budget,
                            self.provider.clone(),
                            &self.model,
                            CompactTrigger::Reactive,
                            &session.compaction_history,
                        )
                        .await
                    } else {
                        Err(anyhow::anyhow!("Session not found for reactive compaction"))
                    };

                    let (history, compaction_history) = match compact_result {
                        Ok(result) => {
                            let session = self.sessions.get_or_create(session_key);
                            session.last_compacted = result.new_compacted_index;
                            session.compaction_history.push(result.summary);
                            (session.get_history(50), session.compaction_history.clone())
                        }
                        Err(error) => {
                            warn!("Reactive compaction failed (non-blocking): {}", error);
                            let session = self.sessions.get_or_create(session_key);
                            (session.get_history(50), session.compaction_history.clone())
                        }
                    };
                    if let Some(session) = self.sessions.get(session_key) {
                        if let Err(error) = self.sessions.save(session) {
                            error!("Failed to persist reactive compaction state: {}", error);
                        }
                    }

                    *messages = self.context.build_messages_for_session(
                        history,
                        message_content.to_string(),
                        Some(&message.channel),
                        Some(&message.chat_id),
                        &compaction_history,
                        session_key,
                    );
                    if let Some(markdown) = approved_plan_markdown {
                        messages.insert(1, prompt::approved_plan(markdown).system());
                    }
                    if read_only {
                        messages.insert(1, prompt::ask_mode().system());
                    }
                    if scheduled {
                        let current_message = messages.pop();
                        messages.push(prompt::scheduled_turn().system());
                        if let Some(current_message) = current_message {
                            messages.push(current_message);
                        }
                    }
                    replace_current_turn_message(messages, current_turn_message.clone());
                    crate::context::ContextBuilder::sanitize_messages_for_provider(messages);
                    info!("Reactive compaction complete, retrying provider call...");
                }
                Err(error) => return Err(error.into()),
            }
        }
    }

    pub(crate) async fn collect_model_stream(
        &mut self,
        mut stream: ProviderEventStream,
        session_key: &str,
        message: &InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
    ) -> Result<Option<StreamedModelStep>, Box<dyn std::error::Error>> {
        let mut streamed_content = String::new();
        let mut streamed_reasoning = String::new();
        let mut output_guard = InternalProtocolGuard::default();
        let mut response = None;

        loop {
            self.drain_runtime_control_commands().await;
            if self.is_session_cancelled(session_key) {
                self.emit_error_event(message, event_tx, "Generation stopped by user.");
                return Ok(None);
            }

            let stream_event =
                match tokio::time::timeout(Duration::from_millis(250), stream.next()).await {
                    Ok(Some(event)) => event,
                    Ok(None) => break,
                    Err(_) => continue,
                };
            match stream_event? {
                LLMStreamEvent::TextDelta(delta) => {
                    streamed_content.push_str(&delta);
                    if let Some(safe_delta) = output_guard.push(delta) {
                        let event = AgentEvent::AssistantDelta { text: safe_delta };
                        if let Some(tx) = event_tx {
                            let _ = tx.send(event.clone());
                        }
                        let _ = self.bus.publish_event(
                            message.channel.clone(),
                            message.chat_id.clone(),
                            event,
                        );
                    }
                }
                LLMStreamEvent::ReasoningDelta(delta) => {
                    debug!("Stream ReasoningDelta: {:?}", delta);
                    streamed_reasoning.push_str(&delta);
                    let event = AgentEvent::ReasoningDelta { text: delta };
                    if let Some(tx) = event_tx {
                        let _ = tx.send(event.clone());
                    }
                    let _ = self.bus.publish_event(
                        message.channel.clone(),
                        message.chat_id.clone(),
                        event,
                    );
                }
                LLMStreamEvent::ToolCallDelta {
                    name,
                    arguments_delta,
                    ..
                } => {
                    if let Some(delta) = arguments_delta {
                        let event = AgentEvent::ToolCallDelta {
                            name,
                            args_delta: delta,
                        };
                        if let Some(tx) = event_tx {
                            let _ = tx.send(event.clone());
                        }
                        let _ = self.bus.publish_event(
                            message.channel.clone(),
                            message.chat_id.clone(),
                            event,
                        );
                    }
                }
                LLMStreamEvent::Completed(done) => {
                    response = Some(done);
                    break;
                }
            }
        }

        let response = response.unwrap_or_else(|| LLMResponse {
            content: (!streamed_content.is_empty()).then_some(streamed_content),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: std::collections::HashMap::new(),
            reasoning_content: (!streamed_reasoning.is_empty()).then_some(streamed_reasoning),
        });
        let protocol_leak_detected = output_guard.detected()
            || response
                .content
                .as_deref()
                .is_some_and(contains_internal_protocol);
        if !protocol_leak_detected {
            if let Some(safe_tail) = output_guard.finish() {
                let event = AgentEvent::AssistantDelta { text: safe_tail };
                if let Some(tx) = event_tx {
                    let _ = tx.send(event.clone());
                }
                let _ =
                    self.bus
                        .publish_event(message.channel.clone(), message.chat_id.clone(), event);
            }
        }

        Ok(Some(StreamedModelStep {
            response,
            protocol_leak_detected,
        }))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IterationPass {
    pub index: usize,
    pub summary_only: bool,
}

/// Owns the bounded model-loop counters, including the single summary-only pass.
#[derive(Debug, Default)]
pub(crate) struct IterationBudget {
    completed: usize,
    summary_bonus_remaining: usize,
    summary_nudge_injected: bool,
}

impl IterationBudget {
    pub(crate) fn can_continue(&self, max_iterations: usize) -> bool {
        self.completed < max_iterations + self.summary_bonus_remaining
    }

    pub(crate) fn begin_pass(&mut self, max_iterations: usize) -> IterationPass {
        self.completed += 1;
        let summary_only = self.completed > max_iterations;
        if summary_only {
            self.summary_bonus_remaining = 0;
        }
        IterationPass {
            index: self.completed,
            summary_only,
        }
    }

    pub(crate) fn take_summary_nudge(&mut self) -> bool {
        if self.summary_nudge_injected {
            false
        } else {
            self.summary_nudge_injected = true;
            true
        }
    }

    pub(crate) fn grant_summary_bonus_if_exhausted(&mut self, max_iterations: usize) -> bool {
        if self.completed >= max_iterations && self.summary_bonus_remaining == 0 {
            self.summary_bonus_remaining = 1;
            true
        } else {
            false
        }
    }
}

/// Data handed from model iteration to finalization.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct IterationOutcome {
    pub content: Option<String>,
    pub reasoning: Option<String>,
    pub token_usage: Option<TokenUsage>,
    pub stopped_for_plan_approval: bool,
}

impl IterationOutcome {
    pub(crate) fn resolved_content(&self) -> &str {
        self.content.as_deref().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_text_and_plan_barrier_state_for_finalization() {
        let outcome = IterationOutcome {
            content: Some("done".into()),
            stopped_for_plan_approval: true,
            ..Default::default()
        };
        assert_eq!(outcome.resolved_content(), "done");
        assert!(outcome.stopped_for_plan_approval);
    }

    #[test]
    fn grants_and_consumes_exactly_one_summary_only_pass() {
        let mut budget = IterationBudget::default();
        assert!(budget.can_continue(1));
        assert_eq!(
            budget.begin_pass(1),
            IterationPass {
                index: 1,
                summary_only: false,
            }
        );
        assert!(budget.grant_summary_bonus_if_exhausted(1));
        assert!(budget.can_continue(1));
        assert_eq!(
            budget.begin_pass(1),
            IterationPass {
                index: 2,
                summary_only: true,
            }
        );
        assert!(!budget.can_continue(1));
        assert!(budget.take_summary_nudge());
        assert!(!budget.take_summary_nudge());
    }
}
