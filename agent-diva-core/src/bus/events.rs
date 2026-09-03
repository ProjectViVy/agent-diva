//! AgentEvent projection and lifecycle event types.

use crate::planning::model::{PlanPhase, PlanStatus, TodoPriority, TodoStatus};
use crate::planning::update_plan::UpdatePlanArgs;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::planning::ApprovalReceipt;
use crate::planning::PlanReportDetail;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRuntimeStep {
    pub id: String,
    pub ordinal: i32,
    pub title: String,
    pub rationale: Option<String>,
    pub expected_output: Option<String>,
    pub status: PlanStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRuntimeTodo {
    pub id: String,
    pub plan_step_id: Option<String>,
    pub title: String,
    pub detail: Option<String>,
    pub status: TodoStatus,
    pub priority: TodoPriority,
    pub evidence_ref: Option<String>,
    pub block_reason: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRuntimeState {
    pub plan_id: String,
    /// Frozen submission revision, when the active plan has been submitted.
    pub revision: Option<i64>,
    pub title: String,
    pub goal: String,
    pub phase: PlanPhase,
    pub status: PlanStatus,
    pub strategy: Option<String>,
    pub summary: String,
    pub steps: Vec<PlanRuntimeStep>,
    pub todos: Vec<PlanRuntimeTodo>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// The persisted plan state and immutable receipt returned after approval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanApprovalResult {
    pub plan: PlanRuntimeState,
    pub receipt: ApprovalReceipt,
}

/// Streaming events emitted by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// Observable per-session admission lifecycle transition.
    SessionAdmission {
        observation: SessionAdmissionObservation,
    },
    IterationStarted {
        index: usize,
        max_iterations: usize,
    },
    AssistantDelta {
        text: String,
    },
    ReasoningDelta {
        text: String,
    },
    ToolCallDelta {
        name: Option<String>,
        args_delta: String,
    },
    ToolCallStarted {
        name: String,
        args_preview: String,
        call_id: String,
    },
    ToolCallFinished {
        name: String,
        result: String,
        is_error: bool,
        call_id: String,
    },
    TodoCreated {
        plan: PlanRuntimeState,
        todo: PlanRuntimeTodo,
    },
    TodoStepUpdated {
        plan: PlanRuntimeState,
        todo: PlanRuntimeTodo,
    },
    TodoCompleted {
        plan: PlanRuntimeState,
        todo: PlanRuntimeTodo,
    },
    TodoCancelled {
        plan: PlanRuntimeState,
        todo: PlanRuntimeTodo,
    },
    PlanReadyForApproval {
        plan: PlanRuntimeState,
    },
    /// A canonical Markdown report produced by a Plan-mode exploration turn.
    PlanReportReadyForApproval {
        report: PlanReportDetail,
    },
    /// A lightweight plan update emitted during a normal chat turn.
    /// Carries a per-turn TODO list that is not persisted as part of Plan mode.
    ChatPlanUpdate {
        args: UpdatePlanArgs,
    },
    FinalResponse {
        content: String,
    },
    Error {
        message: String,
    },
    /// The provider reported a transient failure and is retrying with backoff.
    ProviderRetry {
        model: String,
        /// 1-based retry attempt number.
        attempt: u32,
        max_retries: u32,
        delay_ms: u64,
        reason: String,
    },
    /// No bus event for an extended period while a turn is still running.
    /// Emitted by the manager chat bridge as a non-terminal stall hint.
    ProviderStalled {
        model: Option<String>,
    },
    /// Automatic or reactive context compaction progress for one session.
    /// `summary` is present when the backend can provide a bounded result or
    /// failure explanation; the original session context remains authoritative
    /// when `phase` is `failed`.
    ContextCompaction {
        session_id: String,
        trigger: String,
        phase: String,
        summary: Option<String>,
    },
}

/// Stable machine-readable bounded-admission outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAdmissionCode {
    SessionQueueFull,
    SessionQueueWaitTimeout,
    SessionTurnCancelled,
    SessionReset,
    SessionWorkerUnavailable,
    SessionSlotEvicted,
}

/// Observable stage of one request's admission lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAdmissionPhase {
    Queued,
    Running,
    Rejected,
    Cancelled,
    Reset,
    Unavailable,
    Evicted,
}

/// Correlated admission state projected to every streaming surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionAdmissionObservation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<SessionAdmissionCode>,
    pub phase: SessionAdmissionPhase,
    pub session_key: String,
    pub request_id: String,
    pub trace_id: String,
    pub queue_depth: usize,
    pub wait_latency_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionControlAction {
    Stop,
    Reset,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionControlTargetState {
    Running,
    QueuedPreserved,
    Session,
    Absent,
}

/// Typed result returned after a runtime session-control decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionControlOutcome {
    pub action: SessionControlAction,
    pub session_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<SessionAdmissionCode>,
    pub target_state: SessionControlTargetState,
    pub running_cancelled: bool,
    pub queued_cancelled: usize,
    pub cleanup_complete: bool,
}

/// Event with context for the bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBusEvent {
    pub channel: String,
    pub chat_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    pub event: AgentEvent,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::update_plan::{PlanItem, PlanItemStatus, UpdatePlanArgs};

    fn sample_update_plan_args() -> UpdatePlanArgs {
        UpdatePlanArgs {
            explanation: Some("Normal-chat plan update".to_string()),
            plan: vec![
                PlanItem {
                    step: "Analyze user request".to_string(),
                    status: PlanItemStatus::Completed,
                },
                PlanItem {
                    step: "Draft the response".to_string(),
                    status: PlanItemStatus::InProgress,
                },
                PlanItem {
                    step: "Review before sending".to_string(),
                    status: PlanItemStatus::Pending,
                },
            ],
        }
    }

    #[test]
    fn chat_plan_update_serde_roundtrip() {
        let original = AgentEvent::ChatPlanUpdate {
            args: sample_update_plan_args(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: AgentEvent = serde_json::from_str(&json).unwrap();

        match back {
            AgentEvent::ChatPlanUpdate { args } => {
                assert_eq!(args, sample_update_plan_args());
            }
            other => panic!("expected ChatPlanUpdate, got {:?}", other),
        }
    }

    #[test]
    fn chat_plan_update_nested_in_bus_event() {
        let original = AgentBusEvent {
            channel: "telegram".to_string(),
            chat_id: "12345".to_string(),
            session_key: None,
            request_id: None,
            trace_id: None,
            event: AgentEvent::ChatPlanUpdate {
                args: sample_update_plan_args(),
            },
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: AgentBusEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(back.channel, "telegram");
        assert_eq!(back.chat_id, "12345");
        match back.event {
            AgentEvent::ChatPlanUpdate { args } => {
                assert_eq!(args, sample_update_plan_args());
            }
            other => panic!("expected ChatPlanUpdate, got {:?}", other),
        }
    }

    #[test]
    fn provider_retry_serde_roundtrip() {
        let original = AgentEvent::ProviderRetry {
            model: "deepseek-chat".to_string(),
            attempt: 2,
            max_retries: 3,
            delay_ms: 2000,
            reason: "unexpected EOF during handshake".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: AgentEvent = serde_json::from_str(&json).unwrap();

        match back {
            AgentEvent::ProviderRetry {
                model,
                attempt,
                max_retries,
                delay_ms,
                reason,
            } => {
                assert_eq!(model, "deepseek-chat");
                assert_eq!(attempt, 2);
                assert_eq!(max_retries, 3);
                assert_eq!(delay_ms, 2000);
                assert_eq!(reason, "unexpected EOF during handshake");
            }
            other => panic!("expected ProviderRetry, got {:?}", other),
        }
    }

    #[test]
    fn provider_stalled_serde_roundtrip() {
        let original = AgentEvent::ProviderStalled {
            model: Some("deepseek-chat".to_string()),
        };
        let json = serde_json::to_string(&original).unwrap();
        let back: AgentEvent = serde_json::from_str(&json).unwrap();

        match back {
            AgentEvent::ProviderStalled { model } => {
                assert_eq!(model.as_deref(), Some("deepseek-chat"));
            }
            other => panic!("expected ProviderStalled, got {:?}", other),
        }
    }

    #[test]
    fn context_compaction_serde_roundtrip() {
        let original = AgentEvent::ContextCompaction {
            session_id: "gui:chat".into(),
            trigger: "reactive".into(),
            phase: "failed".into(),
            summary: Some("original context retained".into()),
        };
        let back: AgentEvent =
            serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
        assert!(
            matches!(back, AgentEvent::ContextCompaction { session_id, trigger, phase, summary }
            if session_id == "gui:chat" && trigger == "reactive" && phase == "failed"
                && summary.as_deref() == Some("original context retained"))
        );
    }
}

/// Lifecycle and audit events for the poke 8 event chain.
///
/// These events supplement the streaming `AgentEvent` variants with
/// system-level lifecycle events used for audit, monitoring, and
/// cross-module communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PokeEvent {
    /// A poke/message is about to be sent.
    PokeSend { message: String },
    /// A chat message is being sent outbound.
    ChatSend { content: String },
    /// A chat message was successfully sent.
    ChatSent { content: String, message_id: String },
    /// Identity-only user activity; message content is never exposed.
    UserActivity {
        session_key: String,
        sender_id: Option<String>,
    },
    /// Reasoning/thought content received from the LLM.
    ReasoningReceived { content: String, model: String },
    /// A chat turn or conversation ended.
    ChatOver { reason: String },
    /// Messages added to persistent chat history.
    ChatHistoryAdd { message_ids: Vec<String> },
    /// Token consumption event emitted by providers.
    TokenUsed {
        tokens: u32,
        model: String,
        provider: String,
    },
    /// Config hot-reload: some fields changed but require restart.
    /// Emitted so the GUI can show a "restart required" indicator.
    ConfigChangeNeedsRestart {
        /// Dot-separated field paths that require restart.
        fields: Vec<String>,
    },
}

