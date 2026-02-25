//! Mattermost channel handler using REST API polling.

use async_trait::async_trait;
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::MattermostConfig;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::base::{ChannelError, ChannelHandler, Result};

/// Mattermost channel handler
pub struct MattermostHandler {
    config: MattermostConfig,
    allow_from: Vec<String>,
    running: Arc<RwLock<bool>>,
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    poll_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    http: reqwest::Client,
    bot_user_id: Arc<RwLock<Option<String>>>,
    bot_username: Arc<RwLock<Option<String>>>,
}

impl MattermostHandler {
    pub fn new(config: MattermostConfig) -> Self {
        Self {
            allow_from: config.allow_from.clone(),
            config,
            running: Arc::new(RwLock::new(false)),
            inbound_tx: None,
            poll_task: Arc::new(Mutex::new(None)),
            http: reqwest::Client::new(),
            bot_user_id: Arc::new(RwLock::new(None)),
            bot_username: Arc::new(RwLock::new(None)),
        }
    }

    /// Fetch the bot's own user ID and username via GET /api/v4/users/me
    async fn fetch_bot_identity(
        http: &reqwest::Client,
        base_url: &str,
        token: &str,
    ) -> std::result::Result<(String, String), ChannelError> {
        let url = format!("{}/api/v4/users/me", base_url.trim_end_matches('/'));
        let resp = http
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Failed to get bot identity: {}", e)))?;

        if !resp.status().is_success() {
            return Err(ChannelError::AuthError(format!(
                "GET /users/me returned {}",
                resp.status()
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::ApiError(format!("Invalid JSON from /users/me: {}", e)))?;

        let id = body["id"]
            .as_str()
            .ok_or_else(|| ChannelError::ApiError("Missing 'id' in /users/me response".into()))?
            .to_string();
        let username = body["username"]
            .as_str()
            .ok_or_else(|| ChannelError::ApiError("Missing 'username' in /users/me response".into()))?
            .to_string();

        Ok((id, username))
    }
}

/// Check if text contains an @mention of the given username at a word boundary.
fn contains_bot_mention(text: &str, bot_username: &str) -> bool {
    let mention = format!("@{}", bot_username);
    let lower = text.to_lowercase();
    let target = mention.to_lowercase();

    for (idx, _) in lower.match_indices(&target) {
        let end = idx + target.len();
        let before_ok = idx == 0 || !lower.as_bytes()[idx - 1].is_ascii_alphanumeric();
        let after_ok = end >= lower.len() || !lower.as_bytes()[end].is_ascii_alphanumeric();
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

/// Strip @mentions of the bot from message content.
fn normalize_content(text: &str, bot_username: &str) -> String {
    let mention = format!("@{}", bot_username);
    let result = text.replace(&mention, "");
    // Also try case-insensitive replacement
    let lower_mention = mention.to_lowercase();
    let mut out = String::with_capacity(result.len());
    let lower_result = result.to_lowercase();
    let mut last = 0;
    for (idx, _) in lower_result.match_indices(&lower_mention) {
        out.push_str(&result[last..idx]);
        last = idx + lower_mention.len();
    }
    out.push_str(&result[last..]);
    out.trim().to_string()
}

#[async_trait]
impl ChannelHandler for MattermostHandler {
    fn name(&self) -> &str {
        "mattermost"
    }

    fn is_running(&self) -> bool {
        // Use try_read to avoid blocking; fall back to false
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

        // Fetch bot identity
        let (bot_id, bot_name) =
            Self::fetch_bot_identity(&self.http, &self.config.base_url, &self.config.bot_token)
                .await?;
        info!("Mattermost bot identity: {} ({})", bot_name, bot_id);
        *self.bot_user_id.write().await = Some(bot_id.clone());
        *self.bot_username.write().await = Some(bot_name.clone());

        let running = self.running.clone();
        let http = self.http.clone();
        let base_url = self.config.base_url.clone();
        let token = self.config.bot_token.clone();
        let channel_id = self.config.channel_id.clone();
        let poll_interval = self.config.poll_interval_seconds;
        let mention_only = self.config.mention_only;
        let allow_from = self.allow_from.clone();

        *running.write().await = true;

        let handle = tokio::spawn(async move {
            let mut last_post_time: i64 = chrono::Utc::now().timestamp_millis();

            loop {
                if !*running.read().await {
                    break;
                }

                match poll_posts(&http, &base_url, &token, &channel_id, last_post_time).await {
                    Ok(posts) => {
                        for post in &posts {
                            let post_time = post["create_at"].as_i64().unwrap_or(0);
                            if post_time > last_post_time {
                                last_post_time = post_time;
                            }

                            // Skip bot's own messages
                            let user_id = post["user_id"].as_str().unwrap_or("");
                            if user_id == bot_id {
                                continue;
                            }

                            let message = post["message"].as_str().unwrap_or("");
                            if message.is_empty() {
                                continue;
                            }

                            // Allowlist check
                            if !allow_from.is_empty() && !allow_from.contains(&user_id.to_string()) {
                                debug!("Mattermost: ignoring message from non-allowed user {}", user_id);
                                continue;
                            }

                            // Mention-only check
                            if mention_only && !contains_bot_mention(message, &bot_name) {
                                debug!("Mattermost: ignoring non-mention message");
                                continue;
                            }

                            let content = normalize_content(message, &bot_name);
                            if content.is_empty() {
                                continue;
                            }

                            let post_id = post["id"].as_str().unwrap_or("");
                            let root_id = post["root_id"].as_str().unwrap_or("");
                            let ch_id = post["channel_id"].as_str().unwrap_or(&channel_id);

                            // Build chat_id with thread info: "channel_id:root_id"
                            let chat_id = if !root_id.is_empty() {
                                format!("{}:{}", ch_id, root_id)
                            } else {
                                format!("{}:{}", ch_id, post_id)
                            };

                            let mut msg = InboundMessage::new("mattermost", user_id, &chat_id, &content);
                            msg = msg.with_metadata("post_id", serde_json::Value::String(post_id.to_string()));
                            if !root_id.is_empty() {
                                msg = msg.with_metadata("root_id", serde_json::Value::String(root_id.to_string()));
                            }

                            if let Err(e) = tx.send(msg).await {
                                error!("Mattermost: failed to send inbound message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Mattermost poll error: {}", e);
                    }
                }

                tokio::time::sleep(std::time::Duration::from_secs(poll_interval)).await;
            }

            info!("Mattermost polling task stopped");
        });

        *self.poll_task.lock().await = Some(handle);
        info!("Mattermost channel started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;
        if let Some(handle) = self.poll_task.lock().await.take() {
            handle.abort();
        }
        info!("Mattermost channel stopped");
        Ok(())
    }

    async fn send(&self, message: OutboundMessage) -> Result<()> {
        let base_url = self.config.base_url.trim_end_matches('/');
        let url = format!("{}/api/v4/posts", base_url);

        // Parse chat_id format: "channel_id:root_id"
        let (channel_id, root_id) = if let Some(idx) = message.chat_id.find(':') {
            let (ch, rest) = message.chat_id.split_at(idx);
            (ch.to_string(), rest[1..].to_string())
        } else {
            (message.chat_id.clone(), String::new())
        };

        let mut body = serde_json::json!({
            "channel_id": channel_id,
            "message": message.content,
        });

        // Thread reply if root_id is present and thread_replies is enabled
        if self.config.thread_replies && !root_id.is_empty() {
            body["root_id"] = serde_json::Value::String(root_id);
        }

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.bot_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Mattermost send error: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed(format!(
                "Mattermost POST /posts returned {}: {}",
                status, text
            )));
        }

        Ok(())
    }

    async fn test_connection(&self) -> Result<()> {
        Self::fetch_bot_identity(&self.http, &self.config.base_url, &self.config.bot_token)
            .await
            .map(|_| ())
    }
}

/// Poll for new posts in a channel since `since` timestamp (milliseconds).
async fn poll_posts(
    http: &reqwest::Client,
    base_url: &str,
    token: &str,
    channel_id: &str,
    since: i64,
) -> std::result::Result<Vec<serde_json::Value>, String> {
    let url = format!(
        "{}/api/v4/channels/{}/posts?since={}",
        base_url.trim_end_matches('/'),
        channel_id,
        since
    );

    let resp = http
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| format!("HTTP error: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("GET posts returned {}", resp.status()));
    }

    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;

    // Mattermost returns { order: [...], posts: { id: post, ... } }
    let order = body["order"].as_array();
    let posts_map = &body["posts"];

    let mut posts = Vec::new();
    if let Some(order) = order {
        // Iterate in reverse so oldest first
        for post_id in order.iter().rev() {
            if let Some(id) = post_id.as_str() {
                if let Some(post) = posts_map.get(id) {
                    posts.push(post.clone());
                }
            }
        }
    }

    Ok(posts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_bot_mention_basic() {
        assert!(contains_bot_mention("hello @divabot", "divabot"));
        assert!(contains_bot_mention("@divabot hello", "divabot"));
        assert!(contains_bot_mention("hey @divabot how are you", "divabot"));
    }

    #[test]
    fn test_contains_bot_mention_case_insensitive() {
        assert!(contains_bot_mention("hello @DivaBot", "divabot"));
        assert!(contains_bot_mention("hello @DIVABOT", "divabot"));
    }

    #[test]
    fn test_contains_bot_mention_word_boundary() {
        assert!(!contains_bot_mention("hello @divabotextra", "divabot"));
        assert!(contains_bot_mention("hello @divabot!", "divabot"));
        assert!(contains_bot_mention("@divabot", "divabot"));
    }

    #[test]
    fn test_contains_bot_mention_no_match() {
        assert!(!contains_bot_mention("hello world", "divabot"));
        assert!(!contains_bot_mention("hello divabot", "divabot")); // no @ prefix
    }

    #[test]
    fn test_normalize_content() {
        assert_eq!(normalize_content("@divabot hello", "divabot"), "hello");
        assert_eq!(normalize_content("hello @divabot world", "divabot"), "hello  world");
        assert_eq!(normalize_content("@divabot", "divabot"), "");
    }

    #[test]
    fn test_allowlist_empty_allows_all() {
        let config = MattermostConfig::default();
        let handler = MattermostHandler::new(config);
        assert!(handler.is_allowed("anyone"));
        assert!(handler.is_allowed("user123"));
    }

    #[test]
    fn test_allowlist_restricts() {
        let mut config = MattermostConfig::default();
        config.allow_from = vec!["user1".to_string(), "user2".to_string()];
        let handler = MattermostHandler::new(config);
        assert!(handler.is_allowed("user1"));
        assert!(handler.is_allowed("user2"));
        assert!(!handler.is_allowed("user3"));
    }

    #[test]
    fn test_config_defaults() {
        let config = MattermostConfig::default();
        assert!(!config.enabled);
        assert!(config.thread_replies);
        assert!(!config.mention_only);
        assert_eq!(config.poll_interval_seconds, 3);
    }

    #[test]
    fn test_config_deserialize_minimal() {
        let json = r#"{"enabled": true, "base_url": "https://mm.example.com", "bot_token": "tok123", "channel_id": "ch1"}"#;
        let config: MattermostConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.base_url, "https://mm.example.com");
        assert_eq!(config.bot_token, "tok123");
        assert!(config.thread_replies); // default
        assert_eq!(config.poll_interval_seconds, 3); // default
    }
}
