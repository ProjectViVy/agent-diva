//! Event types for the message bus

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::run_telemetry::RunTelemetrySnapshotV0;
use crate::person_seam::PersonSeamVisibility;

/// Streaming events emitted by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    IterationStarted {
        index: usize,
        max_iterations: usize,
    },
    AssistantDelta {
        text: String,
    },
    ReasoningDelta {
        text: String,
    },
    ToolCallDelta {
        name: Option<String>,
        args_delta: String,
    },
    ToolCallStarted {
        name: String,
        args_preview: String,
        call_id: String,
    },
    ToolCallFinished {
        name: String,
        result: String,
        is_error: bool,
        call_id: String,
    },
    FinalResponse {
        content: String,
    },
    /// 蜂群过程事件批（`ProcessEventV0` JSON，camelCase + `name` snake_case），Story 2.3；与主流式 **分轨**。
    SwarmProcessBatch {
        events: Vec<serde_json::Value>,
    },
    /// FR22：开发者向运行遥测，**不得**拼入用户 transcript（NFR-R2）。
    RunTelemetry(RunTelemetrySnapshotV0),
    Error {
        message: String,
    },
}

/// Event with context for the bus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBusEvent {
    pub channel: String,
    pub chat_id: String,
    pub event: AgentEvent,
}

/// Message received from a chat channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    /// Channel identifier (e.g., "telegram", "discord")
    pub channel: String,
    /// User identifier
    pub sender_id: String,
    /// Chat/channel identifier
    pub chat_id: String,
    /// Message text content
    pub content: String,
    /// Message timestamp
    pub timestamp: DateTime<Utc>,
    /// Media URLs (if any)
    pub media: Vec<String>,
    /// Channel-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Story 6.6 / SWARM-MIG-02: internal subagent payload vs user-visible turn. `None` = person-visible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub person_seam: Option<PersonSeamVisibility>,
}

impl InboundMessage {
    /// Create a new inbound message
    pub fn new(
        channel: impl Into<String>,
        sender_id: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            sender_id: sender_id.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            timestamp: Utc::now(),
            media: Vec::new(),
            metadata: HashMap::new(),
            person_seam: None,
        }
    }

    /// Mark how this inbound message participates in the Person / user-visible transcript.
    pub fn with_person_seam(mut self, seam: PersonSeamVisibility) -> Self {
        self.person_seam = Some(seam);
        self
    }

    /// Get the unique session key for this message
    pub fn session_key(&self) -> String {
        format!("{}:{}", self.channel, self.chat_id)
    }

    /// Add media URL to the message
    pub fn with_media(mut self, url: impl Into<String>) -> Self {
        self.media.push(url.into());
        self
    }

    /// Add metadata to the message
    pub fn with_metadata(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Message to send to a chat channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    /// Channel identifier
    pub channel: String,
    /// Target chat/channel identifier
    pub chat_id: String,
    /// Message text content
    pub content: String,
    /// Optional message to reply to
    pub reply_to: Option<String>,
    /// Media URLs to attach
    pub media: Vec<String>,
    /// Reasoning content (if any)
    pub reasoning_content: Option<String>,
    /// Channel-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl OutboundMessage {
    /// Create a new outbound message
    pub fn new(
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            channel: channel.into(),
            chat_id: chat_id.into(),
            content: content.into(),
            reply_to: None,
            media: Vec::new(),
            reasoning_content: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the reply-to message ID
    pub fn reply_to(mut self, message_id: impl Into<String>) -> Self {
        self.reply_to = Some(message_id.into());
        self
    }

    /// Add media URL to the message
    pub fn with_media(mut self, url: impl Into<String>) -> Self {
        self.media.push(url.into());
        self
    }

    /// Add metadata to the message
    pub fn with_metadata(
        mut self,
        key: impl Into<String>,
        value: impl Into<serde_json::Value>,
    ) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}
