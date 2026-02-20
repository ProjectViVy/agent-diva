//! Telegram channel integration

use crate::base::{ChannelError, ChannelHandler, Result};
use async_trait::async_trait;
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::TelegramConfig;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::dispatching::{Dispatcher, UpdateFilterExt};
use teloxide::prelude::*;
use teloxide::types::{BotCommand, ParseMode};
use teloxide::utils::command::BotCommands;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;

/// Telegram bot commands
#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "agent-diva commands:")]
enum Command {
    /// Start the bot
    #[command(description = "Start the bot")]
    Start,
    /// Reset conversation history
    #[command(description = "Reset conversation history")]
    Reset,
    /// Show available commands
    #[command(description = "Show this help message")]
    Help,
}

/// Telegram channel handler
pub struct TelegramHandler {
    /// Channel name
    name: String,
    /// Bot token
    token: String,
    /// Allowed senders
    allow_from: Vec<String>,
    /// Proxy URL (optional)
    #[allow(dead_code)]
    proxy: Option<String>,
    /// Bot instance
    bot: Option<Bot>,
    /// Running state
    running: bool,
    /// Inbound message sender
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    /// Dispatcher handle
    dispatcher_handle: Option<JoinHandle<()>>,
    /// Chat ID mapping (sender_id -> chat_id)
    chat_ids: Arc<RwLock<HashMap<String, i64>>>,
    /// Typing indicator tasks
    typing_tasks: Arc<Mutex<HashMap<i64, JoinHandle<()>>>>,
}

impl TelegramHandler {
    /// Create a new Telegram handler from config
    pub fn new(config: &TelegramConfig) -> Self {
        Self {
            name: "telegram".to_string(),
            token: config.token.clone(),
            allow_from: config.allow_from.clone(),
            proxy: config.proxy.clone(),
            bot: None,
            running: false,
            inbound_tx: None,
            dispatcher_handle: None,
            chat_ids: Arc::new(RwLock::new(HashMap::new())),
            typing_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Set the inbound message sender
    pub fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    /// Check if a sender is allowed
    fn is_allowed(&self, sender_id: &str) -> bool {
        if self.allow_from.is_empty() {
            return true;
        }

        if self.allow_from.contains(&sender_id.to_string()) {
            return true;
        }

        // Handle compound IDs (e.g., "12345|username")
        if sender_id.contains('|') {
            for part in sender_id.split('|') {
                if !part.is_empty() && self.allow_from.contains(&part.to_string()) {
                    return true;
                }
            }
        }

        false
    }

    /// Convert markdown to Telegram HTML
    fn markdown_to_telegram_html(text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }

        let mut result = text.to_string();

        // Protect code blocks
        let mut code_blocks: Vec<String> = Vec::new();
        let code_block_re = Regex::new(r"```[\w]*\n?([\s\S]*?)```").unwrap();
        result = code_block_re
            .replace_all(&result, |caps: &regex::Captures| {
                let idx = code_blocks.len();
                code_blocks.push(caps[1].to_string());
                format!("\x00CB{idx}\x00")
            })
            .to_string();

        // Protect inline code
        let mut inline_codes: Vec<String> = Vec::new();
        let inline_code_re = Regex::new(r"`([^`]+)`").unwrap();
        result = inline_code_re
            .replace_all(&result, |caps: &regex::Captures| {
                let idx = inline_codes.len();
                inline_codes.push(caps[1].to_string());
                format!("\x00IC{idx}\x00")
            })
            .to_string();

        // Headers # Title -> just the title
        let header_re = Regex::new(r"^#{1,6}\s+(.+)$").unwrap();
        result = header_re.replace_all(&result, "$1").to_string();

        // Blockquotes > text -> just the text
        let quote_re = Regex::new(r"^>\s*(.*)$").unwrap();
        result = quote_re.replace_all(&result, "$1").to_string();

        // Escape HTML special chars
        result = result.replace('&', "&amp;");
        result = result.replace('<', "&lt;");
        result = result.replace('>', "&gt;");

        // Links [text](url) -> <a href="url">text</a>
        let link_re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        result = link_re
            .replace_all(&result, r#"<a href="$2">$1</a>"#)
            .to_string();

        // Bold **text** or __text__
        let bold_re = Regex::new(r"\*\*(.+?)\*\*").unwrap();
        result = bold_re.replace_all(&result, "<b>$1</b>").to_string();
        let bold2_re = Regex::new(r"__(.+?)__").unwrap();
        result = bold2_re.replace_all(&result, "<b>$1</b>").to_string();

        // Italic _text_ - simplified without look-around
        let italic_re = Regex::new(r"_([^_]+)_").unwrap();
        result = italic_re.replace_all(&result, "<i>$1</i>").to_string();

        // Strikethrough ~~text~~
        let strike_re = Regex::new(r"~~(.+?)~~").unwrap();
        result = strike_re.replace_all(&result, "<s>$1</s>").to_string();

        // Bullet lists - item -> • item
        let bullet_re = Regex::new(r"^[-*]\s+").unwrap();
        result = bullet_re.replace_all(&result, "• ").to_string();

        // Restore inline code
        for (i, code) in inline_codes.iter().enumerate() {
            let escaped = code
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            result = result.replace(
                &format!("\x00IC{i}\x00"),
                &format!("<code>{escaped}</code>"),
            );
        }

        // Restore code blocks
        for (i, code) in code_blocks.iter().enumerate() {
            let escaped = code
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;");
            result = result.replace(
                &format!("\x00CB{i}\x00"),
                &format!("<pre><code>{escaped}</code></pre>"),
            );
        }

        result
    }

    /// Handle incoming text message
    #[allow(dead_code)]
    async fn handle_text_message(&self, msg: Message, bot: Bot) -> Result<()> {
        let user = msg
            .from
            .clone()
            .ok_or_else(|| ChannelError::Error("Message has no sender".to_string()))?;

        let chat_id = msg.chat.id;
        let user_id = user.id.0;

        // Build sender ID
        let sender_id = if let Some(username) = &user.username {
            format!("{}|{}", user_id, username)
        } else {
            user_id.to_string()
        };

        // Store chat ID for replies
        {
            let mut chat_ids = self.chat_ids.write().await;
            chat_ids.insert(sender_id.clone(), chat_id.0);
        }

        // Check permissions
        if !self.is_allowed(&sender_id) {
            tracing::warn!(
                "Access denied for sender {} on channel {}",
                sender_id,
                self.name
            );
            return Err(ChannelError::AccessDenied(sender_id));
        }

        // Get text content
        let content = if let Some(text) = msg.text() {
            text.to_string()
        } else if let Some(caption) = msg.caption() {
            caption.to_string()
        } else {
            "[empty message]".to_string()
        };

        // Build metadata
        let mut metadata = serde_json::Map::new();
        metadata.insert(
            "message_id".to_string(),
            serde_json::Value::Number(msg.id.0.into()),
        );
        metadata.insert(
            "user_id".to_string(),
            serde_json::Value::Number(user_id.into()),
        );
        if let Some(username) = &user.username {
            metadata.insert(
                "username".to_string(),
                serde_json::Value::String(username.clone()),
            );
        }
        metadata.insert(
            "first_name".to_string(),
            serde_json::Value::String(user.first_name.clone()),
        );
        metadata.insert(
            "is_group".to_string(),
            serde_json::Value::Bool(msg.chat.id.0 < 0),
        );

        // Start typing indicator
        self.start_typing(chat_id.0, bot.clone()).await;

        // Send to inbound channel
        if let Some(tx) = &self.inbound_tx {
            let inbound_msg =
                InboundMessage::new(self.name.clone(), sender_id, chat_id.0.to_string(), content)
                    .with_metadata("message_id", msg.id.0)
                    .with_metadata("user_id", user_id)
                    .with_metadata("first_name", user.first_name.clone())
                    .with_metadata("is_group", msg.chat.id.0 < 0);

            tx.send(inbound_msg)
                .await
                .map_err(|e| ChannelError::SendError(e.to_string()))?;
        }

        Ok(())
    }

    /// Handle /start command
    async fn handle_start(&self, msg: Message, bot: Bot) -> Result<()> {
        let user = msg
            .from
            .clone()
            .ok_or_else(|| ChannelError::Error("Message has no sender".to_string()))?;

        let text = format!(
            "👋 Hi {}! I'm agent-diva.\n\nSend me a message and I'll respond!\nType /help to see available commands.",
            user.first_name
        );

        bot.send_message(msg.chat.id, text)
            .await
            .map_err(|e| ChannelError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Handle /reset command
    async fn handle_reset(&self, msg: Message, bot: Bot) -> Result<()> {
        bot.send_message(
            msg.chat.id,
            "🔄 Conversation history cleared. Let's start fresh!",
        )
        .await
        .map_err(|e| ChannelError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Handle /help command
    async fn handle_help(&self, msg: Message, bot: Bot) -> Result<()> {
        let help_text = "🐈 <b>agent-diva commands</b>\n\n/start — Start the bot\n/reset — Reset conversation history\n/help — Show this help message\n\nJust send me a text message to chat!";

        bot.send_message(msg.chat.id, help_text)
            .parse_mode(ParseMode::Html)
            .await
            .map_err(|e| ChannelError::ApiError(e.to_string()))?;

        Ok(())
    }

    /// Start typing indicator
    #[allow(dead_code)]
    async fn start_typing(&self, chat_id: i64, bot: Bot) {
        // Cancel existing typing task
        self.stop_typing(chat_id).await;

        let typing_tasks = self.typing_tasks.clone();
        let handle = tokio::spawn(async move {
            loop {
                let _ = bot
                    .send_chat_action(ChatId(chat_id), teloxide::types::ChatAction::Typing)
                    .await;
                tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
            }
        });

        let mut tasks = typing_tasks.lock().await;
        tasks.insert(chat_id, handle);
    }

    /// Stop typing indicator
    async fn stop_typing(&self, chat_id: i64) {
        let mut tasks = self.typing_tasks.lock().await;
        if let Some(handle) = tasks.remove(&chat_id) {
            handle.abort();
        }
    }
}

#[async_trait]
impl ChannelHandler for TelegramHandler {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_running(&self) -> bool {
        self.running
    }

    async fn start(&mut self) -> Result<()> {
        if self.token.is_empty() {
            return Err(ChannelError::NotConfigured(
                "Telegram token not configured".to_string(),
            ));
        }

        if self.running {
            return Ok(());
        }

        tracing::info!("Starting Telegram bot (polling mode)...");

        // Create bot
        let bot = Bot::new(&self.token);

        // Set up command menu
        let commands = vec![
            BotCommand::new("start", "Start the bot"),
            BotCommand::new("reset", "Reset conversation history"),
            BotCommand::new("help", "Show available commands"),
        ];

        if let Err(e) = bot.set_my_commands(commands).await {
            tracing::warn!("Failed to set bot commands: {}", e);
        }

        // Get bot info
        match bot.get_me().await {
            Ok(me) => {
                let username = me.username.clone().unwrap_or_else(|| "unknown".to_string());
                tracing::info!("Telegram bot @{} connected", username);
            }
            Err(e) => {
                return Err(ChannelError::ApiError(format!(
                    "Failed to get bot info: {}",
                    e
                )));
            }
        }

        self.bot = Some(bot.clone());
        self.running = true;

        // Clone data for dispatcher - use Arc for shared ownership
        let inbound_tx = self.inbound_tx.clone();
        let chat_ids = self.chat_ids.clone();
        let typing_tasks = self.typing_tasks.clone();
        let allow_from = self.allow_from.clone();
        let name = Arc::new(self.name.clone());
        let name_cmd = name.clone();

        // Create dispatcher
        let handler = dptree::entry()
            .branch(
                Update::filter_message()
                    .filter_command::<Command>()
                    .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                        let name = name_cmd.clone();
                        async move {
                            match cmd {
                                Command::Start => {
                                    let handler = TelegramHandler {
                                        name: (*name).clone(),
                                        token: String::new(),
                                        allow_from: Vec::new(),
                                        proxy: None,
                                        bot: Some(bot.clone()),
                                        running: true,
                                        inbound_tx: None,
                                        dispatcher_handle: None,
                                        chat_ids: Arc::new(RwLock::new(HashMap::new())),
                                        typing_tasks: Arc::new(Mutex::new(HashMap::new())),
                                    };
                                    if let Err(e) = handler.handle_start(msg, bot).await {
                                        tracing::error!("Error handling /start: {}", e);
                                    }
                                }
                                Command::Reset => {
                                    let handler = TelegramHandler {
                                        name: (*name).clone(),
                                        token: String::new(),
                                        allow_from: Vec::new(),
                                        proxy: None,
                                        bot: Some(bot.clone()),
                                        running: true,
                                        inbound_tx: None,
                                        dispatcher_handle: None,
                                        chat_ids: Arc::new(RwLock::new(HashMap::new())),
                                        typing_tasks: Arc::new(Mutex::new(HashMap::new())),
                                    };
                                    if let Err(e) = handler.handle_reset(msg, bot).await {
                                        tracing::error!("Error handling /reset: {}", e);
                                    }
                                }
                                Command::Help => {
                                    let handler = TelegramHandler {
                                        name: (*name).clone(),
                                        token: String::new(),
                                        allow_from: Vec::new(),
                                        proxy: None,
                                        bot: Some(bot.clone()),
                                        running: true,
                                        inbound_tx: None,
                                        dispatcher_handle: None,
                                        chat_ids: Arc::new(RwLock::new(HashMap::new())),
                                        typing_tasks: Arc::new(Mutex::new(HashMap::new())),
                                    };
                                    if let Err(e) = handler.handle_help(msg, bot).await {
                                        tracing::error!("Error handling /help: {}", e);
                                    }
                                }
                            }
                            Ok::<(), teloxide::RequestError>(())
                        }
                    }),
            )
            .branch(
                Update::filter_message().endpoint(move |bot: Bot, msg: Message| {
                    let inbound_tx = inbound_tx.clone();
                    let chat_ids = chat_ids.clone();
                    let typing_tasks = typing_tasks.clone();
                    let allow_from = allow_from.clone();
                    let name = name.clone();

                    async move {
                        // Build sender ID
                        let user = match msg.from.clone() {
                            Some(u) => u,
                            None => return Ok(()),
                        };

                        let chat_id = msg.chat.id;
                        let user_id = user.id.0;
                        let sender_id = if let Some(ref username) = user.username {
                            format!("{}|{}", user_id, username)
                        } else {
                            user_id.to_string()
                        };

                        // Check permissions
                        let is_allowed = allow_from.is_empty()
                            || allow_from.contains(&sender_id)
                            || (sender_id.contains('|')
                                && sender_id
                                    .split('|')
                                    .any(|p| allow_from.contains(&p.to_string())));

                        if !is_allowed {
                            tracing::warn!(
                                "Access denied for sender {} on channel {}",
                                sender_id,
                                name
                            );
                            return Ok(());
                        }

                        // Store chat ID
                        {
                            let mut ids = chat_ids.write().await;
                            ids.insert(sender_id.clone(), chat_id.0);
                        }

                        // Get content
                        let content = msg
                            .text()
                            .map(|t| t.to_string())
                            .or_else(|| msg.caption().map(|c| c.to_string()))
                            .unwrap_or_else(|| "[empty message]".to_string());

                        // Start typing
                        {
                            let mut tasks = typing_tasks.lock().await;
                            let handle = tokio::spawn({
                                let bot = bot.clone();
                                let chat_id = chat_id.0;
                                async move {
                                    loop {
                                        let _ = bot
                                            .send_chat_action(
                                                ChatId(chat_id),
                                                teloxide::types::ChatAction::Typing,
                                            )
                                            .await;
                                        tokio::time::sleep(tokio::time::Duration::from_secs(4))
                                            .await;
                                    }
                                }
                            });
                            tasks.insert(chat_id.0, handle);
                        }

                        // Send to inbound channel
                        if let Some(tx) = inbound_tx {
                            let inbound_msg = InboundMessage::new(
                                name.as_ref().clone(),
                                sender_id,
                                chat_id.0.to_string(),
                                content,
                            )
                            .with_metadata("message_id", msg.id.0)
                            .with_metadata("user_id", user_id)
                            .with_metadata("first_name", user.first_name.clone())
                            .with_metadata("is_group", msg.chat.id.0 < 0);

                            if let Err(e) = tx.send(inbound_msg).await {
                                tracing::error!("Failed to send inbound message: {}", e);
                            }
                        }

                        Ok::<(), teloxide::RequestError>(())
                    }
                }),
            );

        // Start dispatcher in background
        let dispatcher_handle = tokio::spawn(async move {
            Dispatcher::builder(bot, handler)
                .enable_ctrlc_handler()
                .build()
                .dispatch()
                .await;
        });

        self.dispatcher_handle = Some(dispatcher_handle);

        tracing::info!("Telegram bot started successfully");

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        tracing::info!("Stopping Telegram bot...");

        // Stop all typing indicators
        let mut tasks = self.typing_tasks.lock().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }
        drop(tasks);

        // Abort dispatcher
        if let Some(handle) = self.dispatcher_handle.take() {
            handle.abort();
        }

        self.bot = None;
        self.running = false;

        tracing::info!("Telegram bot stopped");

        Ok(())
    }

    async fn send(&self, message: OutboundMessage) -> Result<()> {
        let bot = self
            .bot
            .as_ref()
            .ok_or_else(|| ChannelError::NotRunning("Telegram bot not running".to_string()))?;

        // Stop typing for this chat
        if let Ok(chat_id) = message.chat_id.parse::<i64>() {
            self.stop_typing(chat_id).await;
        }

        // Parse chat ID
        let chat_id: i64 = message
            .chat_id
            .parse()
            .map_err(|_| ChannelError::Error(format!("Invalid chat_id: {}", message.chat_id)))?;

        // Convert markdown to HTML
        let html_content = Self::markdown_to_telegram_html(&message.content);

        // Send message
        match bot
            .send_message(ChatId(chat_id), html_content.clone())
            .parse_mode(ParseMode::Html)
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => {
                // Fallback to plain text
                tracing::warn!("HTML parse failed, falling back to plain text: {}", e);
                bot.send_message(ChatId(chat_id), &message.content)
                    .await
                    .map_err(|e2| {
                        ChannelError::ApiError(format!("Failed to send message: {}", e2))
                    })?;
                Ok(())
            }
        }
    }

    fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.is_allowed(sender_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_telegram_html_basic() {
        let input = "Hello **world**";
        let output = TelegramHandler::markdown_to_telegram_html(input);
        assert!(output.contains("<b>world</b>"));
    }

    #[test]
    fn test_markdown_to_telegram_html_italic() {
        let input = "Hello _world_";
        let output = TelegramHandler::markdown_to_telegram_html(input);
        assert!(output.contains("<i>world</i>"));
    }

    #[test]
    fn test_markdown_to_telegram_html_code() {
        let input = "Use `code` here";
        let output = TelegramHandler::markdown_to_telegram_html(input);
        assert!(output.contains("<code>code</code>"));
    }

    #[test]
    fn test_markdown_to_telegram_html_code_block() {
        let input = "```rust\nfn main() {}\n```";
        let output = TelegramHandler::markdown_to_telegram_html(input);
        assert!(output.contains("<pre><code>"));
        assert!(output.contains("fn main() {}"));
    }

    #[test]
    fn test_markdown_to_telegram_html_link() {
        let input = "[link](https://example.com)";
        let output = TelegramHandler::markdown_to_telegram_html(input);
        assert!(output.contains(r#"<a href="https://example.com">link</a>"#));
    }

    #[test]
    fn test_markdown_to_telegram_html_strikethrough() {
        let input = "~~deleted~~";
        let output = TelegramHandler::markdown_to_telegram_html(input);
        assert!(output.contains("<s>deleted</s>"));
    }

    #[test]
    fn test_markdown_to_telegram_html_escape_html() {
        let input = "<script>alert('xss')</script>";
        let output = TelegramHandler::markdown_to_telegram_html(input);
        assert!(!output.contains("<script>"));
        assert!(output.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_telegram_handler_new() {
        let config = TelegramConfig {
            enabled: true,
            token: "test_token".to_string(),
            allow_from: vec!["user1".to_string()],
            proxy: None,
        };

        let handler = TelegramHandler::new(&config);
        assert_eq!(handler.name, "telegram");
        assert_eq!(handler.token, "test_token");
        assert_eq!(handler.allow_from, vec!["user1".to_string()]);
        assert!(!handler.running);
    }

    #[test]
    fn test_telegram_handler_is_allowed() {
        let config = TelegramConfig {
            enabled: true,
            token: "test_token".to_string(),
            allow_from: vec!["user1".to_string(), "12345".to_string()],
            proxy: None,
        };

        let handler = TelegramHandler::new(&config);
        assert!(handler.is_allowed("user1"));
        assert!(handler.is_allowed("12345"));
        assert!(handler.is_allowed("12345|user1"));
        assert!(!handler.is_allowed("unknown"));
    }

    #[test]
    fn test_telegram_handler_is_allowed_empty_list() {
        let config = TelegramConfig {
            enabled: true,
            token: "test_token".to_string(),
            allow_from: vec![],
            proxy: None,
        };

        let handler = TelegramHandler::new(&config);
        assert!(handler.is_allowed("anyone"));
        assert!(handler.is_allowed("12345"));
    }
}
