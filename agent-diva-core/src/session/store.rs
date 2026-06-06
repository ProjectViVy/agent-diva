//! Session data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Compaction types
// ---------------------------------------------------------------------------

/// What triggered a context compaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactTrigger {
    /// Budget threshold exceeded — automatic compaction
    Auto,
    /// User-triggered (e.g. /compact command)
    Manual,
    /// Provider overflow catch — reactive compaction (P1)
    Reactive,
}

/// Index range of compacted messages in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionRange {
    /// Start index (inclusive) of compacted messages
    pub start_index: usize,
    /// End index (exclusive) of compacted messages
    pub end_index: usize,
}

/// A type-safe, serializable compaction record stored in the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactSummary {
    /// Schema version for forward compatibility
    pub schema_version: u32,
    /// Unique compact event ID
    pub compact_id: String,
    /// ISO8601 timestamp when compaction occurred
    pub created_at: String,
    /// What triggered this compaction
    pub trigger: CompactTrigger,
    /// Index range of the compacted messages
    pub source_range: CompactionRange,
    /// Number of recent messages kept (not compacted)
    pub kept_recent_count: usize,
    /// Message count before compaction
    pub pre_compact_message_count: usize,
    /// Estimated tokens before compaction
    pub pre_compact_estimated_tokens: usize,
    /// The generated natural-language summary
    pub summary: String,
    /// Quality score of the adopted summary (0.0–1.0), if quality validation ran
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub quality_score: Option<f64>,
    /// Number of LLM retries before the final summary was adopted (0 = first attempt succeeded)
    #[serde(default)]
    pub retry_count: u32,
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

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
    /// Index of last compacted message (messages before this are summarized in `compaction`)
    #[serde(default)]
    pub last_compacted: usize,
    /// Context compaction summary, if compaction has occurred
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub compaction: Option<CompactSummary>,
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
            last_compacted: 0,
            compaction: None,
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
            reasoning_content: None,
            thinking_blocks: None,
        });
        self.updated_at = Utc::now();
    }

    /// Add a complete ChatMessage to the session
    pub fn add_full_message(&mut self, msg: ChatMessage) {
        self.messages.push(msg);
        self.updated_at = Utc::now();
    }

    /// Get message history for LLM context.
    ///
    /// Uses the *higher* of `last_consolidated` and `last_compacted` as the
    /// floor so that both compacted and consolidated messages are excluded.
    pub fn get_history(&self, max_messages: usize) -> Vec<ChatMessage> {
        // Floor = max of the two progress pointers
        let floor = self
            .last_consolidated
            .max(self.last_compacted)
            .min(self.messages.len());
        let window = &self.messages[floor..];
        let start = window.len().saturating_sub(max_messages);
        let mut sliced: Vec<ChatMessage> = window[start..]
            .iter()
            .filter(|m| matches!(m.role.as_str(), "user" | "assistant" | "tool"))
            .cloned()
            .collect();
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
        self.last_compacted = 0;
        self.compaction = None;
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
    /// Optional reasoning content captured from thinking-capable models
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reasoning_content: Option<String>,
    /// Optional structured thinking blocks (provider-specific)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub thinking_blocks: Option<Vec<serde_json::Value>>,
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
            reasoning_content: None,
            thinking_blocks: None,
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
            reasoning_content: None,
            thinking_blocks: None,
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
