//! Session data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session key (channel:chat_id)
    pub key: String,
    /// Messages in the session
    pub messages: Vec<ChatMessage>,
    /// Session creation time
    pub created_at: DateTime<Utc>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
    /// Session metadata
    pub metadata: serde_json::Value,
    /// Index of last consolidated message (for memory consolidation)
    #[serde(default)]
    pub last_consolidated: usize,
}

impl Session {
    /// Create a new session
    pub fn new(key: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            key: key.into(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
            metadata: serde_json::Value::Object(serde_json::Map::new()),
            last_consolidated: 0,
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, role: impl Into<String>, content: impl Into<String>) {
        self.messages.push(ChatMessage {
            role: role.into(),
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        });
        self.updated_at = Utc::now();
    }

    /// Add a complete ChatMessage to the session
    pub fn add_full_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
        self.updated_at = Utc::now();
    }

    /// Get message history for LLM context
    pub fn get_history(&self, max_messages: usize) -> Vec<ChatMessage> {
        // Clamp last_consolidated to avoid out-of-bounds on corrupted data
        let consolidated = self.last_consolidated.min(self.messages.len());
        let unconsolidated = &self.messages[consolidated..];
        let start = unconsolidated.len().saturating_sub(max_messages);
        let mut sliced = unconsolidated[start..].to_vec();
        // Drop leading non-user messages to avoid orphaned tool results
        if let Some(pos) = sliced.iter().position(|m| m.role == "user") {
            sliced = sliced[pos..].to_vec();
        }
        sliced
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.last_consolidated = 0;
        self.updated_at = Utc::now();
    }
}

/// A chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Message role (user, assistant, system, tool)
    pub role: String,
    /// Message content
    pub content: String,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Tool call ID (for tool-result messages)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_call_id: Option<String>,
    /// Tool calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub tool_calls: Option<Vec<serde_json::Value>>,
    /// Tool name (for tool-result messages)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub name: Option<String>,
}

impl ChatMessage {
    /// Create a new chat message
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        }
    }

    /// Create a chat message with full tool metadata
    pub fn with_tool_metadata(
        role: impl Into<String>,
        content: impl Into<String>,
        tool_call_id: Option<String>,
        tool_calls: Option<Vec<serde_json::Value>>,
        name: Option<String>,
    ) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id,
            tool_calls,
            name,
        }
    }

    /// Convert to LLM format (role and content only)
    pub fn to_llm_format(&self) -> serde_json::Value {
        serde_json::json!({
            "role": &self.role,
            "content": &self.content,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("telegram:12345");
        assert_eq!(session.key, "telegram:12345");
        assert!(session.messages.is_empty());
    }

    #[test]
    fn test_add_message() {
        let mut session = Session::new("test");
        session.add_message("user", "Hello");
        session.add_message("assistant", "Hi there!");

        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[1].role, "assistant");
    }

    #[test]
    fn test_get_history() {
        let mut session = Session::new("test");
        for i in 0..60 {
            session.add_message("user", format!("Message {}", i));
        }

        let history = session.get_history(50);
        assert_eq!(history.len(), 50);
    }
}
