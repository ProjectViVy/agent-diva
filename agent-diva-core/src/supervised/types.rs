//! SupervisedRun types — control-plane data model for supervised task execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
}
