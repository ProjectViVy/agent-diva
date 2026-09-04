//! Message forwarding tool

use agent_diva_core::channel::{
    AttachmentRef, ChannelAddress, ChannelCommand, ChannelDirection, ChannelEnvelopeV1,
    ChannelOrigin, ChannelPayloadV1, ContentPart, Correlation,
};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Callback type for sending typed adapter commands.
type SendCallback = Arc<
    dyn Fn(
            ChannelCommand,
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
        F: Fn(ChannelCommand) -> Fut + Send + Sync + 'static,
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
                "attachments": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "description": "AttachmentRef resolved by the shared file authority"
                    },
                    "description": "Optional typed AttachmentRef values"
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

        let attachments = if let Some(m) = params.get("attachments").and_then(|v| v.as_array()) {
            m.iter()
                .map(|value| {
                    serde_json::from_value(value.clone()).map_err(|error| {
                        ToolError::InvalidParams(format!("Invalid typed attachment: {error}"))
                    })
                })
                .collect::<Result<Vec<agent_diva_core::channel::AttachmentRef>, _>>()?
        } else {
            Vec::<agent_diva_core::channel::AttachmentRef>::new()
        };

        // Validate channel and chat_id
        if channel.is_empty() || chat_id.is_empty() {
            return Ok("Error: No target channel/chat specified".to_string());
        }

        // Check callback
        let callback = self.send_callback.as_ref().ok_or_else(|| {
            ToolError::ExecutionFailed("Message sending not configured".to_string())
        })?;

        let mut parts = vec![ContentPart::Text { text: content }];
        parts.extend(attachments.iter().cloned().map(attachment_content_part));
        let address = ChannelAddress {
            channel: channel.clone(),
            account_id: None,
            sender_id: None,
            chat_id: chat_id.clone(),
            thread_id: None,
        };
        let correlation = Correlation::new(format!("{channel}:{chat_id}"));
        let envelope = ChannelEnvelopeV1::new(
            ChannelDirection::Egress,
            address,
            correlation,
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Message {
                parts,
                subject: None,
                locale: None,
                context: None,
            },
        );
        let command = ChannelCommand::Send {
            envelope,
            idempotency_key: Some(uuid::Uuid::new_v4().to_string()),
        };

        // Send message
        match callback(command).await {
            Ok(_) => {
                let media_info = if !attachments.is_empty() {
                    format!(" with {} attachments", attachments.len())
                } else {
                    String::new()
                };
                Ok(format!(
                    "Message sent to {}:{}{}",
                    channel, chat_id, media_info
                ))
            }
            Err(e) => Ok(format!("Error sending message: {}", e)),
        }
    }
}

fn attachment_content_part(attachment: AttachmentRef) -> ContentPart {
    if attachment.media_type.starts_with("image/") {
        ContentPart::Image { attachment }
    } else if attachment.media_type.starts_with("audio/") {
        ContentPart::Audio {
            attachment,
            transcript: None,
        }
    } else if attachment.media_type.starts_with("video/") {
        ContentPart::Video { attachment }
    } else {
        ContentPart::File { attachment }
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

    #[tokio::test]
    async fn typed_attachments_keep_their_media_variants_in_the_command() {
        let mut tool = MessageTool::new();
        tool.set_context("telegram".to_string(), "123".to_string())
            .await;
        let captured = Arc::new(Mutex::new(None));
        let callback_capture = captured.clone();
        tool.set_send_callback(move |command| {
            let callback_capture = callback_capture.clone();
            async move {
                *callback_capture.lock().await = Some(command);
                Ok(())
            }
        });

        let attachment = |uri: &str, media_type: &str, file_name: &str| {
            json!({
                "uri": uri,
                "media_type": media_type,
                "size_bytes": 1,
                "sha256": uri.trim_start_matches("sha256:"),
                "file_name": file_name,
            })
        };
        let result = tool
            .execute(json!({
                "content": "hello",
                "attachments": [
                    attachment("sha256:image", "image/png", "image.png"),
                    attachment("sha256:audio", "audio/ogg", "audio.ogg"),
                    attachment("sha256:video", "video/mp4", "video.mp4"),
                    attachment("sha256:file", "application/pdf", "file.pdf"),
                ]
            }))
            .await
            .unwrap();

        assert!(result.contains("with 4 attachments"));
        let command = captured.lock().await.take().expect("callback command");
        let ChannelCommand::Send { envelope, .. } = command else {
            panic!("message tool must emit a typed Send command");
        };
        assert_eq!(envelope.direction, ChannelDirection::Egress);
        assert_eq!(envelope.origin, ChannelOrigin::Runtime);
        match envelope.payload {
            ChannelPayloadV1::Message { parts, .. } => {
                assert!(
                    matches!(parts.first(), Some(ContentPart::Text { text }) if text == "hello")
                );
                assert!(
                    matches!(parts.get(1), Some(ContentPart::Image { attachment }) if attachment.media_type == "image/png")
                );
                assert!(
                    matches!(parts.get(2), Some(ContentPart::Audio { attachment, transcript }) if attachment.media_type == "audio/ogg" && transcript.is_none())
                );
                assert!(
                    matches!(parts.get(3), Some(ContentPart::Video { attachment }) if attachment.media_type == "video/mp4")
                );
                assert!(
                    matches!(parts.get(4), Some(ContentPart::File { attachment }) if attachment.media_type == "application/pdf")
                );
            }
            payload => panic!("unexpected message tool payload: {payload:?}"),
        }
    }
}
