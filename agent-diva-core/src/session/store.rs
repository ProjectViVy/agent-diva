//! Session data structures

use crate::config::schema::TokenUsage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SESSION_META_CONVERSATION_TITLE: &str = "conversation_title";
pub const SESSION_META_TITLE_GENERATED: &str = "title_generated";
pub const SESSION_META_TITLE_MANUALLY_SET: &str = "title_manually_set";
pub const SESSION_META_PINNED: &str = "pinned";

// ---------------------------------------------------------------------------
// Canonical checkpoint types
// ---------------------------------------------------------------------------

/// The only durable checkpoint schema written by the runtime.
pub const CANONICAL_CHECKPOINT_SCHEMA_VERSION: u32 = 1;
/// Maximum number of Unicode scalar values in a checkpoint body.
pub const CANONICAL_CHECKPOINT_MAX_CHARS: usize = 8_000;

/// Why a canonical checkpoint was produced.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointTrigger {
    /// The active context crossed its configured budget threshold.
    Auto,
    /// The user explicitly requested compaction.
    Manual,
    /// A provider rejected the context as too large.
    Reactive,
}

/// The bounded, durable model-visible state for a session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CanonicalCheckpoint {
    /// Schema version for the `canonical_checkpoint_v1` representation.
    pub schema_version: u32,
    /// Stable identifier for this replacement, not an append-only event ID.
    pub checkpoint_id: String,
    /// ISO-8601 creation time.
    pub created_at: String,
    /// Trigger used for diagnostics only; it does not change compaction semantics.
    pub trigger: CheckpointTrigger,
    /// Durable transcript index covered by this checkpoint.
    pub durable_message_index: usize,
    /// Number of source messages folded into this checkpoint.
    pub source_message_count: usize,
    /// Estimated source token count before folding.
    pub source_token_count: usize,
    /// Quality score of the adopted body, when semantic validation ran.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub quality_score: Option<f64>,
    /// Quality-gate diagnostics retained for operators, not injected separately.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub quality_issues: Vec<String>,
    /// Number of provider retries used to produce the body.
    #[serde(default)]
    pub retry_count: u32,
    /// Fixed-section checkpoint body, bounded to [`CANONICAL_CHECKPOINT_MAX_CHARS`].
    #[serde(deserialize_with = "deserialize_bounded_body")]
    pub body: String,
}

impl CanonicalCheckpoint {
    /// Create a checkpoint while enforcing the body bound.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        checkpoint_id: impl Into<String>,
        created_at: impl Into<String>,
        trigger: CheckpointTrigger,
        durable_message_index: usize,
        source_message_count: usize,
        source_token_count: usize,
        quality_score: Option<f64>,
        quality_issues: Vec<String>,
        retry_count: u32,
        body: impl AsRef<str>,
    ) -> Self {
        Self {
            schema_version: CANONICAL_CHECKPOINT_SCHEMA_VERSION,
            checkpoint_id: checkpoint_id.into(),
            created_at: created_at.into(),
            trigger,
            durable_message_index,
            source_message_count,
            source_token_count,
            quality_score,
            quality_issues,
            retry_count,
            body: bound_checkpoint_body(body.as_ref()),
        }
    }

    /// Render the one model-visible checkpoint block.
    pub fn render_for_context(&self) -> String {
        format!(
            "## Canonical Checkpoint v{}\n{}\n[canonical checkpoint end]",
            self.schema_version, self.body
        )
    }

    /// Move the durable coverage forward when a pending reactive turn is finalized.
    pub fn with_durable_message_index(&self, durable_message_index: usize) -> Self {
        let mut checkpoint = self.clone();
        checkpoint.durable_message_index = durable_message_index;
        checkpoint
    }
}

/// Apply the checkpoint body bound without splitting UTF-8.
pub fn bound_checkpoint_body(body: &str) -> String {
    body.chars().take(CANONICAL_CHECKPOINT_MAX_CHARS).collect()
}

fn deserialize_bounded_body<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let body = String::deserialize(deserializer)?;
    Ok(bound_checkpoint_body(&body))
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
    /// Session title generated from first user message
    #[serde(default)]
    pub title: Option<String>,
    /// Index of last consolidated message (for memory consolidation)
    #[serde(default)]
    pub last_consolidated: usize,
    /// The single bounded model-visible checkpoint, if one has been adopted.
    #[serde(default)]
    pub canonical_checkpoint: Option<CanonicalCheckpoint>,
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
            title: None,
            last_consolidated: 0,
            canonical_checkpoint: None,
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
            metadata: None,
            token_usage: None,
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
    /// Uses the higher of consolidation progress and the checkpoint coverage as
    /// the durable transcript floor.
    pub fn get_history(&self, max_messages: usize) -> Vec<ChatMessage> {
        let checkpoint_floor = self
            .canonical_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.durable_message_index)
            .unwrap_or_default();
        let floor = self
            .last_consolidated
            .max(checkpoint_floor)
            .min(self.messages.len());
        let window = &self.messages[floor..];
        let start = window.len().saturating_sub(max_messages);
        let sliced: Vec<ChatMessage> = window[start..]
            .iter()
            .filter(|m| matches!(m.role.as_str(), "user" | "assistant" | "tool"))
            .cloned()
            .collect();
        align_chat_history(sliced)
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.last_consolidated = 0;
        self.canonical_checkpoint = None;
        self.updated_at = Utc::now();
    }

    pub fn conversation_title(&self) -> Option<String> {
        self.metadata
            .get(SESSION_META_CONVERSATION_TITLE)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| {
                self.title
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
            })
    }

    pub fn set_conversation_title(&mut self, title: Option<String>) {
        let title = title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        self.title = title.clone();
        self.set_metadata_string(SESSION_META_CONVERSATION_TITLE, title);
    }

    pub fn title_generated(&self) -> bool {
        self.metadata
            .get(SESSION_META_TITLE_GENERATED)
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    pub fn set_title_generated(&mut self, value: bool) {
        self.set_metadata_bool(SESSION_META_TITLE_GENERATED, value);
    }

    pub fn title_manually_set(&self) -> bool {
        self.metadata
            .get(SESSION_META_TITLE_MANUALLY_SET)
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    pub fn set_title_manually_set(&mut self, value: bool) {
        self.set_metadata_bool(SESSION_META_TITLE_MANUALLY_SET, value);
    }

    pub fn pinned(&self) -> bool {
        self.metadata
            .get(SESSION_META_PINNED)
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    pub fn set_pinned(&mut self, value: bool) {
        self.set_metadata_bool(SESSION_META_PINNED, value);
    }

    fn set_metadata_string(&mut self, key: &str, value: Option<String>) {
        let metadata = ensure_object(&mut self.metadata);
        match value {
            Some(value) => {
                metadata.insert(key.to_string(), serde_json::Value::String(value));
            }
            None => {
                metadata.remove(key);
            }
        }
    }

    fn set_metadata_bool(&mut self, key: &str, value: bool) {
        let metadata = ensure_object(&mut self.metadata);
        metadata.insert(key.to_string(), serde_json::Value::Bool(value));
    }
}

/// Repair a chat history window so tool results always follow an assistant
/// message that declared matching `tool_calls`.
///
/// Providers (DeepSeek / OpenAI-compatible) reject histories where a `tool`
/// role message is not a response to the immediately preceding tool-call
/// group. Naive tail truncation (e.g. keep last N messages) can create that
/// shape; this keeps the window API-safe.
pub fn align_chat_history(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    // Start at the first user turn so we never open with a bare tool result.
    let Some(user_pos) = messages.iter().position(|m| m.role == "user") else {
        return Vec::new();
    };
    let window = &messages[user_pos..];

    let mut out: Vec<ChatMessage> = Vec::with_capacity(window.len());
    let mut open_tool_ids: HashSet<String> = HashSet::new();

    for msg in window {
        match msg.role.as_str() {
            "tool" => {
                let id = msg.tool_call_id.as_deref().unwrap_or("");
                if !id.is_empty() && open_tool_ids.remove(id) {
                    out.push(msg.clone());
                }
                // Orphan tool results (no open parent call) are dropped.
            }
            "assistant" => {
                // Incomplete previous tool group: strip dangling tool_calls so
                // the next turn does not require missing tool messages.
                if !open_tool_ids.is_empty() {
                    if let Some(prev) = out.iter_mut().rev().find(|m| m.role == "assistant") {
                        prev.tool_calls = None;
                    }
                    open_tool_ids.clear();
                }
                let mut cloned = msg.clone();
                if let Some(ref tcs) = cloned.tool_calls {
                    if tcs.is_empty() {
                        cloned.tool_calls = None;
                    } else {
                        for tc in tcs {
                            if let Some(id) = tc.get("id").and_then(|v| v.as_str()) {
                                if !id.is_empty() {
                                    open_tool_ids.insert(id.to_string());
                                }
                            }
                        }
                    }
                }
                out.push(cloned);
            }
            "user" => {
                if !open_tool_ids.is_empty() {
                    if let Some(prev) = out.iter_mut().rev().find(|m| m.role == "assistant") {
                        prev.tool_calls = None;
                    }
                    open_tool_ids.clear();
                }
                out.push(msg.clone());
            }
            _ => {}
        }
    }

    // Trailing incomplete tool_calls at the end of history.
    if !open_tool_ids.is_empty() {
        if let Some(prev) = out.iter_mut().rev().find(|m| m.role == "assistant") {
            prev.tool_calls = None;
        }
    }
    out
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
    /// Structured UI/runtime metadata, such as a persisted plan snapshot.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub metadata: Option<serde_json::Value>,
    /// Token usage from the LLM response for this turn (assistant messages only)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub token_usage: Option<TokenUsage>,
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
            metadata: None,
            token_usage: None,
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
            metadata: None,
            token_usage: None,
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

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("telegram:12345");
        assert_eq!(session.key, "telegram:12345");
        assert!(session.messages.is_empty());
        assert!(session.canonical_checkpoint.is_none());
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

    #[test]
    fn align_chat_history_drops_leading_orphan_tools() {
        let messages = vec![
            ChatMessage::with_tool_metadata(
                "tool",
                "orphan result",
                Some("call_orphan".into()),
                None,
                Some("read_file".into()),
            ),
            ChatMessage::new("user", "continue"),
            ChatMessage::new("assistant", "ok"),
        ];
        let aligned = align_chat_history(messages);
        assert_eq!(aligned.len(), 2);
        assert_eq!(aligned[0].role, "user");
        assert_eq!(aligned[1].role, "assistant");
    }

    #[test]
    fn align_chat_history_keeps_paired_tool_calls() {
        let assistant = ChatMessage::with_tool_metadata(
            "assistant",
            "",
            None,
            Some(vec![serde_json::json!({
                "id": "call_1",
                "type": "function",
                "function": { "name": "read_file", "arguments": "{}" }
            })]),
            None,
        );
        let tool = ChatMessage::with_tool_metadata(
            "tool",
            "file contents",
            Some("call_1".into()),
            None,
            Some("read_file".into()),
        );
        let messages = vec![
            ChatMessage::new("user", "read it"),
            assistant,
            tool,
            ChatMessage::new("assistant", "done"),
        ];
        let aligned = align_chat_history(messages);
        assert_eq!(aligned.len(), 4);
        assert_eq!(aligned[1].role, "assistant");
        assert!(aligned[1].tool_calls.is_some());
        assert_eq!(aligned[2].role, "tool");
        assert_eq!(aligned[2].tool_call_id.as_deref(), Some("call_1"));
    }

    #[test]
    fn align_chat_history_strips_incomplete_tool_calls() {
        let assistant = ChatMessage::with_tool_metadata(
            "assistant",
            "",
            None,
            Some(vec![serde_json::json!({
                "id": "call_missing",
                "type": "function",
                "function": { "name": "read_file", "arguments": "{}" }
            })]),
            None,
        );
        let messages = vec![
            ChatMessage::new("user", "read it"),
            assistant,
            ChatMessage::new("user", "never mind"),
        ];
        let aligned = align_chat_history(messages);
        assert_eq!(aligned.len(), 3);
        assert!(aligned[1].tool_calls.is_none());
    }

    #[test]
    fn test_chat_message_token_usage_serialization() {
        let mut msg = ChatMessage::new("assistant", "Hello!");
        msg.token_usage = Some(TokenUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        });

        let json = serde_json::to_string(&msg).unwrap();
        // token_usage should appear in serialized form
        assert!(json.contains("token_usage"));
        assert!(json.contains("prompt_tokens"));

        // Roundtrip
        let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(
            deserialized.token_usage.as_ref().unwrap().prompt_tokens,
            100
        );
        assert_eq!(
            deserialized.token_usage.as_ref().unwrap().completion_tokens,
            50
        );
        assert_eq!(deserialized.token_usage.as_ref().unwrap().total_tokens, 150);
    }

    #[test]
    fn test_chat_message_without_token_usage_omits_field() {
        let msg = ChatMessage::new("user", "Hello");
        let json = serde_json::to_string(&msg).unwrap();
        // When token_usage is None, skip_serializing_if should omit it
        assert!(!json.contains("token_usage"));
    }

    #[test]
    fn test_chat_message_plan_metadata_roundtrips_and_is_optional() {
        let mut msg = ChatMessage::new("assistant", "Plan snapshot");
        msg.metadata = Some(serde_json::json!({
            "kind": "plan_snapshot",
            "version": 1,
            "plan": { "plan_id": "plan-1", "phase": "Execute" }
        }));

        let json = serde_json::to_string(&msg).unwrap();
        let restored: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(
            restored
                .metadata
                .as_ref()
                .and_then(|value| value.get("kind"))
                .and_then(|value| value.as_str()),
            Some("plan_snapshot")
        );
    }

    #[test]
    fn test_chat_message_backward_compat_deserialization() {
        // Old messages without token_usage field should deserialize fine
        let json = r#"{"role":"assistant","content":"Hi","timestamp":"2025-01-01T00:00:00Z"}"#;
        let msg: ChatMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, "assistant");
        assert!(msg.token_usage.is_none());
    }

    #[test]
    fn canonical_checkpoint_is_bounded_and_old_fields_are_ignored() {
        let body = "x".repeat(CANONICAL_CHECKPOINT_MAX_CHARS + 100);
        let checkpoint = CanonicalCheckpoint::new(
            "checkpoint-1",
            "2026-01-01T00:00:00Z",
            CheckpointTrigger::Auto,
            4,
            4,
            120,
            Some(0.9),
            Vec::new(),
            0,
            body,
        );
        assert_eq!(
            checkpoint.body.chars().count(),
            CANONICAL_CHECKPOINT_MAX_CHARS
        );

        let json = serde_json::json!({
            "key": "clean-break",
            "messages": [],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "metadata": {},
            "legacy_compaction_metadata": { "ignored": true },
            "canonical_checkpoint": checkpoint,
        });
        let restored: Session = serde_json::from_value(json).unwrap();
        assert_eq!(
            restored.canonical_checkpoint.unwrap().durable_message_index,
            4
        );
    }

    #[test]
    fn missing_checkpoint_defaults_to_none() {
        let json = serde_json::json!({
            "key": "fresh",
            "messages": [],
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z",
            "metadata": {}
        });
        let session: Session = serde_json::from_value(json).unwrap();
        assert!(session.canonical_checkpoint.is_none());
    }
}

fn ensure_object(value: &mut serde_json::Value) -> &mut serde_json::Map<String, serde_json::Value> {
    if !value.is_object() {
        *value = serde_json::Value::Object(serde_json::Map::new());
    }
    value.as_object_mut().expect("metadata should be an object")
}
