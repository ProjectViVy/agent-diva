use agent_diva_core::bus::{AgentEvent, InboundMessage};
use agent_diva_core::session::{ChatMessage, CheckpointTrigger, TokenUsage};
use agent_diva_providers::retry::{RetryAttempt, RetryListener};
use agent_diva_providers::{
    LLMResponse, LLMStreamEvent, Message, ProviderEventStream, ToolChoiceMode,
};
use futures::StreamExt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::super::loop_turn::is_context_overflow_error;
use super::super::AgentLoop;
use super::context::PreparedTurnContext;
use crate::compaction::{CheckpointCompactor, CheckpointSnapshot};
use crate::context::ContextBuilder;
use crate::context_assembly::{
    apply_core_tool_cache_anchor, core_tool_hash, measure_provider_context, AssemblyDecision,
    AssemblyDecisionReason, BudgetLayer, CacheObservationTicket, CacheObserveInput,
    ContextBudgetPlan, ContextBudgetRegion, PromptSection, StablePrefixSnapshot,
};
use agent_diva_tooling::ToolDefinitionSet;

const REJECTION_CIRCUIT_TRIPPED: &str =
    "model/provider rejection storm tripped the circuit breaker; refusing new iteration";

pub(crate) const DEFAULT_COMPLETION_TOKENS: i32 = 4096;
pub(crate) const SUMMARY_ONLY_COMPLETION_TOKENS: i32 = 8192;
pub(crate) const FALLBACK_CONTEXT_WINDOW: usize = 128_000;
const INPUT_PRESSURE_RATIO_NUMERATOR: u64 = 90;
const INPUT_PRESSURE_RATIO_DENOMINATOR: u64 = 100;

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

pub(crate) struct StartedModelStream {
    pub stream: ProviderEventStream,
    pub cache_ticket: CacheObservationTicket,
    pub assembly_total_estimated: usize,
    pub assembly_total_max: usize,
    pub requested_max_tokens: i32,
}

/// Why a successful provider call produced no user-visible text after tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EmptyFollowupKind {
    OutputTruncated,
    InputPressure,
    EmptyStop,
}

pub(crate) struct EmptyFollowupSignals<'a> {
    pub content: Option<&'a str>,
    pub finish_reason: &'a str,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub context_window: usize,
    pub assembly_total_estimated: usize,
    pub assembly_total_max: usize,
    pub requested_max_tokens: i32,
    pub has_tool_results: bool,
    pub summary_only: bool,
}

pub(crate) fn classify_empty_followup(
    signals: EmptyFollowupSignals<'_>,
) -> Option<EmptyFollowupKind> {
    if signals.content.is_some_and(|text| !text.trim().is_empty()) {
        return None;
    }
    if !signals.has_tool_results || signals.summary_only {
        return None;
    }

    let window = signals.context_window.max(1);
    let near_window = u64::from(signals.prompt_tokens) * INPUT_PRESSURE_RATIO_DENOMINATOR
        >= window as u64 * INPUT_PRESSURE_RATIO_NUMERATOR;
    let over_assembly = signals.assembly_total_max > 0
        && signals.assembly_total_estimated > signals.assembly_total_max;
    if near_window || over_assembly {
        return Some(EmptyFollowupKind::InputPressure);
    }

    let finish = signals.finish_reason.to_ascii_lowercase();
    let hit_output_cap = signals.requested_max_tokens > 0
        && signals.completion_tokens >= signals.requested_max_tokens as u32;
    if finish == "length"
        || finish == "max_tokens"
        || finish == "max_output_tokens"
        || hit_output_cap
    {
        return Some(EmptyFollowupKind::OutputTruncated);
    }

    Some(EmptyFollowupKind::EmptyStop)
}

pub(crate) fn context_window_for_model(model: &str) -> usize {
    agent_diva_providers::model_capabilities_for_model(model)
        .context_window
        .unwrap_or(FALLBACK_CONTEXT_WINDOW)
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
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
    ) -> Result<StartedModelStream, Box<dyn std::error::Error>> {
        let requested_max_tokens = if summary_only {
            SUMMARY_ONLY_COMPLETION_TOKENS
        } else {
            DEFAULT_COMPLETION_TOKENS
        };
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
            let microcompact = if tool_result_tokens
                > budget_plan.layer(BudgetLayer::ToolResultInline).soft_limit
            {
                crate::tool_results::microcompact_tool_results(
                    &self.workspace,
                    session_key,
                    messages,
                )
                .await
            } else {
                crate::tool_results::MicrocompactReport::default()
            };
            let mut assembly_report =
                measure_provider_context(messages, tool_definitions, &budget_plan);
            assembly_report
                .compacted
                .extend(
                    microcompact
                        .compacted
                        .iter()
                        .map(|tool_call_id| AssemblyDecision {
                            id: format!("tool_result:{tool_call_id}"),
                            region: ContextBudgetRegion::ActiveTail,
                            layer: BudgetLayer::ToolResultInline,
                            reason: AssemblyDecisionReason::Microcompact,
                        }),
                );
            if !microcompact.compacted.is_empty() {
                debug!(
                    session_id = %session_key,
                    compacted_tool_results = microcompact.compacted.len(),
                    "microcompacted old inline tool results before cache observation"
                );
            }
            for (tool_call_id, error_code) in &microcompact.failures {
                warn!(
                    session_id = %session_key,
                    tool_call_id,
                    error_code,
                    "tool result microcompact left the original message unchanged"
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
                let event_message = message.clone();
                let listener: RetryListener = Arc::new(move |r: RetryAttempt| {
                    super::super::publish_message_event(
                        &bus,
                        &event_message,
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
                        requested_max_tokens,
                        0.7,
                    )
                    .await;
                self.provider.set_retry_listener(None);
                self.provider.set_final_wire_cache_listener(None);
                result
            };
            let mut snapshot = final_wire_snapshot
                .lock()
                .ok()
                .and_then(|mut slot| slot.take());
            // A provider without a cache-control anchor reports an empty CORE
            // prefix.  Preserve the provider's final-wire snapshot when it has
            // an anchor, otherwise fill the hash from the agent-owned
            // CORE/DEFERRED boundary captured before the call.
            if let Some(snapshot) = snapshot.as_mut() {
                if !snapshot.profile.is_enabled() || snapshot.core_tools_hash.is_empty() {
                    snapshot.core_tools_hash = core_tool_hash(tool_definitions);
                }
            }
            let (cache_ticket, _) = self.cache_observer.note_pre_call(CacheObserveInput {
                session_id: session_key,
                snapshot,
                break_reasons: &stable_prefix.cache_break_reasons(),
                expected_deletion: !microcompact.compacted.is_empty(),
            });
            match stream_result {
                Ok(stream) => {
                    return Ok(StartedModelStream {
                        stream,
                        cache_ticket,
                        assembly_total_estimated: assembly_report.total_estimated,
                        assembly_total_max: assembly_report.total_max,
                        requested_max_tokens,
                    });
                }
                Err(error) if !reactive_retry_attempted && is_context_overflow_error(&error) => {
                    self.cache_observer.abandon_call(cache_ticket);
                    warn!(
                        "Context overflow detected from provider: {}. Triggering reactive compaction...",
                        error
                    );
                    self.emit_agent_event(
                        message,
                        event_tx,
                        AgentEvent::ContextCompaction {
                            session_id: session_key.to_string(),
                            trigger: "reactive".to_string(),
                            phase: "started".to_string(),
                            summary: Some("provider context limit exceeded".to_string()),
                        },
                    );
                    reactive_retry_attempted = true;
                    self.rebuild_after_context_pressure(
                        messages,
                        turn_messages_start,
                        dynamic_sections,
                        current_turn_message,
                        session_key,
                        message,
                        event_tx,
                    )
                    .await?;
                    info!("Reactive checkpoint rebuild complete, retrying provider call...");
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

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn rebuild_after_context_pressure(
        &mut self,
        messages: &mut Vec<Message>,
        turn_messages_start: &mut usize,
        dynamic_sections: &[PromptSection],
        current_turn_message: &Message,
        session_key: &str,
        message: &InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let current_turn_snapshot =
            std::iter::once(provider_message_to_chat_message(current_turn_message))
                .flatten()
                .chain(
                    messages[*turn_messages_start..]
                        .iter()
                        .filter_map(provider_message_to_chat_message),
                )
                .collect::<Vec<_>>();
        let compact_result = if let Some(session) = self.sessions.get(session_key) {
            CheckpointCompactor::compact_snapshot(
                CheckpointSnapshot::from_session(session, current_turn_snapshot),
                &self.tool_config.budget,
                self.provider.clone(),
                &self.model,
                CheckpointTrigger::Reactive,
            )
            .await
        } else {
            Err(anyhow::anyhow!("Session not found for reactive compaction"))
        };

        let (history, canonical_checkpoint, current_turn_tail) = match compact_result {
            Ok(Some(pending)) => {
                self.emit_agent_event(
                    message,
                    event_tx,
                    AgentEvent::ContextCompaction {
                        session_id: session_key.to_string(),
                        trigger: "reactive".to_string(),
                        phase: "completed".to_string(),
                        summary: Some(format!(
                            "{} messages compressed",
                            pending.checkpoint.source_message_count
                        )),
                    },
                );
                let checkpoint = pending.checkpoint.clone();
                let mut active = pending.active_durable_messages.clone();
                let mut turn_tail = pending.active_current_turn_messages.clone();
                if turn_tail
                    .first()
                    .is_some_and(|message| message.role == "user")
                {
                    turn_tail.remove(0);
                }
                let provider_tail = turn_tail
                    .iter()
                    .filter_map(chat_message_to_provider_message)
                    .collect::<Vec<_>>();
                active.append(&mut turn_tail);
                self.pending_checkpoint_updates
                    .insert(session_key.to_string(), pending);
                (
                    agent_diva_core::session::align_chat_history(active),
                    Some(checkpoint),
                    provider_tail,
                )
            }
            Ok(None) => {
                self.emit_agent_event(
                    message,
                    event_tx,
                    AgentEvent::ContextCompaction {
                        session_id: session_key.to_string(),
                        trigger: "reactive".to_string(),
                        phase: "completed".to_string(),
                        summary: Some("nothing to compact; original context retained".to_string()),
                    },
                );
                let session = self.sessions.get_or_create(session_key);
                (
                    session.get_history(usize::MAX),
                    session.canonical_checkpoint.clone(),
                    messages[*turn_messages_start..].to_vec(),
                )
            }
            Err(error) => {
                warn!("Reactive compaction failed (non-blocking): {}", error);
                self.emit_agent_event(
                    message,
                    event_tx,
                    AgentEvent::ContextCompaction {
                        session_id: session_key.to_string(),
                        trigger: "reactive".to_string(),
                        phase: "failed".to_string(),
                        summary: Some(format!("{error}; original context retained")),
                    },
                );
                let session = self.sessions.get_or_create(session_key);
                (
                    session.get_history(usize::MAX),
                    session.canonical_checkpoint.clone(),
                    messages[*turn_messages_start..].to_vec(),
                )
            }
        };

        let stable_prefix = messages
            .first()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("reactive compaction cannot recover a stable prefix"))?;
        let mut prefix = self.context.build_prefix_messages_for_session(
            history,
            canonical_checkpoint.as_ref(),
            session_key,
            None,
        );
        if let Some(first) = prefix.first_mut() {
            *first = stable_prefix;
        }
        let mut prepared = PreparedTurnContext::prepare_budgeted(
            prefix,
            dynamic_sections,
            self.provider.dynamic_context_transport(),
            current_turn_message.clone(),
            &self.tool_config.budget,
        )?;
        if !current_turn_tail.is_empty() {
            prepared.messages.extend(current_turn_tail);
            ContextBuilder::sanitize_messages_for_provider(&mut prepared.messages);
        }
        *turn_messages_start = prepared.turn_messages_start;
        *messages = prepared.messages;
        Ok(())
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
                        super::super::publish_message_event(&self.bus, message, event);
                    }
                }
                LLMStreamEvent::ReasoningDelta(delta) => {
                    debug!("Stream ReasoningDelta: {:?}", delta);
                    streamed_reasoning.push_str(&delta);
                    let event = AgentEvent::ReasoningDelta { text: delta };
                    if let Some(tx) = event_tx {
                        let _ = tx.send(event.clone());
                    }
                    super::super::publish_message_event(&self.bus, message, event);
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
                        super::super::publish_message_event(&self.bus, message, event);
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
                super::super::publish_message_event(&self.bus, message, event);
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
    force_summary_only: bool,
    summary_followup_used: bool,
}

impl IterationBudget {
    pub(crate) fn can_continue(&self, max_iterations: usize) -> bool {
        self.completed < max_iterations + self.summary_bonus_remaining
    }

    pub(crate) fn begin_pass(&mut self, max_iterations: usize) -> IterationPass {
        self.completed += 1;
        let summary_only = self.force_summary_only || self.completed > max_iterations;
        if summary_only {
            self.summary_bonus_remaining = 0;
            self.force_summary_only = false;
            self.summary_followup_used = true;
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
        if self.summary_followup_used
            || self.force_summary_only
            || self.summary_bonus_remaining > 0
            || self.completed < max_iterations
        {
            return false;
        }
        self.summary_followup_used = true;
        self.summary_bonus_remaining = 1;
        true
    }

    /// Grant the single summary-only bonus before `max_iterations` is exhausted.
    ///
    /// Used when tools already ran and the follow-up sample returned no text.
    pub(crate) fn request_summary_followup(&mut self) -> bool {
        if self.summary_followup_used || self.force_summary_only || self.summary_bonus_remaining > 0
        {
            return false;
        }
        self.summary_followup_used = true;
        self.summary_bonus_remaining = 1;
        self.force_summary_only = true;
        true
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

fn provider_message_to_chat_message(message: &Message) -> Option<ChatMessage> {
    match message.role.as_str() {
        "user" | "assistant" | "tool" => {
            let tool_calls = message.tool_calls.as_ref().map(|calls| {
                calls
                    .iter()
                    .filter_map(|call| serde_json::to_value(call).ok())
                    .collect::<Vec<_>>()
            });
            Some(ChatMessage::with_tool_metadata(
                message.role.clone(),
                message.content.to_text_lossy(),
                message.tool_call_id.clone(),
                tool_calls,
                message.name.clone(),
            ))
        }
        _ => None,
    }
}

fn chat_message_to_provider_message(message: &ChatMessage) -> Option<Message> {
    match message.role.as_str() {
        "user" => Some(Message::user(&message.content)),
        "assistant" => {
            let mut provider = Message::assistant(&message.content);
            if let Some(calls) = message.tool_calls.as_ref() {
                provider.tool_calls =
                    serde_json::from_value(serde_json::Value::Array(calls.clone())).ok();
            }
            provider.reasoning_content = message.reasoning_content.clone();
            provider.thinking_blocks = message.thinking_blocks.clone();
            Some(provider)
        }
        "tool" => {
            let mut provider = Message::tool(
                &message.content,
                message.tool_call_id.clone().unwrap_or_default(),
            );
            provider.name = message.name.clone();
            Some(provider)
        }
        _ => None,
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

    #[test]
    fn request_summary_followup_forces_next_pass_before_max() {
        let mut budget = IterationBudget::default();
        assert_eq!(
            budget.begin_pass(20),
            IterationPass {
                index: 1,
                summary_only: false,
            }
        );
        assert!(budget.request_summary_followup());
        assert!(!budget.request_summary_followup());
        assert_eq!(
            budget.begin_pass(20),
            IterationPass {
                index: 2,
                summary_only: true,
            }
        );
        assert!(!budget.request_summary_followup());
    }

    fn empty_signals() -> EmptyFollowupSignals<'static> {
        EmptyFollowupSignals {
            content: None,
            finish_reason: "stop",
            prompt_tokens: 100,
            completion_tokens: 10,
            context_window: 128_000,
            assembly_total_estimated: 1_000,
            assembly_total_max: 180_000,
            requested_max_tokens: 4096,
            has_tool_results: true,
            summary_only: false,
        }
    }

    #[test]
    fn classify_ignores_non_empty_or_no_tools() {
        let mut signals = empty_signals();
        signals.content = Some("done");
        assert_eq!(classify_empty_followup(signals), None);

        let mut signals = empty_signals();
        signals.has_tool_results = false;
        assert_eq!(classify_empty_followup(signals), None);

        let mut signals = empty_signals();
        signals.summary_only = true;
        assert_eq!(classify_empty_followup(signals), None);
    }

    #[test]
    fn classify_input_pressure_from_usage_or_assembly() {
        let mut signals = empty_signals();
        signals.prompt_tokens = 120_000;
        signals.context_window = 128_000;
        assert_eq!(
            classify_empty_followup(signals),
            Some(EmptyFollowupKind::InputPressure)
        );

        let mut signals = empty_signals();
        signals.assembly_total_estimated = 200_000;
        signals.assembly_total_max = 180_000;
        assert_eq!(
            classify_empty_followup(signals),
            Some(EmptyFollowupKind::InputPressure)
        );
    }

    #[test]
    fn classify_output_truncated_from_finish_reason_or_completion_cap() {
        let mut signals = empty_signals();
        signals.finish_reason = "length";
        assert_eq!(
            classify_empty_followup(signals),
            Some(EmptyFollowupKind::OutputTruncated)
        );

        let mut signals = empty_signals();
        signals.completion_tokens = 4096;
        assert_eq!(
            classify_empty_followup(signals),
            Some(EmptyFollowupKind::OutputTruncated)
        );
    }

    #[test]
    fn classify_empty_stop_when_budget_remains() {
        assert_eq!(
            classify_empty_followup(empty_signals()),
            Some(EmptyFollowupKind::EmptyStop)
        );
    }

    #[test]
    fn classify_prefers_input_pressure_over_truncated() {
        let mut signals = empty_signals();
        signals.finish_reason = "length";
        signals.prompt_tokens = 126_000;
        signals.context_window = 128_000;
        assert_eq!(
            classify_empty_followup(signals),
            Some(EmptyFollowupKind::InputPressure)
        );
    }
}
