//! Message forwarding tool

use crate::base::{Tool, ToolError};
use async_trait::async_trait;
use agent_diva_core::bus::OutboundMessage;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Callback type for sending outbound messages
type SendCallback = Arc<
    dyn Fn(
            OutboundMessage,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
        + Send
        + Sync,
>;

/// Message tool for sending messages to users
pub struct MessageTool {
    send_callback: Option<SendCallback>,
    default_channel: Arc<Mutex<String>>,
    default_chat_id: Arc<Mutex<String>>,
}

impl MessageTool {
    /// Create a new message tool
    pub fn new() -> Self {
        Self {
            send_callback: None,
            default_channel: Arc::new(Mutex::new(String::new())),
            default_chat_id: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Create with default context
    pub fn with_context(channel: String, chat_id: String) -> Self {
        Self {
            send_callback: None,
            default_channel: Arc::new(Mutex::new(channel)),
            default_chat_id: Arc::new(Mutex::new(chat_id)),
        }
    }

    /// Set the current message context
    pub async fn set_context(&self, channel: String, chat_id: String) {
        *self.default_channel.lock().await = channel;
        *self.default_chat_id.lock().await = chat_id;
    }

    /// Set the callback for sending messages
    pub fn set_send_callback<F, Fut>(&mut self, callback: F)
    where
        F: Fn(OutboundMessage) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        self.send_callback = Some(Arc::new(move |msg| Box::pin(callback(msg))));
    }
}

impl Default for MessageTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MessageTool {
    fn name(&self) -> &str {
        "message"
    }

    fn description(&self) -> &str {
        "Send a message to the user. Use this when you want to communicate something."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The message content to send"
                },
                "channel": {
                    "type": "string",
                    "description": "Optional: target channel (telegram, discord, etc.)"
                },
                "chat_id": {
                    "type": "string",
                    "description": "Optional: target chat/user ID"
                },
                "media": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Optional: list of file paths to attach (images, audio, documents)"
                }
            },
            "required": ["content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let content = params
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'content' parameter".to_string()))?
            .to_string();

        let channel = if let Some(ch) = params.get("channel").and_then(|v| v.as_str()) {
            ch.to_string()
        } else {
            self.default_channel.lock().await.clone()
        };

        let chat_id = if let Some(id) = params.get("chat_id").and_then(|v| v.as_str()) {
            id.to_string()
        } else {
            self.default_chat_id.lock().await.clone()
        };

        let media = if let Some(m) = params.get("media").and_then(|v| v.as_array()) {
            m.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            Vec::new()
        };

        // Validate channel and chat_id
        if channel.is_empty() || chat_id.is_empty() {
            return Ok("Error: No target channel/chat specified".to_string());
        }

        // Check callback
        let callback = self.send_callback.as_ref().ok_or_else(|| {
            ToolError::ExecutionFailed("Message sending not configured".to_string())
        })?;

        // Create outbound message
        let mut msg = OutboundMessage::new(channel.clone(), chat_id.clone(), content);
        msg.media = media.clone();

        // Send message
        match callback(msg).await {
            Ok(_) => {
                let media_info = if !media.is_empty() {
                    format!(" with {} attachments", media.len())
                } else {
                    String::new()
                };
                Ok(format!("Message sent to {}:{}{}", channel, chat_id, media_info))
            }
            Err(e) => Ok(format!("Error sending message: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_tool_no_callback() {
        let tool = MessageTool::new();
        tool.set_context("telegram".to_string(), "123".to_string())
            .await;

        let params = json!({"content": "Hello"});
        let result = tool.execute(params).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_message_tool_with_callback() {
        let mut tool = MessageTool::new();
        tool.set_context("telegram".to_string(), "123".to_string())
            .await;

        // Set a simple callback
        tool.set_send_callback(|_msg| async { Ok(()) });

        let params = json!({"content": "Hello"});
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("Message sent"));
    }

    #[tokio::test]
    async fn test_message_tool_no_context() {
        let mut tool = MessageTool::new();
        tool.set_send_callback(|_msg| async { Ok(()) });

        let params = json!({"content": "Hello"});
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("No target channel/chat"));
    }
}
