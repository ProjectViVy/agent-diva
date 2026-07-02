//! Unified security decision model.
//!
//! Provides a single enum [`SecurityDecision`] that all security
//! subsystems return, so the caller knows whether to allow, sanitize,
//! block, or quarantine content.

use serde::{Deserialize, Serialize};

/// Re-export Severity from the audit module.
use crate::audit::Severity;

/// The outcome of a security check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityDecision {
    /// Content is safe — pass through unchanged.
    Allow,
    /// Content contains sensitive data — redact and log findings.
    Sanitize {
        redacted: String,
        findings: Vec<SecurityFinding>,
    },
    /// Content is malicious or policy-violating — block entirely.
    Block {
        reason: String,
        findings: Vec<SecurityFinding>,
    },
    /// Content is suspicious but not proven malicious — quarantine for review.
    Quarantine {
        reason: String,
        findings: Vec<SecurityFinding>,
    },
}

/// A single security finding (e.g., PII match, injection pattern).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityFinding {
    /// What kind of finding this is.
    pub kind: SecurityKind,
    /// Severity of the finding.
    pub severity: Severity,
    /// Byte span in the original content (start, end).
    pub span: (usize, usize),
    /// Human-readable explanation.
    pub reason: String,
    /// Source context for the finding.
    pub source: SecurityContext,
}

/// Categories of security findings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityKind {
    PiiDetected,
    InjectionDetected,
    InstructionConflict,
    ToolOutputSuspicious,
    SkillValidationFailed,
    ChannelAuthFailed,
    PolicyViolation,
}

/// Context in which a security check is performed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityContext {
    /// Type of source (e.g., "user_input", "tool_output", "skill", "channel").
    pub source_type: String,
    /// Workspace ID if available.
    pub workspace_id: Option<String>,
    /// Run ID if available.
    pub run_id: Option<String>,
    /// Channel ID if available.
    pub channel_id: Option<String>,
    /// Tool name if applicable.
    pub tool_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_decision_allow() {
        let decision = SecurityDecision::Allow;
        assert!(matches!(decision, SecurityDecision::Allow));
    }

    #[test]
    fn test_security_decision_sanitize() {
        let finding = SecurityFinding {
            kind: SecurityKind::PiiDetected,
            severity: Severity::High,
            span: (0, 10),
            reason: "Email found".into(),
            source: SecurityContext {
                source_type: "user_input".into(),
                workspace_id: None,
                run_id: None,
                channel_id: None,
                tool_name: None,
            },
        };
        let decision = SecurityDecision::Sanitize {
            redacted: "[REDACTED]".into(),
            findings: vec![finding],
        };
        assert!(matches!(decision, SecurityDecision::Sanitize { .. }));
    }

    #[test]
    fn test_security_decision_block() {
        let decision = SecurityDecision::Block {
            reason: "Injection detected".into(),
            findings: vec![],
        };
        assert!(matches!(decision, SecurityDecision::Block { .. }));
    }

    #[test]
    fn test_security_decision_quarantine() {
        let decision = SecurityDecision::Quarantine {
            reason: "Suspicious skill".into(),
            findings: vec![],
        };
        assert!(matches!(decision, SecurityDecision::Quarantine { .. }));
    }
}
