//! Base trait for channel handlers

use async_trait::async_trait;
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::Config;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Trait for channel handlers
#[async_trait]
pub trait ChannelHandler: Send + Sync {
    /// Get the channel name
    fn name(&self) -> &str;

    /// Check if the channel is running
    fn is_running(&self) -> bool;

    /// Start the channel handler
    async fn start(&mut self) -> Result<()>;

    /// Stop the channel handler
    async fn stop(&mut self) -> Result<()>;

    /// Send a message
    async fn send(&self, message: OutboundMessage) -> Result<()>;

    /// Set the inbound message sender
    fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>);

    /// Check if a sender is allowed
    fn is_allowed(&self, sender_id: &str) -> bool;
}

/// Channel errors
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Channel error: {0}")]
    Error(String),

    #[error("Channel not configured: {0}")]
    NotConfigured(String),

    #[error("Channel not running: {0}")]
    NotRunning(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Send error: {0}")]
    SendError(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Access denied for sender: {0}")]
    AccessDenied(String),
}

pub type Result<T> = std::result::Result<T, ChannelError>;

/// Base channel implementation with common functionality
pub struct BaseChannel {
    /// Channel name
    pub name: String,
    /// Channel configuration
    pub config: Config,
    /// Running state
    pub running: bool,
    /// Allowed senders list (empty = allow all)
    pub allow_from: Vec<String>,
    /// Inbound message sender
    pub inbound_tx: Option<mpsc::Sender<InboundMessage>>,
}

impl BaseChannel {
    /// Create a new base channel
    pub fn new(name: impl Into<String>, config: Config, allow_from: Vec<String>) -> Self {
        Self {
            name: name.into(),
            config,
            running: false,
            allow_from,
            inbound_tx: None,
        }
    }

    /// Set the inbound message sender
    pub fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    /// Check if a sender is allowed
    pub fn is_allowed(&self, sender_id: &str) -> bool {
        // If no allow list, allow everyone
        if self.allow_from.is_empty() {
            return true;
        }

        let sender_str = sender_id.to_string();
        if self.allow_from.contains(&sender_str) {
            return true;
        }

        // Handle compound IDs (e.g., "12345|username")
        if sender_str.contains('|') {
            for part in sender_str.split('|') {
                if !part.is_empty() && self.allow_from.contains(&part.to_string()) {
                    return true;
                }
            }
        }

        false
    }

    /// Handle an incoming message
    pub async fn handle_message(
        &self,
        sender_id: impl Into<String>,
        chat_id: impl Into<String>,
        content: impl Into<String>,
        media: Vec<String>,
        metadata: Option<serde_json::Map<String, serde_json::Value>>,
    ) -> Result<()> {
        let sender_id = sender_id.into();

        // Check permissions
        if !self.is_allowed(&sender_id) {
            tracing::warn!(
                "Access denied for sender {} on channel {}. Add them to allowFrom list in config to grant access.",
                sender_id,
                self.name
            );
            return Err(ChannelError::AccessDenied(sender_id));
        }

        let mut msg = InboundMessage::new(self.name.clone(), sender_id, chat_id, content);

        // Add media
        for m in media {
            msg = msg.with_media(m);
        }

        // Add metadata
        if let Some(meta) = metadata {
            for (key, value) in meta {
                msg = msg.with_metadata(key, value);
            }
        }

        // Send to inbound channel
        if let Some(tx) = &self.inbound_tx {
            tx.send(msg)
                .await
                .map_err(|e| ChannelError::SendError(e.to_string()))?;
        }

        Ok(())
    }
}

/// Shared channel handler type
pub type ChannelHandlerPtr = Arc<RwLock<dyn ChannelHandler>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_channel_is_allowed_empty_list() {
        let config = Config::default();
        let channel = BaseChannel::new("test", config, vec![]);

        assert!(channel.is_allowed("user1"));
        assert!(channel.is_allowed("12345"));
        assert!(channel.is_allowed("anyone"));
    }

    #[test]
    fn test_base_channel_is_allowed_with_list() {
        let config = Config::default();
        let channel = BaseChannel::new(
            "test",
            config,
            vec!["user1".to_string(), "12345".to_string()],
        );

        assert!(channel.is_allowed("user1"));
        assert!(channel.is_allowed("12345"));
        assert!(!channel.is_allowed("user2"));
        assert!(!channel.is_allowed("99999"));
    }

    #[test]
    fn test_base_channel_is_allowed_compound_id() {
        let config = Config::default();
        let channel = BaseChannel::new(
            "test",
            config,
            vec!["user1".to_string(), "12345".to_string()],
        );

        // Compound ID with username
        assert!(channel.is_allowed("12345|user1"));
        assert!(channel.is_allowed("99999|user1"));
        assert!(!channel.is_allowed("99999|unknown"));
    }

    #[test]
    fn test_channel_error_display() {
        let err = ChannelError::NotConfigured("test".to_string());
        assert_eq!(err.to_string(), "Channel not configured: test");

        let err = ChannelError::AccessDenied("user1".to_string());
        assert_eq!(err.to_string(), "Access denied for sender: user1");
    }
}
