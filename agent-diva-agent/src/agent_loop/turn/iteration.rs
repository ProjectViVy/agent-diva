use agent_diva_core::bus::{AgentEvent, InboundMessage};
use agent_diva_core::session::{CompactTrigger, TokenUsage};
use agent_diva_providers::retry::{RetryAttempt, RetryListener};
use agent_diva_providers::{
    LLMResponse, LLMStreamEvent, Message, ProviderEventStream, ToolChoiceMode,
};
use futures::StreamExt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use super::super::loop_turn::is_context_overflow_error;
use super::super::AgentLoop;
use super::context::PreparedTurnContext;
use crate::compaction::ContextCompactor;
use crate::context_assembly::{
    apply_core_tool_cache_anchor, measure_provider_context, AssemblyDecision,
    AssemblyDecisionReason, BudgetLayer, CacheObservationTicket, CacheObserveInput,
    ContextBudgetPlan, PromptSection, StablePrefixSnapshot,
};
use agent_diva_tooling::ToolDefinitionSet;

const REJECTION_CIRCUIT_TRIPPED: &str =
    "model/provider rejection storm tripped the circuit breaker; refusing new iteration";

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
        tool_definitions: &ToolDefinitionSet,
        summary_only: bool,
        session_key: &str,
        model: &str,
        message: &InboundMessage,
        dynamic_sections: &[PromptSection],
        current_turn_message: &Message,
        turn_messages_start: &mut usize,
        stable_prefix: &StablePrefixSnapshot,
    ) -> Result<(ProviderEventStream, CacheObservationTicket), Box<dyn std::error::Error>> {
        let mut reactive_retry_attempted = false;
        loop {
            if self.rejection_circuit.is_triggered() {
                warn!(
                    session_id = %session_key,
                    rejections = self.rejection_circuit.rejection_count(),
                    threshold = self.rejection_circuit.threshold(),
                    "model/provider rejection storm tripped the circuit breaker"
                );
                return Err(REJECTION_CIRCUIT_TRIPPED.into());
            }
            self.enforce_session_token_budget(session_key)?;
            let budget_plan = ContextBudgetPlan::from_config(&self.tool_config.budget);
            let preliminary_report =
                measure_provider_context(messages, tool_definitions, &budget_plan);
            let tool_result_tokens = preliminary_report
                .totals_by_layer
                .get(&BudgetLayer::ToolResultInline)
                .copied()
                .unwrap_or_default();
            let microcompacted = if tool_result_tokens
                > budget_plan.layer(BudgetLayer::ToolResultInline).soft_limit
            {
                crate::tool_results::microcompact_tool_results(
                    &self.workspace,
                    session_key,
                    messages,
                )
                .await
            } else {
                Vec::new()
            };
            let mut assembly_report =
                measure_provider_context(messages, tool_definitions, &budget_plan);
            assembly_report
                .compacted
                .extend(microcompacted.iter().map(|tool_call_id| AssemblyDecision {
                    id: format!("tool_result:{tool_call_id}"),
                    layer: BudgetLayer::ToolResultInline,
                    reason: AssemblyDecisionReason::Microcompact,
                }));
            if !microcompacted.is_empty() {
                debug!(
                    session_id = %session_key,
                    compacted_tool_results = microcompacted.len(),
                    "microcompacted old inline tool results before cache observation"
                );
            }
            if assembly_report.total_estimated > assembly_report.total_max {
                warn!(
                    session_id = %session_key,
                    total_estimated = assembly_report.total_estimated,
                    total_max = assembly_report.total_max,
                    dropped = assembly_report.dropped.len(),
                    compacted = assembly_report.compacted.len(),
                    "provider context exceeds the typed hard budget"
                );
            } else {
                debug!(
                    session_id = %session_key,
                    total_estimated = assembly_report.total_estimated,
                    total_max = assembly_report.total_max,
                    layers = assembly_report.totals_by_layer.len(),
                    "provider context assembly report"
                );
            }
            let profile = self.provider.prompt_cache_profile(model);
            let mut cacheable_tools = tool_definitions.clone();
            apply_core_tool_cache_anchor(&mut cacheable_tools, &profile);
            let tools = (!cacheable_tools.is_empty()).then_some(cacheable_tools.definitions);
            crate::context::ContextBuilder::sanitize_messages_for_provider(messages);
            let final_wire_snapshot = Arc::new(Mutex::new(None));
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
                let snapshot_slot = final_wire_snapshot.clone();
                self.provider
                    .set_final_wire_cache_listener(Some(Arc::new(move |snapshot| {
                        if let Ok(mut slot) = snapshot_slot.lock() {
                            *slot = Some(snapshot);
                        }
                    })));
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
                self.provider.set_final_wire_cache_listener(None);
                result
            };
            let snapshot = final_wire_snapshot
                .lock()
                .ok()
                .and_then(|mut slot| slot.take());
            let (cache_ticket, _) = self.cache_observer.note_pre_call(CacheObserveInput {
                session_id: session_key,
                snapshot,
                break_reasons: &stable_prefix.cache_break_reasons(),
                expected_deletion: !microcompacted.is_empty(),
            });
            match stream_result {
                Ok(stream) => return Ok((stream, cache_ticket)),
                Err(error) if !reactive_retry_attempted && is_context_overflow_error(&error) => {
                    self.cache_observer.abandon_call(cache_ticket);
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

                    let stable_prefix = messages.first().cloned().ok_or_else(|| {
                        anyhow::anyhow!("reactive compaction cannot recover a stable prefix")
                    })?;
                    let mut prefix = self.context.build_prefix_messages_for_session(
                        history,
                        &compaction_history,
                        session_key,
                        None,
                    );
                    if let Some(first) = prefix.first_mut() {
                        *first = stable_prefix;
                    }
                    let prepared = PreparedTurnContext::prepare_budgeted(
                        prefix,
                        dynamic_sections,
                        self.provider.dynamic_context_transport(),
                        current_turn_message.clone(),
                        &self.tool_config.budget,
                    )?;
                    *turn_messages_start = prepared.turn_messages_start;
                    *messages = prepared.messages;
                    info!("Reactive compaction complete, retrying provider call...");
                }
                Err(error) => {
                    self.cache_observer.abandon_call(cache_ticket);
                    let count = self.rejection_circuit.record_rejection();
                    warn!(
                        session_id = %session_key,
                        model = %model,
                        rejection_count = count,
                        threshold = self.rejection_circuit.threshold(),
                        "provider call failed; counted toward rejection circuit: {}",
                        error
                    );
                    return Err(error.into());
                }
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
