//! SupervisedRun types — control-plane data model for supervised task execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Status of a supervised run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
    Lost,
}

impl RunStatus {
    /// Convert to the string stored in SQLite
    pub fn as_str(&self) -> &'static str {
        match self {
            RunStatus::Queued => "queued",
            RunStatus::Running => "running",
            RunStatus::Completed => "completed",
            RunStatus::Failed => "failed",
            RunStatus::Cancelled => "cancelled",
            RunStatus::Lost => "lost",
        }
    }

    /// Parse from the SQLite string representation
    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "queued" => Some(RunStatus::Queued),
            "running" => Some(RunStatus::Running),
            "completed" => Some(RunStatus::Completed),
            "failed" => Some(RunStatus::Failed),
            "cancelled" => Some(RunStatus::Cancelled),
            "lost" => Some(RunStatus::Lost),
            _ => None,
        }
    }

    /// Returns true if this is a terminal (final) state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled | RunStatus::Lost
        )
    }

    /// Returns true if a transition from self to target is valid.
    pub fn can_transition_to(&self, target: RunStatus) -> bool {
        match self {
            RunStatus::Queued => matches!(target, RunStatus::Running | RunStatus::Cancelled),
            RunStatus::Running => matches!(
                target,
                RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled | RunStatus::Lost
            ),
            RunStatus::Completed | RunStatus::Failed | RunStatus::Cancelled | RunStatus::Lost => {
                false
            }
        }
    }
}

/// A single supervised run item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunItem {
    pub id: String,
    pub status: RunStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron_job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RunItem {
    /// Create a new RunItem with generated UUID, Queued status, and current timestamps
    pub fn new(message: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            status: RunStatus::Queued,
            message: message.into(),
            channel: None,
            cron_job_id: None,
            heartbeat_at: None,
            created_at: now,
            updated_at: now,
        }
    }
}

/// Kind of a supervised run
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunKind {
    Cron,
    Subagent,
    Tool,
    Maintenance,
    Generic,
}

impl RunKind {
    /// Convert to the string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            RunKind::Cron => "cron",
            RunKind::Subagent => "subagent",
            RunKind::Tool => "tool",
            RunKind::Maintenance => "maintenance",
            RunKind::Generic => "generic",
        }
    }

    /// Parse from string representation
    pub fn from_str_lossy(s: &str) -> Option<Self> {
        match s {
            "cron" => Some(RunKind::Cron),
            "subagent" => Some(RunKind::Subagent),
            "tool" => Some(RunKind::Tool),
            "maintenance" => Some(RunKind::Maintenance),
            "generic" => Some(RunKind::Generic),
            _ => None,
        }
    }
}

/// Specification for a supervised run, used to enqueue a new run with full context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisedRunSpec {
    pub message: String,
    pub channel: Option<String>,
    pub cron_job_id: Option<String>,
    pub kind: RunKind,
    pub priority: i32,
    pub max_attempts: i32,
    pub timeout_secs: Option<i64>,
    pub metadata: Option<serde_json::Value>,
}

impl SupervisedRunSpec {
    /// Create a new SupervisedRunSpec from a message string
    pub fn from_spec(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            channel: None,
            cron_job_id: None,
            kind: RunKind::Generic,
            priority: 0,
            max_attempts: 3,
            timeout_secs: None,
            metadata: None,
        }
    }

    /// Set the channel
    pub fn with_channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    /// Set the cron_job_id
    pub fn with_cron_job_id(mut self, cron_job_id: impl Into<String>) -> Self {
        self.cron_job_id = Some(cron_job_id.into());
        self
    }

    /// Set the kind
    pub fn with_kind(mut self, kind: RunKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set the max_attempts
    pub fn with_max_attempts(mut self, max_attempts: i32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// Set the timeout_secs
    pub fn with_timeout_secs(mut self, timeout_secs: i64) -> Self {
        self.timeout_secs = Some(timeout_secs);
        self
    }

    /// Set the metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Full record of a supervised run, extending RunItem with additional fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    // Core fields (from RunItem)
    pub id: String,
    pub status: RunStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cron_job_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Extended fields
    pub kind: RunKind,
    pub priority: i32,
    pub attempt: i32,
    pub max_attempts: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

impl RunRecord {
    /// Create a new RunRecord from a SupervisedRunSpec
    pub fn from_spec(spec: &SupervisedRunSpec) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            status: RunStatus::Queued,
            message: spec.message.clone(),
            channel: spec.channel.clone(),
            cron_job_id: spec.cron_job_id.clone(),
            heartbeat_at: None,
            created_at: now,
            updated_at: now,
            kind: spec.kind,
            priority: spec.priority,
            attempt: 0,
            max_attempts: spec.max_attempts,
            timeout_secs: spec.timeout_secs,
            started_at: None,
            completed_at: None,
            error_message: None,
            result_summary: None,
            claimed_by: None,
            metadata: spec.metadata.clone(),
            parent_id: None,
            tags: None,
        }
    }

    /// Convert to a RunItem for backward compatibility
    pub fn to_run_item(&self) -> RunItem {
        RunItem {
            id: self.id.clone(),
            status: self.status,
            message: self.message.clone(),
            channel: self.channel.clone(),
            cron_job_id: self.cron_job_id.clone(),
            heartbeat_at: self.heartbeat_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

/// Errors that can occur during run storage operations
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RunStoreError {
    /// The requested run was not found
    #[error("run not found: {0}")]
    NotFound(String),

    /// The run is in an invalid state for the requested operation
    #[error("invalid run state: {0}")]
    InvalidState(String),

    /// The caller is not the owner of this run
    #[error("not owner: {0}")]
    NotOwner(String),

    /// Maximum retry attempts reached
    #[error("max attempts reached: {0}")]
    MaxAttemptsReached(String),

    /// Database error (internal)
    #[error("database error")]
    SqlxError(String),
}

/// Errors that can occur during run execution
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RunExecutionError {
    /// The run timed out
    #[error("run timed out")]
    Timeout,

    /// The run was cancelled
    #[error("run was cancelled")]
    Cancelled,

    /// The handler encountered an error
    #[error("handler error: {0}")]
    HandlerError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_status_roundtrip() {
        for status in [
            RunStatus::Queued,
            RunStatus::Running,
            RunStatus::Completed,
            RunStatus::Failed,
            RunStatus::Cancelled,
            RunStatus::Lost,
        ] {
            assert_eq!(RunStatus::from_str_lossy(status.as_str()), Some(status));
        }
    }

    #[test]
    fn test_run_status_from_str_lossy_invalid() {
        assert_eq!(RunStatus::from_str_lossy("unknown"), None);
    }

    #[test]
    fn test_run_item_new() {
        let item = RunItem::new("test message");
        assert_eq!(item.status, RunStatus::Queued);
        assert_eq!(item.message, "test message");
        assert!(item.channel.is_none());
        assert!(item.cron_job_id.is_none());
        assert!(item.heartbeat_at.is_none());
        assert!(!item.id.is_empty());
    }

    #[test]
    fn test_run_item_serialization() {
        let item = RunItem::new("hello");
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"status\":\"queued\""));
        assert!(json.contains("\"message\":\"hello\""));

        let deserialized: RunItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, item.id);
        assert_eq!(deserialized.message, item.message);
    }

    #[test]
    fn test_run_kind_roundtrip() {
        for kind in [
            RunKind::Cron,
            RunKind::Subagent,
            RunKind::Tool,
            RunKind::Maintenance,
            RunKind::Generic,
        ] {
            assert_eq!(RunKind::from_str_lossy(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn test_run_kind_serialization() {
        let kind = RunKind::Subagent;
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(json, "\"subagent\"");

        let deserialized: RunKind = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, kind);
    }

    #[test]
    fn test_run_store_error_display() {
        let err = RunStoreError::NotFound("run-123".to_string());
        assert_eq!(err.to_string(), "run not found: run-123");

        let err = RunStoreError::InvalidState("already running".to_string());
        assert_eq!(err.to_string(), "invalid run state: already running");

        let err = RunStoreError::NotOwner("user-456".to_string());
        assert_eq!(err.to_string(), "not owner: user-456");

        let err = RunStoreError::MaxAttemptsReached("3".to_string());
        assert_eq!(err.to_string(), "max attempts reached: 3");

        let err = RunStoreError::SqlxError("connection lost".to_string());
        assert_eq!(err.to_string(), "database error");
    }

    #[test]
    fn test_run_execution_error_display() {
        let err = RunExecutionError::Timeout;
        assert_eq!(err.to_string(), "run timed out");

        let err = RunExecutionError::Cancelled;
        assert_eq!(err.to_string(), "run was cancelled");

        let err = RunExecutionError::HandlerError("panic".to_string());
        assert_eq!(err.to_string(), "handler error: panic");
    }

    #[test]
    fn test_run_status_is_terminal() {
        assert!(!RunStatus::Queued.is_terminal());
        assert!(!RunStatus::Running.is_terminal());
        assert!(RunStatus::Completed.is_terminal());
        assert!(RunStatus::Failed.is_terminal());
        assert!(RunStatus::Cancelled.is_terminal());
        assert!(RunStatus::Lost.is_terminal());
    }

    #[test]
    fn test_run_status_can_transition_to() {
        // Queued transitions
        assert!(RunStatus::Queued.can_transition_to(RunStatus::Running));
        assert!(RunStatus::Queued.can_transition_to(RunStatus::Cancelled));
        assert!(!RunStatus::Queued.can_transition_to(RunStatus::Queued));
        assert!(!RunStatus::Queued.can_transition_to(RunStatus::Completed));
        assert!(!RunStatus::Queued.can_transition_to(RunStatus::Failed));
        assert!(!RunStatus::Queued.can_transition_to(RunStatus::Lost));

        // Running transitions
        assert!(RunStatus::Running.can_transition_to(RunStatus::Completed));
        assert!(RunStatus::Running.can_transition_to(RunStatus::Failed));
        assert!(RunStatus::Running.can_transition_to(RunStatus::Cancelled));
        assert!(RunStatus::Running.can_transition_to(RunStatus::Lost));
        assert!(!RunStatus::Running.can_transition_to(RunStatus::Queued));
        assert!(!RunStatus::Running.can_transition_to(RunStatus::Running));

        // Terminal states cannot transition to anything
        for terminal in [
            RunStatus::Completed,
            RunStatus::Failed,
            RunStatus::Cancelled,
            RunStatus::Lost,
        ] {
            for target in [
                RunStatus::Queued,
                RunStatus::Running,
                RunStatus::Completed,
                RunStatus::Failed,
                RunStatus::Cancelled,
                RunStatus::Lost,
            ] {
                assert!(
                    !terminal.can_transition_to(target),
                    "{:?} should not be able to transition to {:?}",
                    terminal,
                    target
                );
            }
        }
    }
}
