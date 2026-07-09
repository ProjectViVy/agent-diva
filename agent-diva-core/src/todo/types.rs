//! Runtime todo types for the append-only JSONL store

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status of a todo item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

    /// Stable API/CLI spelling for this status.
    pub fn as_str(&self) -> &'static str {
        match self {
            TodoStatus::Pending => "pending",
            TodoStatus::Active => "active",
            TodoStatus::Completed => "completed",
            TodoStatus::Cancelled => "cancelled",
        }
    }

    /// Parse status values accepted by update operations.
    ///
    /// `open` is retained as a CLI/API alias for `pending`; `done` is an alias
    /// for `completed`.
    pub fn parse_update(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "open" | "pending" => Some(TodoStatus::Pending),
            "active" => Some(TodoStatus::Active),
            "done" | "completed" => Some(TodoStatus::Completed),
            "cancelled" => Some(TodoStatus::Cancelled),
            _ => None,
        }
    }
}

/// Status filter accepted by todo list endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoStatusFilter {
    /// Non-terminal work: pending or active.
    Open,
    /// Exact persisted status.
    Status(TodoStatus),
}

impl TodoStatusFilter {
    /// Parse list filter values.
    ///
    /// `open` means pending plus active, `done` means completed, and exact
    /// persisted status names are accepted as well.
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "open" => Some(TodoStatusFilter::Open),
            "pending" => Some(TodoStatusFilter::Status(TodoStatus::Pending)),
            "active" => Some(TodoStatusFilter::Status(TodoStatus::Active)),
            "done" | "completed" => Some(TodoStatusFilter::Status(TodoStatus::Completed)),
            "cancelled" => Some(TodoStatusFilter::Status(TodoStatus::Cancelled)),
            _ => None,
        }
    }

    /// Return true when this filter includes `status`.
    pub fn matches(&self, status: TodoStatus) -> bool {
        match self {
            TodoStatusFilter::Open => matches!(status, TodoStatus::Pending | TodoStatus::Active),
            TodoStatusFilter::Status(expected) => *expected == status,
        }
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
    fn test_todo_status_update_aliases() {
        assert_eq!(TodoStatus::parse_update("open"), Some(TodoStatus::Pending));
        assert_eq!(
            TodoStatus::parse_update("pending"),
            Some(TodoStatus::Pending)
        );
        assert_eq!(TodoStatus::parse_update("active"), Some(TodoStatus::Active));
        assert_eq!(
            TodoStatus::parse_update("done"),
            Some(TodoStatus::Completed)
        );
        assert_eq!(
            TodoStatus::parse_update("completed"),
            Some(TodoStatus::Completed)
        );
        assert_eq!(
            TodoStatus::parse_update("cancelled"),
            Some(TodoStatus::Cancelled)
        );
        assert_eq!(TodoStatus::parse_update("garbage"), None);
    }

    #[test]
    fn test_todo_status_filter_contract() {
        let open = TodoStatusFilter::parse("open").expect("open filter");
        assert!(open.matches(TodoStatus::Pending));
        assert!(open.matches(TodoStatus::Active));
        assert!(!open.matches(TodoStatus::Completed));
        assert!(!open.matches(TodoStatus::Cancelled));

        let done = TodoStatusFilter::parse("done").expect("done filter");
        assert!(done.matches(TodoStatus::Completed));
        assert!(!done.matches(TodoStatus::Active));

        let pending = TodoStatusFilter::parse("pending").expect("pending filter");
        assert!(pending.matches(TodoStatus::Pending));
        assert!(!pending.matches(TodoStatus::Active));

        assert_eq!(TodoStatusFilter::parse("invalid"), None);
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
