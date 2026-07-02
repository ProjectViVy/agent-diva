//! ID newtypes for planning domain objects.
//!
//! [`PlanId`] and [`TodoId`] are UUID-backed identifiers that provide
//! type safety and prevent mixing unrelated IDs at compile time.

use serde::{Deserialize, Serialize};

/// Unique identifier for a [`Plan`](super::model::Plan).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlanId(pub String);

/// Unique identifier for a [`TodoItem`](super::model::TodoItem).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TodoId(pub String);

impl PlanId {
    /// Generate a new random plan ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl TodoId {
    /// Generate a new random todo ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

impl std::fmt::Display for PlanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for TodoId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for PlanId {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for TodoId {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_id_creation() {
        let id = PlanId::new();
        assert!(!id.0.is_empty());
        // UUID v4 format: 8-4-4-4-12
        assert_eq!(id.0.len(), 36);
        assert_eq!(id.0.chars().filter(|c| *c == '-').count(), 4);
    }

    #[test]
    fn test_todo_id_display() {
        let id = TodoId("test-id-123".to_string());
        assert_eq!(format!("{id}"), "test-id-123");
    }

    #[test]
    fn test_ids_not_equal() {
        let a = PlanId::new();
        let b = PlanId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_default_ids_are_unique() {
        let a = PlanId::default();
        let b = PlanId::default();
        assert_ne!(a, b);
    }

    #[test]
    fn test_serde_roundtrip_ids() {
        let plan_id = PlanId::new();
        let json = serde_json::to_string(&plan_id).unwrap();
        let back: PlanId = serde_json::from_str(&json).unwrap();
        assert_eq!(plan_id, back);

        let todo_id = TodoId::new();
        let json = serde_json::to_string(&todo_id).unwrap();
        let back: TodoId = serde_json::from_str(&json).unwrap();
        assert_eq!(todo_id, back);
    }
}
