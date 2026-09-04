//! AgentEvent to Neuro-Link projection fan-out.
//!
//! The AgentLoop remains the owner of execution and the AgentEventBus remains a
//! compatibility observation surface.  This module gives the typed gateway a
//! single, process-wide projection stream: correlated AgentBusEvents are
//! converted into bounded Fabric envelopes, appended to the profile-local
//! journal, and broadcast to live WebSocket clients.  Transient stream deltas
//! are intentionally not replayed after reconnect; durable lifecycle and state
//! changes are.

use agent_diva_core::bus::{AgentBusEvent, AgentEvent, AgentEventBus, PlanRuntimeState};
use agent_diva_core::channel::{
    ChannelAddress, ChannelDirection, ChannelEnvelopeV1, ChannelOrigin, ChannelPayloadV1,
    ChannelRuntimeProjectionV1, ContentPart, Correlation, ProjectionEventV1, StreamPhase,
};
use agent_diva_core::planning::PlanReportDetail;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::{broadcast, mpsc};

use crate::projection_journal::ProjectionJournal;

const BROADCAST_CAPACITY: usize = 512;
const MAX_TEXT_CHARS: usize = 8 * 1024;
const MAX_PREVIEW_CHARS: usize = 512;

/// Process-wide projection hub shared by all Neuro-Link sockets.
#[derive(Clone)]
pub struct NeuroLinkProjectionHub {
    submit_tx: mpsc::UnboundedSender<ChannelRuntimeProjectionV1>,
    events_tx: broadcast::Sender<ProjectionEventV1>,
}

impl NeuroLinkProjectionHub {
    /// Start one journal/broadcast pump for the supplied AgentEventBus.
    ///
    /// AppState is also used by synchronous handler fixtures.  In that case
    /// there is no Tokio runtime to host a pump, so the hub remains inert
    /// rather than panicking during fixture construction.
    pub fn new(bus: AgentEventBus, data_root: impl Into<PathBuf>) -> Arc<Self> {
        let (submit_tx, submit_rx) = mpsc::unbounded_channel();
        let (events_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        let hub = Arc::new(Self {
            submit_tx,
            events_tx: events_tx.clone(),
        });
        if let Ok(handle) = Handle::try_current() {
            handle.spawn(run_projection_pump(
                bus.subscribe_events(),
                submit_rx,
                data_root.into(),
                events_tx,
            ));
        }
        hub
    }

    /// Subscribe to journaled live projections.
    pub fn subscribe(&self) -> broadcast::Receiver<ProjectionEventV1> {
        self.events_tx.subscribe()
    }

    /// Submit a pre-built projection for persistence and live fan-out.
    ///
    /// The AgentLoop path normally enters through the AgentEventBus observer; a
    /// direct submitter is retained for future Fabric producers and makes the
    /// boundary explicit without exposing the SQLite handle to them.
    pub fn submit(
        &self,
        projection: ChannelRuntimeProjectionV1,
    ) -> Result<(), Box<ChannelRuntimeProjectionV1>> {
        self.submit_tx
            .send(projection)
            .map_err(|error| Box::new(error.0))
    }
}

async fn run_projection_pump(
    mut bus_rx: broadcast::Receiver<AgentBusEvent>,
    mut submit_rx: mpsc::UnboundedReceiver<ChannelRuntimeProjectionV1>,
    data_root: PathBuf,
    events_tx: broadcast::Sender<ProjectionEventV1>,
) {
    let journal = match ProjectionJournal::open(data_root).await {
        Ok(journal) => journal,
        Err(error) => {
            tracing::error!(%error, "Neuro-Link projection journal failed to open");
            return;
        }
    };
    let mut presentation_state = PresentationState::default();
    loop {
        tokio::select! {
            result = bus_rx.recv() => match result {
                Ok(event) => {
                    for projection in project_presentation_events(&event, &mut presentation_state) {
                        persist_and_broadcast(&journal, &events_tx, projection).await;
                    }
                    if let Some(projection) = project_bus_event(event) {
                        persist_and_broadcast(&journal, &events_tx, projection).await;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(skipped, "Neuro-Link projection hub lagged on AgentEvent bus");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            projection = submit_rx.recv() => match projection {
                Some(projection) => persist_and_broadcast(&journal, &events_tx, projection).await,
                None => break,
            },
        }
    }
}

#[derive(Debug, Default)]
struct PresentationState {
    turns: HashMap<(String, String), TurnPresentationState>,
}

#[derive(Debug, Default)]
struct TurnPresentationState {
    thinking: bool,
    speaking: bool,
    tools: HashSet<String>,
}

impl PresentationState {
    fn turn_mut(&mut self, event: &AgentBusEvent) -> Option<&mut TurnPresentationState> {
        let session_key = event.session_key.as_deref()?.trim();
        let request_id = event.request_id.as_deref()?.trim();
        if session_key.is_empty() || request_id.is_empty() {
            return None;
        }
        Some(
            self.turns
                .entry((session_key.to_string(), request_id.to_string()))
                .or_default(),
        )
    }

    fn clear_turn(&mut self, event: &AgentBusEvent) {
        if let (Some(session_key), Some(request_id)) =
            (event.session_key.as_deref(), event.request_id.as_deref())
        {
            self.turns
                .remove(&(session_key.to_string(), request_id.to_string()));
        }
    }
}

fn project_presentation_events(
    event: &AgentBusEvent,
    state: &mut PresentationState,
) -> Vec<ChannelRuntimeProjectionV1> {
    if event.channel != "neuro-link"
        || event
            .session_key
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
        || event
            .request_id
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
        || event
            .trace_id
            .as_deref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
    {
        return Vec::new();
    }
    let mut output = Vec::new();
    let Some(turn) = state.turn_mut(event) else {
        return output;
    };
    let emit = |event_name: &str, body: Value, durable: bool, terminal: bool| {
        presentation_projection(event, event_name, body, durable, terminal)
    };
    let mut finish_thinking = |output: &mut Vec<ChannelRuntimeProjectionV1>| {
        if turn.thinking {
            output.push(emit(
                "assistant.thinking.completed",
                json!({}),
                false,
                false,
            ));
            turn.thinking = false;
        }
    };
    let mut finish_speaking = |output: &mut Vec<ChannelRuntimeProjectionV1>| {
        if turn.speaking {
            output.push(emit(
                "assistant.speaking.completed",
                json!({}),
                false,
                false,
            ));
            turn.speaking = false;
        }
    };
    match &event.event {
        AgentEvent::SessionAdmission { observation }
            if matches!(
                observation.phase,
                agent_diva_core::bus::SessionAdmissionPhase::Queued
                    | agent_diva_core::bus::SessionAdmissionPhase::Running
            ) =>
        {
            output.push(emit("subtitle.cleared", json!({}), false, false));
        }
        AgentEvent::ReasoningDelta { .. } => {
            if !turn.thinking {
                output.push(emit("assistant.thinking.started", json!({}), false, false));
                turn.thinking = true;
            }
        }
        AgentEvent::AssistantDelta { .. } => {
            finish_thinking(&mut output);
            if !turn.speaking {
                output.push(emit("assistant.speaking.started", json!({}), false, false));
                turn.speaking = true;
            }
        }
        AgentEvent::ToolCallStarted { name, call_id, .. } => {
            finish_thinking(&mut output);
            finish_speaking(&mut output);
            if turn.tools.insert(call_id.clone()) {
                output.push(emit(
                    "assistant.tool.started",
                    json!({ "name": bounded(name, MAX_PREVIEW_CHARS), "call_id": bounded(call_id, MAX_PREVIEW_CHARS) }),
                    false,
                    false,
                ));
            }
        }
        AgentEvent::ToolCallFinished {
            name,
            call_id,
            is_error,
            ..
        } => {
            if turn.tools.remove(call_id) {
                output.push(emit(
                    "assistant.tool.completed",
                    json!({ "name": bounded(name, MAX_PREVIEW_CHARS), "call_id": bounded(call_id, MAX_PREVIEW_CHARS), "is_error": is_error }),
                    false,
                    false,
                ));
            }
        }
        AgentEvent::PlanReadyForApproval { plan } => {
            finish_thinking(&mut output);
            finish_speaking(&mut output);
            output.push(emit(
                "assistant.waiting_for_approval",
                json!({ "kind": "plan", "plan_id": plan.plan_id, "revision": plan.revision }),
                true,
                false,
            ));
        }
        AgentEvent::PlanReportReadyForApproval { report } => {
            finish_thinking(&mut output);
            finish_speaking(&mut output);
            output.push(emit(
                "assistant.waiting_for_approval",
                json!({ "kind": "plan_report", "report_id": report.report.id, "revision": report.revision.revision }),
                true,
                false,
            ));
        }
        AgentEvent::FinalResponse { content } => {
            finish_thinking(&mut output);
            finish_speaking(&mut output);
            for call_id in std::mem::take(&mut turn.tools) {
                output.push(emit(
                    "assistant.tool.completed",
                    json!({ "call_id": bounded(&call_id, MAX_PREVIEW_CHARS), "is_error": true }),
                    false,
                    false,
                ));
            }
            output.push(emit(
                "subtitle.updated",
                json!({ "text": bounded(content, MAX_TEXT_CHARS) }),
                true,
                false,
            ));
            state.clear_turn(event);
        }
        AgentEvent::Error { .. }
        | AgentEvent::SessionAdmission {
            observation:
                agent_diva_core::bus::SessionAdmissionObservation {
                    phase:
                        agent_diva_core::bus::SessionAdmissionPhase::Rejected
                        | agent_diva_core::bus::SessionAdmissionPhase::Cancelled
                        | agent_diva_core::bus::SessionAdmissionPhase::Reset
                        | agent_diva_core::bus::SessionAdmissionPhase::Unavailable
                        | agent_diva_core::bus::SessionAdmissionPhase::Evicted,
                    ..
                },
        } => {
            finish_thinking(&mut output);
            finish_speaking(&mut output);
            turn.tools.clear();
            output.push(emit("subtitle.cleared", json!({}), false, false));
            if matches!(event.event, AgentEvent::Error { .. }) {
                state.clear_turn(event);
            }
        }
        _ => {}
    }
    output
}

fn presentation_projection(
    event: &AgentBusEvent,
    event_name: &str,
    body: Value,
    durable: bool,
    terminal: bool,
) -> ChannelRuntimeProjectionV1 {
    let session_key = event.session_key.clone().unwrap_or_default();
    let request_id = event.request_id.clone().unwrap_or_default();
    let trace_id = event.trace_id.clone().unwrap_or_default();
    let mut correlation = Correlation::new(session_key);
    correlation.request_id = Some(request_id);
    correlation.trace_id = Some(trace_id);
    ChannelRuntimeProjectionV1 {
        method: "presentation/event".to_string(),
        envelope: ChannelEnvelopeV1::new(
            ChannelDirection::InternalProjection,
            ChannelAddress::new("neuro-link", event.chat_id.clone()),
            correlation,
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Presentation {
                event: event_name.to_string(),
                body,
            },
        ),
        durable,
        terminal,
    }
}

async fn persist_and_broadcast(
    journal: &ProjectionJournal,
    events_tx: &broadcast::Sender<ProjectionEventV1>,
    projection: ChannelRuntimeProjectionV1,
) {
    if let Err(error) = projection.envelope.validate() {
        tracing::warn!(%error, method = %projection.method, "discarding invalid Neuro-Link projection");
        return;
    }
    let ChannelRuntimeProjectionV1 {
        method,
        envelope,
        durable,
        terminal,
    } = projection;
    let session_key = envelope.correlation.session_key.clone();
    match journal
        .append_event(&session_key, method, envelope, durable, terminal)
        .await
    {
        Ok(event) => {
            let _ = events_tx.send(event);
        }
        Err(error) => tracing::warn!(%error, "failed to append Neuro-Link projection"),
    }
}

fn project_bus_event(event: AgentBusEvent) -> Option<ChannelRuntimeProjectionV1> {
    // The gateway is a typed Neuro-Link surface.  Other channels retain their
    // historical bus subscriptions and must not leak into this profile stream.
    if event.channel != "neuro-link" {
        return None;
    }
    let session_key = event.session_key?.trim().to_owned();
    let request_id = event.request_id?.trim().to_owned();
    let trace_id = event.trace_id?.trim().to_owned();
    if session_key.is_empty() || request_id.is_empty() || trace_id.is_empty() {
        return None;
    }

    // The gateway writes the first queued/running admission projection
    // synchronously with `turn/start`.  Skip those duplicate bus observations
    // while retaining rejected/cancelled/unavailable terminal transitions.
    let (method, payload, durable, terminal) = match event.event {
        AgentEvent::SessionAdmission { observation }
            if matches!(
                observation.phase,
                agent_diva_core::bus::SessionAdmissionPhase::Queued
                    | agent_diva_core::bus::SessionAdmissionPhase::Running
            ) =>
        {
            return None
        }
        AgentEvent::SessionAdmission { observation } => (
            "turn/admission",
            control_payload("turn/admission", serde_json::to_value(&observation).ok()?),
            true,
            matches!(
                observation.phase,
                agent_diva_core::bus::SessionAdmissionPhase::Rejected
                    | agent_diva_core::bus::SessionAdmissionPhase::Cancelled
                    | agent_diva_core::bus::SessionAdmissionPhase::Reset
                    | agent_diva_core::bus::SessionAdmissionPhase::Unavailable
                    | agent_diva_core::bus::SessionAdmissionPhase::Evicted
            ),
        ),
        AgentEvent::IterationStarted {
            index,
            max_iterations,
        } => (
            "turn/iteration",
            ChannelPayloadV1::Control {
                operation: "turn/iteration".to_string(),
                body: json!({"index": index, "max_iterations": max_iterations}),
            },
            false,
            false,
        ),
        AgentEvent::AssistantDelta { text } => (
            "conversation/stream",
            stream_payload(StreamPhase::Delta, text),
            false,
            false,
        ),
        AgentEvent::ReasoningDelta { text } => (
            "conversation/reasoning",
            stream_payload(StreamPhase::Delta, text),
            false,
            false,
        ),
        AgentEvent::ToolCallDelta { name, args_delta } => (
            "tool/lifecycle",
            control_payload(
                "tool/delta",
                json!({
                    "name": name,
                    "args_delta": bounded(&args_delta, MAX_TEXT_CHARS),
                }),
            ),
            false,
            false,
        ),
        AgentEvent::ToolCallStarted {
            name,
            args_preview,
            call_id,
        } => (
            "tool/lifecycle",
            control_payload(
                "tool/started",
                json!({
                    "name": bounded(&name, MAX_PREVIEW_CHARS),
                    "call_id": bounded(&call_id, MAX_PREVIEW_CHARS),
                    "args_preview": bounded(&args_preview, MAX_PREVIEW_CHARS),
                }),
            ),
            false,
            false,
        ),
        AgentEvent::ToolCallFinished {
            name,
            is_error,
            call_id,
            ..
        } => (
            "tool/lifecycle",
            control_payload(
                "tool/finished",
                json!({
                    "name": bounded(&name, MAX_PREVIEW_CHARS),
                    "call_id": bounded(&call_id, MAX_PREVIEW_CHARS),
                    "is_error": is_error,
                }),
            ),
            true,
            false,
        ),
        AgentEvent::TodoCreated { plan, todo }
        | AgentEvent::TodoStepUpdated { plan, todo }
        | AgentEvent::TodoCompleted { plan, todo }
        | AgentEvent::TodoCancelled { plan, todo } => (
            "planning/changed",
            control_payload(
                "todo/changed",
                json!({
                    "plan": plan_summary(&plan),
                    "todo": {
                        "id": todo.id,
                        "title": bounded(&todo.title, MAX_PREVIEW_CHARS),
                        "status": todo.status,
                        "priority": todo.priority,
                    },
                }),
            ),
            true,
            false,
        ),
        AgentEvent::PlanReadyForApproval { plan } => (
            "planning/changed",
            control_payload("plan/ready_for_approval", plan_summary(&plan)),
            true,
            false,
        ),
        AgentEvent::PlanReportReadyForApproval { report } => (
            "planning/changed",
            control_payload("plan_report/ready_for_approval", report_summary(&report)),
            true,
            false,
        ),
        AgentEvent::ChatPlanUpdate { args } => (
            "planning/changed",
            control_payload(
                "chat_plan/changed",
                json!({
                    "explanation": args.explanation.map(|value| bounded(&value, MAX_PREVIEW_CHARS)),
                    "plan": args.plan.into_iter().map(|item| json!({
                        "step": bounded(&item.step, MAX_PREVIEW_CHARS),
                        "status": item.status,
                    })).collect::<Vec<_>>(),
                }),
            ),
            false,
            false,
        ),
        AgentEvent::FinalResponse { content } => (
            "conversation/stream",
            stream_payload(StreamPhase::Finalized, content),
            true,
            true,
        ),
        AgentEvent::Error { message } => (
            "conversation/stream",
            stream_payload(StreamPhase::Failed, message),
            true,
            true,
        ),
        AgentEvent::ProviderRetry {
            model,
            attempt,
            max_retries,
            delay_ms,
            reason,
        } => (
            "provider/status",
            control_payload(
                "provider/retry",
                json!({
                    "model": bounded(&model, MAX_PREVIEW_CHARS),
                    "attempt": attempt,
                    "max_retries": max_retries,
                    "delay_ms": delay_ms,
                    "reason": bounded(&reason, MAX_PREVIEW_CHARS),
                }),
            ),
            false,
            false,
        ),
        AgentEvent::ProviderStalled { model } => (
            "provider/status",
            control_payload(
                "provider/stalled",
                json!({"model": model.map(|value| bounded(&value, MAX_PREVIEW_CHARS))}),
            ),
            false,
            false,
        ),
        AgentEvent::ContextCompaction {
            session_id,
            trigger,
            phase,
            summary,
        } => (
            "context/compaction",
            control_payload(
                "context/compaction",
                json!({
                    "session_id": bounded(&session_id, MAX_PREVIEW_CHARS),
                    "trigger": bounded(&trigger, MAX_PREVIEW_CHARS),
                    "phase": bounded(&phase, MAX_PREVIEW_CHARS),
                    "summary": summary.map(|value| bounded(&value, MAX_PREVIEW_CHARS)),
                }),
            ),
            true,
            false,
        ),
    };

    let mut correlation = Correlation::new(session_key.clone());
    correlation.request_id = Some(request_id);
    correlation.trace_id = Some(trace_id);
    Some(ChannelRuntimeProjectionV1 {
        method: method.to_string(),
        envelope: ChannelEnvelopeV1::new(
            ChannelDirection::InternalProjection,
            ChannelAddress::new("neuro-link", event.chat_id),
            correlation,
            ChannelOrigin::Runtime,
            payload,
        ),
        durable,
        terminal,
    })
}

fn stream_payload(phase: StreamPhase, text: String) -> ChannelPayloadV1 {
    ChannelPayloadV1::Stream {
        phase,
        parts: vec![ContentPart::Text {
            text: bounded(&text, MAX_TEXT_CHARS),
        }],
    }
}

fn control_payload(operation: &str, body: Value) -> ChannelPayloadV1 {
    ChannelPayloadV1::Control {
        operation: operation.to_string(),
        body,
    }
}

fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn plan_summary(plan: &PlanRuntimeState) -> Value {
    json!({
        "plan_id": plan.plan_id,
        "revision": plan.revision,
        "title": bounded(&plan.title, MAX_PREVIEW_CHARS),
        "phase": plan.phase,
        "status": plan.status,
        "steps": plan.steps.iter().map(|step| json!({
            "id": step.id,
            "ordinal": step.ordinal,
            "title": bounded(&step.title, MAX_PREVIEW_CHARS),
            "status": step.status,
        })).collect::<Vec<_>>(),
        "todos": plan.todos.iter().map(|todo| json!({
            "id": todo.id,
            "title": bounded(&todo.title, MAX_PREVIEW_CHARS),
            "status": todo.status,
            "priority": todo.priority,
        })).collect::<Vec<_>>(),
    })
}

fn report_summary(report: &PlanReportDetail) -> Value {
    json!({
        "report_id": report.report.id,
        "session_key": report.report.session_key,
        "revision": report.revision.revision,
        "title": bounded(&report.revision.title, MAX_PREVIEW_CHARS),
        "status": report.report.status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::bus::{AgentEvent, SessionAdmissionObservation, SessionAdmissionPhase};

    fn bus_event(event: AgentEvent) -> AgentBusEvent {
        AgentBusEvent {
            channel: "neuro-link".to_string(),
            chat_id: "chat-1".to_string(),
            session_key: Some("session-1".to_string()),
            request_id: Some("request-1".to_string()),
            trace_id: Some("trace-1".to_string()),
            event,
        }
    }

    #[test]
    fn correlated_agent_events_become_bounded_fabric_projections() {
        let projection = project_bus_event(bus_event(AgentEvent::AssistantDelta {
            text: "hello".to_string(),
        }))
        .unwrap();
        assert_eq!(projection.method, "conversation/stream");
        assert!(!projection.durable);
        assert!(!projection.terminal);
        assert_eq!(projection.envelope.correlation.session_key, "session-1");
    }

    #[test]
    fn terminal_events_are_durable_and_sensitive_tool_results_are_omitted() {
        let projection = project_bus_event(bus_event(AgentEvent::FinalResponse {
            content: "done".to_string(),
        }))
        .unwrap();
        assert!(projection.durable);
        assert!(projection.terminal);

        let tool = project_bus_event(bus_event(AgentEvent::ToolCallFinished {
            name: "shell".to_string(),
            result: "secret output".to_string(),
            is_error: false,
            call_id: "call-1".to_string(),
        }))
        .unwrap();
        let json = serde_json::to_value(tool.envelope).unwrap();
        assert!(!json.to_string().contains("secret output"));
    }

    #[test]
    fn queued_and_running_admission_are_owned_by_turn_start_response() {
        assert!(project_bus_event(bus_event(AgentEvent::SessionAdmission {
            observation: SessionAdmissionObservation {
                code: None,
                phase: SessionAdmissionPhase::Running,
                session_key: "session-1".to_string(),
                request_id: "request-1".to_string(),
                trace_id: "trace-1".to_string(),
                queue_depth: 0,
                wait_latency_ms: 0,
            },
        }))
        .is_none());
    }

    #[test]
    fn rejected_admission_is_a_durable_terminal_projection() {
        let projection = project_bus_event(bus_event(AgentEvent::SessionAdmission {
            observation: SessionAdmissionObservation {
                code: Some(agent_diva_core::bus::SessionAdmissionCode::SessionQueueFull),
                phase: SessionAdmissionPhase::Rejected,
                session_key: "session-1".to_string(),
                request_id: "request-1".to_string(),
                trace_id: "trace-1".to_string(),
                queue_depth: 64,
                wait_latency_ms: 0,
            },
        }))
        .unwrap();
        assert!(projection.durable);
        assert!(projection.terminal);
    }

    #[test]
    fn presentation_lifecycle_is_semantic_and_ordered() {
        let mut state = PresentationState::default();

        let thinking = project_presentation_events(
            &bus_event(AgentEvent::ReasoningDelta {
                text: "considering".to_string(),
            }),
            &mut state,
        );
        assert_eq!(thinking.len(), 1);
        assert_presentation_event(&thinking[0], "assistant.thinking.started");

        let duplicate_thinking = project_presentation_events(
            &bus_event(AgentEvent::ReasoningDelta {
                text: "more".to_string(),
            }),
            &mut state,
        );
        assert!(duplicate_thinking.is_empty());

        let speaking = project_presentation_events(
            &bus_event(AgentEvent::AssistantDelta {
                text: "answer".to_string(),
            }),
            &mut state,
        );
        assert_eq!(speaking.len(), 2);
        assert_presentation_event(&speaking[0], "assistant.thinking.completed");
        assert_presentation_event(&speaking[1], "assistant.speaking.started");

        let terminal = project_presentation_events(
            &bus_event(AgentEvent::FinalResponse {
                content: "final answer".to_string(),
            }),
            &mut state,
        );
        assert_eq!(terminal.len(), 2);
        assert_presentation_event(&terminal[0], "assistant.speaking.completed");
        assert_presentation_event(&terminal[1], "subtitle.updated");
        assert!(!terminal[0].durable);
        assert!(terminal[1].durable);
        assert!(matches!(
            &terminal[1].envelope.payload,
            ChannelPayloadV1::Presentation { body, .. }
                if body.get("text").and_then(Value::as_str) == Some("final answer")
        ));
    }

    #[test]
    fn accepted_admission_clears_previous_subtitle() {
        let mut state = PresentationState::default();
        let projections = project_presentation_events(
            &bus_event(AgentEvent::SessionAdmission {
                observation: SessionAdmissionObservation {
                    code: None,
                    phase: SessionAdmissionPhase::Running,
                    session_key: "session-1".to_string(),
                    request_id: "request-1".to_string(),
                    trace_id: "trace-1".to_string(),
                    queue_depth: 0,
                    wait_latency_ms: 0,
                },
            }),
            &mut state,
        );
        assert_eq!(projections.len(), 1);
        assert_presentation_event(&projections[0], "subtitle.cleared");
        assert!(!projections[0].durable);
    }

    fn assert_presentation_event(projection: &ChannelRuntimeProjectionV1, expected: &str) {
        assert_eq!(projection.method, "presentation/event");
        assert!(matches!(
            &projection.envelope.payload,
            ChannelPayloadV1::Presentation { event, .. } if event == expected
        ));
    }

    #[tokio::test]
    async fn bus_events_are_persisted_once_and_broadcast_to_live_subscribers() {
        let temp = tempfile::tempdir().unwrap();
        let bus = AgentEventBus::new();
        let hub = NeuroLinkProjectionHub::new(bus.clone(), temp.path());
        let mut events = hub.subscribe();
        bus.publish_correlated_event(
            "neuro-link",
            "chat-1",
            "session-1",
            "request-1",
            "trace-1",
            AgentEvent::FinalResponse {
                content: "complete".to_string(),
            },
        )
        .unwrap();

        let live = tokio::time::timeout(std::time::Duration::from_secs(3), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(live.cursor.sequence, 1);
        assert_eq!(live.method, "presentation/event");
        assert!(live.envelope.correlation.sequence.is_some());
        let second_live = tokio::time::timeout(std::time::Duration::from_secs(3), events.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(second_live.cursor.sequence, 2);
        assert_eq!(second_live.method, "conversation/stream");

        let journal = ProjectionJournal::open(temp.path()).await.unwrap();
        let sync = journal
            .replay(
                "session-1",
                Some(&agent_diva_core::channel::CursorV1 {
                    stream: "session-1".to_string(),
                    sequence: 0,
                }),
            )
            .await
            .unwrap();
        assert_eq!(sync.events.len(), 2);
        assert_eq!(sync.events[0].method, "presentation/event");
        assert_eq!(sync.events[1].method, "conversation/stream");
    }
}
