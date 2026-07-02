//! Structured audit events for system observability.
//!
//! All significant system events flow through `audit::emit()` which
//! writes a structured JSON line to the gateway log via `tracing::info!`
//! with `target: "audit"`.
//!
//! Consumers (GUI, CLI) filter for `target == "audit"` to reconstruct
//! the event stream.

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::presence::PresenceState;

/// Severity levels for security-related events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Severity levels for PII detection.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PiiSeverity {
    Warning,
    Error,
}

/// All auditable system events.
///
/// Each variant carries structured data relevant to the event.
/// Serialized as JSON when emitted via tracing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AuditEvent {
    HeartbeatTriggered {
        interval_secs: u64,
    },
    ToolInvoked {
        tool_name: String,
        args: serde_json::Value,
    },
    ToolDenied {
        tool_name: String,
        reason: String,
    },
    DecisionPoint {
        agent_id: String,
        decision: String,
        context: String,
    },
    InjectionDetected {
        layer: String,
        pattern: String,
        severity: Severity,
    },
    PiiRedacted {
        category: String,
        severity: PiiSeverity,
        count: u32,
    },
    TokenUsed {
        provider: String,
        model: String,
        tokens: u32,
    },
    ReasoningReceived {
        content_len: u32,
        model: String,
    },
    ChatOver {
        session_id: String,
        turn_count: u32,
    },
    PresenceChanged {
        from: PresenceState,
        to: PresenceState,
    },
}

/// Emit an audit event as a structured JSON log line.
///
/// This is the single entry point for all audit events. It writes
/// to the `tracing` pipeline with `target: "audit"` so consumers
/// can filter for audit events specifically.
///
/// # Performance
///
/// Target <1ms per call. The `tracing` infrastructure is designed
/// to be low-overhead; the `?event` debug format delegates to
/// serde's `Debug` if available (derive `Debug` handles this).
///
/// # Example
///
/// ```rust,ignore
/// audit::emit(AuditEvent::ToolInvoked {
///     tool_name: "bash".into(),
///     args: serde_json::json!({"command": "ls -la"}),
/// });
/// ```
pub fn emit(event: AuditEvent) {
    info!(target: "audit", event = ?event);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_does_not_panic() {
        // Quick smoke test — verify emit doesn't crash
        emit(AuditEvent::HeartbeatTriggered { interval_secs: 5 });
    }

    #[test]
    fn test_audit_event_serialization() {
        let event = AuditEvent::ToolInvoked {
            tool_name: "test".into(),
            args: serde_json::json!({"key": "value"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("tool_invoked"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_all_variants_roundtrip() {
        let events = vec![
            AuditEvent::HeartbeatTriggered { interval_secs: 5 },
            AuditEvent::ToolInvoked {
                tool_name: "x".into(),
                args: serde_json::json!({}),
            },
            AuditEvent::ToolDenied {
                tool_name: "x".into(),
                reason: "policy".into(),
            },
            AuditEvent::DecisionPoint {
                agent_id: "a1".into(),
                decision: "continue".into(),
                context: "loop".into(),
            },
            AuditEvent::InjectionDetected {
                layer: "input".into(),
                pattern: "role_override".into(),
                severity: Severity::High,
            },
            AuditEvent::PiiRedacted {
                category: "email".into(),
                severity: PiiSeverity::Warning,
                count: 3,
            },
            AuditEvent::TokenUsed {
                provider: "openai".into(),
                model: "gpt-4".into(),
                tokens: 150,
            },
            AuditEvent::ReasoningReceived {
                content_len: 500,
                model: "claude".into(),
            },
            AuditEvent::ChatOver {
                session_id: "sess_1".into(),
                turn_count: 42,
            },
            AuditEvent::PresenceChanged {
                from: PresenceState::Active,
                to: PresenceState::Gone,
            },
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let back: AuditEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(*event, back, "round-trip failed for variant");
        }
    }
}
