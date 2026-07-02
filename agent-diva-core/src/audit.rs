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
    /// A new supervised run was created
    RunCreated {
        run_id: String,
        kind: String,
    },
    /// A supervised run was claimed by a worker
    RunClaimed {
        run_id: String,
        worker_id: String,
    },
    /// A supervised run completed successfully
    RunCompleted {
        run_id: String,
        duration_ms: u64,
    },
    /// A supervised run failed during execution
    RunFailed {
        run_id: String,
        error: String,
    },
    /// A supervised run was marked as lost (stale heartbeat)
    RunLost {
        run_id: String,
        last_heartbeat: Option<String>,
    },
    /// A message was blocked due to security policy.
    MessageBlocked {
        source: String,
        reason: String,
        severity: Severity,
    },
    /// An instruction hierarchy conflict was detected.
    InstructionConflictDetected {
        lower_tier: String,
        higher_tier: String,
        action: String,
    },
    /// Tool output was sanitized due to suspicious content.
    ToolOutputSanitized {
        tool_name: String,
        bytes_in: u32,
        bytes_out: u32,
        suspicious_spans: Vec<String>,
    },
    /// A skill was loaded successfully.
    SkillLoaded {
        skill_name: String,
        trust_tier: String,
        provenance: String,
    },
    /// A skill was rejected during validation.
    SkillRejected {
        skill_name: String,
        reason: String,
    },
    /// A skill was quarantined for review.
    SkillQuarantined {
        skill_name: String,
        reason: String,
    },
    /// Channel authentication failed.
    ChannelAuthFailed {
        channel_id: String,
        reason: String,
    },
    /// A channel message was blocked.
    ChannelMessageBlocked {
        channel_id: String,
        reason: String,
    },
    /// A security policy decision was made.
    SecurityPolicyDecision {
        source_type: String,
        decision_kind: String,
        reason: String,
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
            AuditEvent::RunCreated {
                run_id: "r1".into(),
                kind: "test".into(),
            },
            AuditEvent::RunClaimed {
                run_id: "r1".into(),
                worker_id: "w1".into(),
            },
            AuditEvent::RunCompleted {
                run_id: "r1".into(),
                duration_ms: 1000,
            },
            AuditEvent::RunFailed {
                run_id: "r1".into(),
                error: "timeout".into(),
            },
            AuditEvent::RunLost {
                run_id: "r1".into(),
                last_heartbeat: None,
            },
            // New variants
            AuditEvent::MessageBlocked {
                source: "user_input".into(),
                reason: "policy violation".into(),
                severity: Severity::High,
            },
            AuditEvent::InstructionConflictDetected {
                lower_tier: "tool".into(),
                higher_tier: "system".into(),
                action: "demote".into(),
            },
            AuditEvent::ToolOutputSanitized {
                tool_name: "bash".into(),
                bytes_in: 1000,
                bytes_out: 950,
                suspicious_spans: vec!["role_override".into()],
            },
            AuditEvent::SkillLoaded {
                skill_name: "test_skill".into(),
                trust_tier: "trusted".into(),
                provenance: "workspace".into(),
            },
            AuditEvent::SkillRejected {
                skill_name: "bad_skill".into(),
                reason: "validation failed".into(),
            },
            AuditEvent::SkillQuarantined {
                skill_name: "suspicious_skill".into(),
                reason: "untrusted provenance".into(),
            },
            AuditEvent::ChannelAuthFailed {
                channel_id: "chan_1".into(),
                reason: "invalid session key".into(),
            },
            AuditEvent::ChannelMessageBlocked {
                channel_id: "chan_1".into(),
                reason: "empty allowlist".into(),
            },
            AuditEvent::SecurityPolicyDecision {
                source_type: "user_input".into(),
                decision_kind: "block".into(),
                reason: "injection detected".into(),
            },
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let back: AuditEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(*event, back, "round-trip failed for variant");
        }
    }
}
