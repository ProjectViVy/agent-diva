//! Runtime todo types for the append-only JSONL store

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a todo item
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    Active,
    Completed,
    Cancelled,
}

impl TodoStatus {
    /// Returns true if the status is terminal (Completed or Cancelled)
    pub fn is_terminal(&self) -> bool {
        matches!(self, TodoStatus::Completed | TodoStatus::Cancelled)
    }
}

/// A single todo item stored in the JSONL file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub title: String,
    pub status: TodoStatus,
    /// Origin of the todo: "user", "cron", "agent"
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_run_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TodoItem {
    /// Create a new todo item with generated UUID and current timestamps
    pub fn new(title: impl Into<String>, source: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            status: TodoStatus::Pending,
            source: source.into(),
            session_id: None,
            linked_run_id: None,
            created_at: now,
            updated_at: now,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_status_terminal() {
        assert!(!TodoStatus::Pending.is_terminal());
        assert!(!TodoStatus::Active.is_terminal());
        assert!(TodoStatus::Completed.is_terminal());
        assert!(TodoStatus::Cancelled.is_terminal());
    }

    #[test]
    fn test_todo_item_new() {
        let item = TodoItem::new("Test task", "user");
        assert_eq!(item.title, "Test task");
        assert_eq!(item.source, "user");
        assert_eq!(item.status, TodoStatus::Pending);
        assert!(item.session_id.is_none());
        assert!(item.linked_run_id.is_none());
        assert!(!item.id.is_empty());
    }

    #[test]
    fn test_todo_item_serialization() {
        let item = TodoItem::new("Test task", "cron");
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"title\":\"Test task\""));
        assert!(json.contains("\"source\":\"cron\""));
        assert!(json.contains("\"status\":\"pending\""));

        let deserialized: TodoItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, item.id);
        assert_eq!(deserialized.title, item.title);
    }
}
