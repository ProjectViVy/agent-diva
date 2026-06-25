use crate::bus::AgentBusEvent;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum AuditEvent {
    ToolInvoked {
        tool: String,
        args_hash: String,
        duration_ms: u64,
    },
    ToolDenied {
        tool: String,
        reason: String,
    },
    DecisionPoint {
        phase: String,
        llm_decision: String,
    },
    InjectionDetected {
        pattern: String,
        severity: String,
    },
    PiiRedacted {
        kind: String,
        count: usize,
    },
    TokenUsed {
        prompt: i64,
        completion: i64,
        total: i64,
        model: String,
    },
    PresenceChanged {
        from: String,
        to: String,
    },
    HeartbeatTriggered {
        state: String,
        tasks: String,
    },
}

impl AuditEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::ToolInvoked { .. } => "tool_invoked",
            Self::ToolDenied { .. } => "tool_denied",
            Self::DecisionPoint { .. } => "decision_point",
            Self::InjectionDetected { .. } => "injection_detected",
            Self::PiiRedacted { .. } => "pii_redacted",
            Self::TokenUsed { .. } => "token_used",
            Self::PresenceChanged { .. } => "presence_changed",
            Self::HeartbeatTriggered { .. } => "heartbeat_triggered",
        }
    }
}

impl From<&AgentBusEvent> for AuditEvent {
    fn from(value: &AgentBusEvent) -> Self {
        match value {
            AgentBusEvent::ToolInvoked {
                tool,
                args_hash,
                duration_ms,
            } => Self::ToolInvoked {
                tool: tool.clone(),
                args_hash: args_hash.clone(),
                duration_ms: *duration_ms,
            },
            AgentBusEvent::ToolDenied { tool, reason } => Self::ToolDenied {
                tool: tool.clone(),
                reason: reason.clone(),
            },
            AgentBusEvent::DecisionPoint {
                phase,
                llm_decision,
            } => Self::DecisionPoint {
                phase: phase.clone(),
                llm_decision: llm_decision.clone(),
            },
            AgentBusEvent::InjectionDetected { pattern, severity } => Self::InjectionDetected {
                pattern: pattern.clone(),
                severity: severity.clone(),
            },
            AgentBusEvent::PiiRedacted { kind, count } => Self::PiiRedacted {
                kind: kind.clone(),
                count: *count,
            },
            AgentBusEvent::TokenUsed {
                prompt,
                completion,
                total,
                model,
            } => Self::TokenUsed {
                prompt: *prompt,
                completion: *completion,
                total: *total,
                model: model.clone(),
            },
            AgentBusEvent::PresenceChanged { from, to } => Self::PresenceChanged {
                from: format!("{from:?}"),
                to: format!("{to:?}"),
            },
            AgentBusEvent::HeartbeatTriggered { state, tasks } => Self::HeartbeatTriggered {
                state: state.clone(),
                tasks: tasks.clone(),
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AuditLogger;

impl AuditLogger {
    pub fn emit_bus_event(&self, event: &AgentBusEvent) {
        self.emit(AuditEvent::from(event));
    }

    pub fn emit(&self, event: AuditEvent) {
        match event {
            AuditEvent::ToolInvoked {
                tool,
                args_hash,
                duration_ms,
            } => {
                info!(
                    target: "audit",
                    event_type = "tool_invoked",
                    tool,
                    args_hash,
                    duration_ms,
                    "audit"
                );
            }
            AuditEvent::ToolDenied { tool, reason } => {
                info!(
                    target: "audit",
                    event_type = "tool_denied",
                    tool,
                    reason,
                    "audit"
                );
            }
            AuditEvent::DecisionPoint {
                phase,
                llm_decision,
            } => {
                info!(
                    target: "audit",
                    event_type = "decision_point",
                    phase,
                    llm_decision,
                    "audit"
                );
            }
            AuditEvent::InjectionDetected { pattern, severity } => {
                info!(
                    target: "audit",
                    event_type = "injection_detected",
                    pattern,
                    severity,
                    "audit"
                );
            }
            AuditEvent::PiiRedacted { kind, count } => {
                info!(
                    target: "audit",
                    event_type = "pii_redacted",
                    kind,
                    count,
                    "audit"
                );
            }
            AuditEvent::TokenUsed {
                prompt,
                completion,
                total,
                model,
            } => {
                info!(
                    target: "audit",
                    event_type = "token_used",
                    prompt,
                    completion,
                    total,
                    model,
                    "audit"
                );
            }
            AuditEvent::PresenceChanged { from, to } => {
                info!(
                    target: "audit",
                    event_type = "presence_changed",
                    from,
                    to,
                    "audit"
                );
            }
            AuditEvent::HeartbeatTriggered { state, tasks } => {
                info!(
                    target: "audit",
                    event_type = "heartbeat_triggered",
                    state,
                    tasks,
                    "audit"
                );
            }
        }
    }
}

pub fn audit_log_file_name_for_date(date: NaiveDate) -> String {
    format!("gateway.log.{}", date.format("%Y-%m-%d"))
}

pub fn is_audit_log_file_name(name: &str) -> bool {
    name == "gateway.log" || name.starts_with("gateway.log.") || name.starts_with("gateway-")
}

#[cfg(test)]
mod tests {
    use super::{audit_log_file_name_for_date, is_audit_log_file_name, AuditEvent, AuditLogger};
    use chrono::NaiveDate;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::writer::MakeWriter;
    use tracing_subscriber::prelude::*;

    #[derive(Clone, Default)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl SharedBuffer {
        fn text(&self) -> String {
            String::from_utf8(self.0.lock().expect("buffer lock poisoned").clone())
                .expect("buffer should be utf8")
        }
    }

    struct SharedWriter(Arc<Mutex<Vec<u8>>>);

    impl std::io::Write for SharedWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("buffer lock poisoned")
                .extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> MakeWriter<'a> for SharedBuffer {
        type Writer = SharedWriter;

        fn make_writer(&'a self) -> Self::Writer {
            SharedWriter(self.0.clone())
        }
    }

    #[test]
    fn audit_logger_emits_structured_json_fields() {
        let sink = SharedBuffer::default();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(sink.clone())
                .with_target(true)
                .with_ansi(false),
        );

        tracing::subscriber::with_default(subscriber, || {
            AuditLogger.emit(AuditEvent::ToolDenied {
                tool: "shell".to_string(),
                reason: "policy".to_string(),
            });
        });

        let line = sink.text();
        assert!(line.contains("\"target\":\"audit\""));
        assert!(line.contains("\"event_type\":\"tool_denied\""));
        assert!(line.contains("\"tool\":\"shell\""));
        assert!(line.contains("\"reason\":\"policy\""));
    }

    #[test]
    fn audit_log_name_uses_daily_rotation_format() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 25).expect("valid date");
        assert_eq!(audit_log_file_name_for_date(date), "gateway.log.2026-06-25");
    }

    #[test]
    fn audit_log_name_matcher_accepts_rotated_and_legacy_files() {
        assert!(is_audit_log_file_name("gateway.log"));
        assert!(is_audit_log_file_name("gateway.log.2026-06-25"));
        assert!(is_audit_log_file_name("gateway-2026-06-25.log"));
        assert!(!is_audit_log_file_name("other.log"));
    }
}
