//! Runtime todo types for the append-only JSONL store

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Origin of a todo item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TodoSource {
    User,
    Agent,
    Cron,
    System,
}

impl std::fmt::Display for TodoSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TodoSource::User => write!(f, "user"),
            TodoSource::Agent => write!(f, "agent"),
            TodoSource::Cron => write!(f, "cron"),
            TodoSource::System => write!(f, "system"),
        }
    }
}

impl From<TodoSource> for String {
    fn from(source: TodoSource) -> Self {
        source.to_string()
    }
}

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
    pub fn new(title: impl Into<String>, source: TodoSource) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            status: TodoStatus::Pending,
            source: String::from(source),
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
    fn test_todo_source_display() {
        assert_eq!(TodoSource::User.to_string(), "user");
        assert_eq!(TodoSource::Agent.to_string(), "agent");
        assert_eq!(TodoSource::Cron.to_string(), "cron");
        assert_eq!(TodoSource::System.to_string(), "system");
    }

    #[test]
    fn test_todo_source_into_string() {
        let s: String = TodoSource::Agent.into();
        assert_eq!(s, "agent");
    }

    #[test]
    fn test_todo_source_serialization() {
        assert_eq!(
            serde_json::to_string(&TodoSource::User).unwrap(),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&TodoSource::Cron).unwrap(),
            "\"cron\""
        );
    }

    #[test]
    fn test_todo_item_new() {
        let item = TodoItem::new("Test task", TodoSource::User);
        assert_eq!(item.title, "Test task");
        assert_eq!(item.source, "user");
        assert_eq!(item.status, TodoStatus::Pending);
        assert!(item.session_id.is_none());
        assert!(item.linked_run_id.is_none());
        assert!(!item.id.is_empty());
    }

    #[test]
    fn test_todo_item_serialization() {
        let item = TodoItem::new("Test task", TodoSource::Cron);
        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("\"title\":\"Test task\""));
        assert!(json.contains("\"source\":\"cron\""));
        assert!(json.contains("\"status\":\"pending\""));

        let deserialized: TodoItem = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, item.id);
        assert_eq!(deserialized.title, item.title);
    }
}
