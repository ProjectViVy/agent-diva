//! Nextcloud Talk channel handler using OCS API long-polling.

use async_trait::async_trait;
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::NextcloudTalkConfig;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::base::{ChannelError, ChannelHandler, Result};

/// Nextcloud Talk channel handler
pub struct NextcloudTalkHandler {
    config: NextcloudTalkConfig,
    allow_from: Vec<String>,
    running: Arc<RwLock<bool>>,
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    poll_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    http: reqwest::Client,
}

/// Message types we care about from Nextcloud Talk
const NC_MSG_COMMENT: i64 = 1;

/// Actor types to skip (bots, system, guests with no ID, etc.)
fn should_skip_actor(actor_type: &str) -> bool {
    matches!(actor_type, "bots" | "bridged" | "")
}

impl NextcloudTalkHandler {
    pub fn new(config: NextcloudTalkConfig) -> Self {
        Self {
            allow_from: config.allow_from.clone(),
            config,
            running: Arc::new(RwLock::new(false)),
            inbound_tx: None,
            poll_task: Arc::new(Mutex::new(None)),
            http: reqwest::Client::new(),
        }
    }

    /// Build the OCS API base URL
    #[allow(dead_code)]
    fn ocs_base(&self) -> String {
        format!(
            "{}/ocs/v2.php/apps/spreed/api/v1",
            self.config.base_url.trim_end_matches('/')
        )
    }

    /// Send a message to the room via OCS API
    async fn send_to_room(
        http: &reqwest::Client,
        base_url: &str,
        token: &str,
        room_token: &str,
        message: &str,
    ) -> std::result::Result<(), ChannelError> {
        let url = format!(
            "{}/ocs/v2.php/apps/spreed/api/v1/chat/{}",
            base_url.trim_end_matches('/'),
            room_token
        );

        let resp = http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("OCS-APIRequest", "true")
            .header("Accept", "application/json")
            .json(&serde_json::json!({ "message": message }))
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Nextcloud Talk send error: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed(format!(
                "Nextcloud Talk POST chat returned {}: {}",
                status, text
            )));
        }

        Ok(())
    }

    /// Fetch the latest message ID to use as starting point for polling
    async fn fetch_last_known_message_id(
        http: &reqwest::Client,
        base_url: &str,
        token: &str,
        room_token: &str,
    ) -> std::result::Result<i64, ChannelError> {
        let url = format!(
            "{}/ocs/v2.php/apps/spreed/api/v1/chat/{}?lookIntoFuture=0&limit=1",
            base_url.trim_end_matches('/'),
            room_token
        );

        let resp = http
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("OCS-APIRequest", "true")
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to fetch last message: {}", e)))?;

        if !resp.status().is_success() {
            return Err(ChannelError::ApiError(format!(
                "GET chat returned {}",
                resp.status()
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::ApiError(format!("Invalid JSON: {}", e)))?;

        // OCS response: { ocs: { data: [ { id: ... }, ... ] } }
        let messages = body["ocs"]["data"].as_array();
        if let Some(msgs) = messages {
            if let Some(first) = msgs.first() {
                if let Some(id) = first["id"].as_i64() {
                    return Ok(id);
                }
            }
        }

        // No messages yet, start from 0
        Ok(0)
    }
}

#[async_trait]
impl ChannelHandler for NextcloudTalkHandler {
    fn name(&self) -> &str {
        "nextcloud_talk"
    }

    fn is_running(&self) -> bool {
        self.running.try_read().map(|r| *r).unwrap_or(false)
    }

    fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        if self.allow_from.is_empty() {
            return true;
        }
        self.allow_from.contains(&sender_id.to_string())
    }

    async fn start(&mut self) -> Result<()> {
        if *self.running.read().await {
            return Ok(());
        }

        let tx = self
            .inbound_tx
            .clone()
            .ok_or_else(|| ChannelError::Error("Inbound sender not set".into()))?;

        // Get the last known message ID so we only poll for new messages
        let last_id = Self::fetch_last_known_message_id(
            &self.http,
            &self.config.base_url,
            &self.config.app_token,
            &self.config.room_token,
        )
        .await?;

        info!("Nextcloud Talk: starting poll from message ID {}", last_id);

        let running = self.running.clone();
        let http = self.http.clone();
        let base_url = self.config.base_url.clone();
        let token = self.config.app_token.clone();
        let room_token = self.config.room_token.clone();
        let poll_interval = self.config.poll_interval_seconds;
        let allow_from = self.allow_from.clone();

        *running.write().await = true;

        let handle = tokio::spawn(async move {
            let mut last_known_id = last_id;

            loop {
                if !*running.read().await {
                    break;
                }

                let url = format!(
                    "{}/ocs/v2.php/apps/spreed/api/v1/chat/{}?lookIntoFuture=1&lastKnownMessageId={}&timeout=30",
                    base_url.trim_end_matches('/'),
                    room_token,
                    last_known_id
                );

                let result = http
                    .get(&url)
                    .header("Authorization", format!("Bearer {}", token))
                    .header("OCS-APIRequest", "true")
                    .header("Accept", "application/json")
                    .timeout(std::time::Duration::from_secs(35))
                    .send()
                    .await;

                match result {
                    Ok(resp) if resp.status().as_u16() == 304 => {
                        // No new messages (long-poll timeout)
                        debug!("Nextcloud Talk: no new messages (304)");
                    }
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(body) = resp.json::<serde_json::Value>().await {
                            if let Some(messages) = body["ocs"]["data"].as_array() {
                                for msg in messages {
                                    let msg_id = msg["id"].as_i64().unwrap_or(0);
                                    if msg_id > last_known_id {
                                        last_known_id = msg_id;
                                    }

                                    // Only process comment-type messages
                                    let msg_type = msg["messageType"]
                                        .as_i64()
                                        .or_else(|| {
                                            // Some versions use string "comment"
                                            if msg["messageType"].as_str() == Some("comment") {
                                                Some(NC_MSG_COMMENT)
                                            } else {
                                                None
                                            }
                                        })
                                        .unwrap_or(0);

                                    if msg_type != NC_MSG_COMMENT {
                                        // Also check the "systemMessage" field
                                        let sys = msg["systemMessage"].as_str().unwrap_or("");
                                        if !sys.is_empty() {
                                            continue;
                                        }
                                    }

                                    let actor_type = msg["actorType"].as_str().unwrap_or("");
                                    if should_skip_actor(actor_type) {
                                        continue;
                                    }

                                    let actor_id = msg["actorId"].as_str().unwrap_or("");
                                    if actor_id.is_empty() {
                                        continue;
                                    }

                                    let content = msg["message"].as_str().unwrap_or("");
                                    if content.is_empty() {
                                        continue;
                                    }

                                    // Allowlist check
                                    if !allow_from.is_empty()
                                        && !allow_from.contains(&actor_id.to_string())
                                    {
                                        debug!(
                                            "Nextcloud Talk: ignoring message from non-allowed user {}",
                                            actor_id
                                        );
                                        continue;
                                    }

                                    let inbound = InboundMessage::new(
                                        "nextcloud_talk",
                                        actor_id,
                                        &room_token,
                                        content,
                                    )
                                    .with_metadata(
                                        "message_id",
                                        serde_json::Value::Number(msg_id.into()),
                                    );

                                    if let Err(e) = tx.send(inbound).await {
                                        error!(
                                            "Nextcloud Talk: failed to send inbound message: {}",
                                            e
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Ok(resp) => {
                        warn!("Nextcloud Talk poll returned {}", resp.status());
                        tokio::time::sleep(std::time::Duration::from_secs(poll_interval)).await;
                    }
                    Err(e) => {
                        // Timeout errors are expected for long-polling
                        if e.is_timeout() {
                            debug!("Nextcloud Talk: poll timeout (expected)");
                        } else {
                            warn!("Nextcloud Talk poll error: {}", e);
                            tokio::time::sleep(std::time::Duration::from_secs(poll_interval))
                                .await;
                        }
                    }
                }
            }

            info!("Nextcloud Talk polling task stopped");
        });

        *self.poll_task.lock().await = Some(handle);
        info!("Nextcloud Talk channel started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;
        if let Some(handle) = self.poll_task.lock().await.take() {
            handle.abort();
        }
        info!("Nextcloud Talk channel stopped");
        Ok(())
    }

    async fn send(&self, message: OutboundMessage) -> Result<()> {
        Self::send_to_room(
            &self.http,
            &self.config.base_url,
            &self.config.app_token,
            &message.chat_id,
            &message.content,
        )
        .await
    }

    async fn test_connection(&self) -> Result<()> {
        Self::fetch_last_known_message_id(
            &self.http,
            &self.config.base_url,
            &self.config.app_token,
            &self.config.room_token,
        )
        .await
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_skip_actor() {
        assert!(should_skip_actor("bots"));
        assert!(should_skip_actor("bridged"));
        assert!(should_skip_actor(""));
        assert!(!should_skip_actor("users"));
        assert!(!should_skip_actor("guests"));
    }

    #[test]
    fn test_allowlist_empty_allows_all() {
        let config = NextcloudTalkConfig::default();
        let handler = NextcloudTalkHandler::new(config);
        assert!(handler.is_allowed("anyone"));
        assert!(handler.is_allowed("user123"));
    }

    #[test]
    fn test_allowlist_restricts() {
        let mut config = NextcloudTalkConfig::default();
        config.allow_from = vec!["alice".to_string(), "bob".to_string()];
        let handler = NextcloudTalkHandler::new(config);
        assert!(handler.is_allowed("alice"));
        assert!(handler.is_allowed("bob"));
        assert!(!handler.is_allowed("charlie"));
    }

    #[test]
    fn test_config_defaults() {
        let config = NextcloudTalkConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.poll_interval_seconds, 5);
        assert!(config.base_url.is_empty());
    }

    #[test]
    fn test_config_deserialize_minimal() {
        let json = r#"{
            "enabled": true,
            "base_url": "https://cloud.example.com",
            "app_token": "tok123",
            "room_token": "room456"
        }"#;
        let config: NextcloudTalkConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.base_url, "https://cloud.example.com");
        assert_eq!(config.poll_interval_seconds, 5);
    }
}
