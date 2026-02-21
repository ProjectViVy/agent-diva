//! Email channel implementation using blocking IMAP polling + SMTP replies
//!
//! Note: This implementation uses blocking IMAP operations wrapped in tokio::task::spawn_blocking
//! for better compatibility. Since email is not a high-frequency channel, the performance
//! trade-off is acceptable.

use async_trait::async_trait;
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::EmailConfig;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::base::{BaseChannel, ChannelError, ChannelHandler, Result};

/// Email message structure
#[derive(Debug, Clone)]
struct EmailMessage {
    sender: String,
    subject: String,
    message_id: String,
    content: String,
    uid: String,
    date: String,
}

/// Email channel handler using IMAP polling + SMTP replies
pub struct EmailHandler {
    config: EmailConfig,
    base: BaseChannel,
    running: Arc<RwLock<bool>>,
    poll_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    processed_uids: Arc<RwLock<HashSet<String>>>,
    last_subjects: Arc<RwLock<HashMap<String, String>>>,
    last_message_ids: Arc<RwLock<HashMap<String, String>>>,
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
}

impl EmailHandler {
    /// Create a new email handler
    pub fn new(config: EmailConfig, base_config: agent_diva_core::config::schema::Config) -> Self {
        let allow_from = config.allow_from.clone();
        let base = BaseChannel::new("email", base_config, allow_from);

        Self {
            config,
            base,
            running: Arc::new(RwLock::new(false)),
            poll_task: Arc::new(Mutex::new(None)),
            processed_uids: Arc::new(RwLock::new(HashSet::new())),
            last_subjects: Arc::new(RwLock::new(HashMap::new())),
            last_message_ids: Arc::new(RwLock::new(HashMap::new())),
            inbound_tx: None,
        }
    }

    /// Validate configuration
    fn validate_config(&self) -> Result<()> {
        let mut missing = Vec::new();

        if self.config.imap_host.is_empty() {
            missing.push("imap_host");
        }
        if self.config.imap_username.is_empty() {
            missing.push("imap_username");
        }
        if self.config.imap_password.is_empty() {
            missing.push("imap_password");
        }
        if self.config.smtp_host.is_empty() {
            missing.push("smtp_host");
        }
        if self.config.smtp_username.is_empty() {
            missing.push("smtp_username");
        }
        if self.config.smtp_password.is_empty() {
            missing.push("smtp_password");
        }

        if !missing.is_empty() {
            return Err(ChannelError::InvalidConfig(format!(
                "Missing required fields: {}",
                missing.join(", ")
            )));
        }

        Ok(())
    }

    /// Format reply subject
    fn reply_subject(&self, base_subject: &str) -> String {
        let subject = if base_subject.is_empty() {
            "agent-diva reply"
        } else {
            base_subject
        };

        let prefix = if self.config.subject_prefix.is_empty() {
            "Re: "
        } else {
            &self.config.subject_prefix
        };

        if subject.to_lowercase().starts_with("re:") {
            subject.to_string()
        } else {
            format!("{}{}", prefix, subject)
        }
    }

    /// Convert HTML to plain text
    #[allow(dead_code)]
    fn html_to_text(&self, html: &str) -> String {
        let text = html
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("</p>", "\n");

        let re = regex::Regex::new(r"<[^>]+>").unwrap_or_else(|_| regex::Regex::new(r"").unwrap());
        let text = re.replace_all(&text, "");

        html_escape::decode_html_entities(&text).to_string()
    }

    /// Parse email from raw bytes
    #[allow(dead_code)]
    fn parse_email(&self, body: &[u8], uid: String) -> Option<EmailMessage> {
        let parsed = mail_parser::MessageParser::default().parse(body)?;

        let sender = parsed
            .from()
            .and_then(|addrs| addrs.first())
            .and_then(|addr| addr.address())
            .map(|s| s.to_lowercase())?;

        if sender.is_empty() {
            return None;
        }

        let subject = parsed.subject().unwrap_or("").to_string();
        let message_id = parsed
            .message_id()
            .map(|id| id.to_string())
            .unwrap_or_default();
        let date = parsed.date().map(|d| d.to_rfc3339()).unwrap_or_default();

        let body_text = if let Some(text) = parsed.body_text(0) {
            text.to_string()
        } else if let Some(html) = parsed.body_html(0) {
            self.html_to_text(&html)
        } else {
            String::from_utf8_lossy(body).to_string()
        };

        let content = format!(
            "Email received.\nFrom: {}\nSubject: {}\nDate: {}\n\n{}",
            sender,
            subject,
            date,
            &body_text[..body_text.len().min(self.config.max_body_chars)]
        );

        Some(EmailMessage {
            sender,
            subject,
            message_id,
            content,
            uid,
            date,
        })
    }

    /// Fetch messages using blocking IMAP (runs in spawn_blocking)
    fn fetch_messages_blocking(
        config: &EmailConfig,
        processed: &HashSet<String>,
    ) -> Result<Vec<EmailMessage>> {
        use native_tls::TlsConnector;

        let mailbox = if config.imap_mailbox.is_empty() {
            "INBOX"
        } else {
            &config.imap_mailbox
        };

        // Connect to IMAP server
        let tls = TlsConnector::new()
            .map_err(|e| ChannelError::ConnectionError(format!("TLS connector: {}", e)))?;

        let client = imap::connect(
            (config.imap_host.as_str(), config.imap_port),
            &config.imap_host,
            &tls,
        )
        .map_err(|e| ChannelError::ConnectionError(format!("IMAP connect: {}", e)))?;

        // Login
        let mut session = client
            .login(&config.imap_username, &config.imap_password)
            .map_err(|e| ChannelError::AuthError(format!("IMAP login: {:?}", e)))?;

        // Select mailbox
        session
            .select(mailbox)
            .map_err(|e| ChannelError::ConnectionError(format!("Select mailbox: {}", e)))?;

        // Search for unseen messages
        let uids = session
            .uid_search("UNSEEN")
            .map_err(|e| ChannelError::ConnectionError(format!("UID search: {}", e)))?;

        let mut messages = Vec::new();

        for uid in uids {
            let uid_str = uid.to_string();

            // Skip if already processed
            if processed.contains(&uid_str) {
                continue;
            }

            // Fetch message
            let fetched = session
                .uid_fetch(&uid_str, "(BODY.PEEK[] UID)")
                .map_err(|e| ChannelError::ConnectionError(format!("Fetch message: {}", e)))?;

            for fetch in fetched.iter() {
                if let Some(body) = fetch.body() {
                    // Parse email
                    if let Some(parsed) = mail_parser::MessageParser::default().parse(body) {
                        let sender = parsed
                            .from()
                            .and_then(|addrs| addrs.first())
                            .and_then(|addr| addr.address())
                            .map(|s| s.to_lowercase());

                        if let Some(sender) = sender {
                            if !sender.is_empty() {
                                let subject = parsed.subject().unwrap_or("").to_string();
                                let message_id = parsed
                                    .message_id()
                                    .map(|id| id.to_string())
                                    .unwrap_or_default();
                                let date =
                                    parsed.date().map(|d| d.to_rfc3339()).unwrap_or_default();

                                let body_text = if let Some(text) = parsed.body_text(0) {
                                    text.to_string()
                                } else if let Some(html) = parsed.body_html(0) {
                                    Self::html_to_text_static(&html)
                                } else {
                                    String::from_utf8_lossy(body).to_string()
                                };

                                let max_chars = config.max_body_chars;
                                let truncated = &body_text[..body_text.len().min(max_chars)];

                                let content = format!(
                                    "Email received.\nFrom: {}\nSubject: {}\nDate: {}\n\n{}",
                                    sender, subject, date, truncated
                                );

                                messages.push(EmailMessage {
                                    sender,
                                    subject,
                                    message_id,
                                    content,
                                    uid: uid_str.clone(),
                                    date,
                                });
                            }
                        }
                    }
                }
            }

            // Mark as seen if configured
            if config.mark_seen {
                let _ = session.uid_store(&uid_str, "+FLAGS (\\Seen)");
            }
        }

        // Logout
        let _ = session.logout();

        Ok(messages)
    }

    /// Static version of html_to_text for use in blocking context
    fn html_to_text_static(html: &str) -> String {
        let text = html
            .replace("<br>", "\n")
            .replace("<br/>", "\n")
            .replace("<br />", "\n")
            .replace("</p>", "\n");

        let re = regex::Regex::new(r"<[^>]+>").unwrap_or_else(|_| regex::Regex::new(r"").unwrap());
        let text = re.replace_all(&text, "");

        html_escape::decode_html_entities(&text).to_string()
    }

    /// Send email via SMTP (blocking)
    fn smtp_send_blocking(
        config: &EmailConfig,
        to_addr: &str,
        subject: &str,
        content: &str,
        in_reply_to: Option<&str>,
        attachments: Vec<(String, Vec<u8>, String)>,
        is_html: bool,
    ) -> Result<()> {
        use lettre::message::header::ContentType;
        use lettre::message::{Attachment, MultiPart, SinglePart};
        use lettre::transport::smtp::authentication::Credentials;
        use lettre::{Message, SmtpTransport, Transport};

        // Build email message
        let from_addr = if !config.from_address.is_empty() {
            config.from_address.clone()
        } else if !config.smtp_username.is_empty() {
            config.smtp_username.clone()
        } else {
            config.imap_username.clone()
        };

        let mut email_builder =
            Message::builder()
                .from(from_addr.parse().map_err(|e| {
                    ChannelError::InvalidConfig(format!("Invalid from address: {}", e))
                })?)
                .to(to_addr.parse().map_err(|e| {
                    ChannelError::InvalidConfig(format!("Invalid to address: {}", e))
                })?)
                .subject(subject);

        // Add In-Reply-To and References headers if available
        if let Some(reply_to_id) = in_reply_to {
            email_builder = email_builder.in_reply_to(reply_to_id.to_string());
            email_builder = email_builder.references(reply_to_id.to_string());
        }

        let email = if !attachments.is_empty() || is_html {
            let builder = MultiPart::mixed();

            // Content part
            let mixed = if is_html {
                let alt = MultiPart::alternative()
                    .singlepart(SinglePart::plain(Self::html_to_text_static(content)))
                    .singlepart(SinglePart::html(content.to_string()));
                builder.multipart(alt)
            } else {
                builder.singlepart(SinglePart::plain(content.to_string()))
            };

            // Attachments
            let mixed = attachments.into_iter().fold(mixed, |acc, (filename, data, content_type)| {
                let mime = content_type
                    .parse()
                    .unwrap_or_else(|_| ContentType::parse("application/octet-stream").unwrap());
                acc.singlepart(Attachment::new(filename).body(data, mime))
            });

            email_builder
                .multipart(mixed)
                .map_err(|e| ChannelError::SendError(format!("Build multipart email: {}", e)))?
        } else {
            email_builder
                .header(ContentType::TEXT_PLAIN)
                .body(content.to_string())
                .map_err(|e| ChannelError::SendError(format!("Build email: {}", e)))?
        };

        // Build SMTP transport
        let credentials =
            Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());

        let transport = if config.smtp_use_ssl {
            SmtpTransport::relay(&config.smtp_host)
                .map_err(|e| ChannelError::ConnectionError(format!("SMTP relay: {}", e)))?
                .credentials(credentials)
                .port(config.smtp_port)
                .build()
        } else if config.smtp_use_tls {
            SmtpTransport::starttls_relay(&config.smtp_host)
                .map_err(|e| ChannelError::ConnectionError(format!("SMTP starttls: {}", e)))?
                .credentials(credentials)
                .port(config.smtp_port)
                .build()
        } else {
            SmtpTransport::builder_dangerous(&config.smtp_host)
                .credentials(credentials)
                .port(config.smtp_port)
                .build()
        };

        // Send email
        transport
            .send(&email)
            .map_err(|e| ChannelError::SendError(format!("SMTP send: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl ChannelHandler for EmailHandler {
    /// Set the inbound message sender
    fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn is_running(&self) -> bool {
        *self.running.blocking_read()
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(ChannelError::NotConfigured(
                "Email channel not enabled".to_string(),
            ));
        }

        if !self.config.consent_granted {
            warn!(
                "Email channel disabled: consent_granted is false. \
                Set channels.email.consentGranted=true after explicit user permission."
            );
            return Err(ChannelError::NotConfigured(
                "Email consent not granted".to_string(),
            ));
        }

        self.validate_config()?;

        let inbound_tx = self.inbound_tx.clone().ok_or_else(|| {
            ChannelError::NotConfigured("Inbound message sender not set".to_string())
        })?;

        *self.running.write().await = true;
        info!("Starting Email channel (IMAP polling mode)...");

        // Spawn IMAP polling task
        let config = self.config.clone();
        let running = self.running.clone();
        let processed_uids = self.processed_uids.clone();
        let last_subjects = self.last_subjects.clone();
        let last_message_ids = self.last_message_ids.clone();
        let allow_from = self.config.allow_from.clone();

        let poll_task = tokio::spawn(async move {
            let poll_seconds = config.poll_interval_seconds.max(5);

            while *running.read().await {
                // Use spawn_blocking for the blocking IMAP operations
                let cfg = config.clone();
                let processed = processed_uids.read().await.clone();

                match tokio::task::spawn_blocking(move || {
                    Self::fetch_messages_blocking(&cfg, &processed)
                })
                .await
                {
                    Ok(Ok(messages)) => {
                        // Update processed UIDs
                        {
                            let mut uids = processed_uids.write().await;
                            for msg in &messages {
                                uids.insert(msg.uid.clone());
                                if uids.len() > 100000 {
                                    uids.clear();
                                }
                            }
                        }

                        // Process messages
                        for email_msg in messages {
                            // Check permissions
                            if !allow_from.is_empty() && !allow_from.contains(&email_msg.sender) {
                                debug!(
                                    "Skipping email from unauthorized sender: {}",
                                    email_msg.sender
                                );
                                continue;
                            }

                            // Store subject and message_id for replies
                            if !email_msg.subject.is_empty() {
                                last_subjects
                                    .write()
                                    .await
                                    .insert(email_msg.sender.clone(), email_msg.subject.clone());
                            }
                            if !email_msg.message_id.is_empty() {
                                last_message_ids
                                    .write()
                                    .await
                                    .insert(email_msg.sender.clone(), email_msg.message_id.clone());
                            }

                            // Send inbound message
                            let mut metadata = HashMap::new();
                            metadata.insert(
                                "message_id".to_string(),
                                serde_json::json!(email_msg.message_id),
                            );
                            metadata.insert(
                                "subject".to_string(),
                                serde_json::json!(email_msg.subject),
                            );
                            metadata.insert("date".to_string(), serde_json::json!(email_msg.date));
                            metadata.insert("uid".to_string(), serde_json::json!(email_msg.uid));

                            let inbound = InboundMessage {
                                channel: "email".to_string(),
                                sender_id: email_msg.sender.clone(),
                                chat_id: email_msg.sender.clone(),
                                content: email_msg.content.clone(),
                                timestamp: chrono::Utc::now(),
                                media: Vec::new(),
                                metadata,
                            };

                            if let Err(e) = inbound_tx.send(inbound).await {
                                error!("Failed to send inbound email message: {}", e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!("Email polling error: {}", e);
                    }
                    Err(e) => {
                        error!("Email polling task error: {}", e);
                    }
                }

                tokio::time::sleep(tokio::time::Duration::from_secs(poll_seconds)).await;
            }

            info!("Email polling task stopped");
        });

        *self.poll_task.lock().await = Some(poll_task);
        info!("Email channel started successfully");

        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;

        let mut task = self.poll_task.lock().await;
        if let Some(handle) = task.take() {
            handle.abort();
            let _ = handle.await;
        }

        info!("Email channel stopped");
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<()> {
        if !self.config.consent_granted {
            warn!("Skip email send: consent_granted is false");
            return Ok(());
        }

        let force_send = msg
            .metadata
            .get("force_send")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if !self.config.auto_reply_enabled && !force_send {
            info!("Skip automatic email reply: auto_reply_enabled is false");
            return Ok(());
        }

        let to_addr = msg.chat_id.trim();
        if to_addr.is_empty() {
            warn!("Email channel missing recipient address");
            return Ok(());
        }

        // Get subject
        let base_subject = self
            .last_subjects
            .read()
            .await
            .get(to_addr)
            .cloned()
            .unwrap_or_else(|| "agent-diva reply".to_string());

        let mut subject = self.reply_subject(&base_subject);

        // Override subject if provided in metadata
        if let Some(override_subject) = msg.metadata.get("subject").and_then(|v| v.as_str()) {
            let trimmed = override_subject.trim();
            if !trimmed.is_empty() {
                subject = trimmed.to_string();
            }
        }

        // Detect HTML
        let is_html = msg
            .metadata
            .get("format")
            .and_then(|v| v.as_str())
            .map(|s| s.eq_ignore_ascii_case("html"))
            .unwrap_or_else(|| {
                msg.content.trim_start().starts_with("<!DOCTYPE html")
                    || msg.content.trim_start().starts_with("<html")
            });

        // Fetch attachments
        let mut attachments = Vec::new();
        for url in &msg.media {
            match reqwest::get(url).await {
                Ok(resp) => {
                    let filename = url
                        .split('/')
                        .last()
                        .unwrap_or("attachment")
                        .to_string();
                    let content_type = resp
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("application/octet-stream")
                        .to_string();
                    match resp.bytes().await {
                        Ok(bytes) => {
                            attachments.push((filename, bytes.to_vec(), content_type));
                        }
                        Err(e) => {
                            error!("Failed to read attachment content {}: {}", url, e);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to fetch attachment {}: {}", url, e);
                }
            }
        }

        // Get In-Reply-To and References
        let in_reply_to = self.last_message_ids.read().await.get(to_addr).cloned();

        // Send email in blocking thread
        let config = self.config.clone();
        let content = msg.content.clone();
        let to_addr_owned = to_addr.to_string();

        tokio::task::spawn_blocking(move || {
            Self::smtp_send_blocking(
                &config,
                &to_addr_owned,
                &subject,
                &content,
                in_reply_to.as_deref(),
                attachments,
                is_html,
            )
        })
        .await
        .map_err(|e| ChannelError::SendError(format!("Join error: {}", e)))??;

        debug!("Email sent to {}", to_addr);
        Ok(())
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.base.is_allowed(sender_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::config::schema::Config;

    #[test]
    fn test_email_handler_new() {
        let mut email_config = EmailConfig::default();
        email_config.enabled = true;

        let config = Config::default();
        let handler = EmailHandler::new(email_config, config);

        assert_eq!(handler.name(), "email");
        assert!(!handler.is_running());
    }

    #[test]
    fn test_validate_config_missing_fields() {
        let email_config = EmailConfig::default();
        let config = Config::default();
        let handler = EmailHandler::new(email_config, config);

        let result = handler.validate_config();
        assert!(result.is_err());

        if let Err(ChannelError::InvalidConfig(msg)) = result {
            assert!(msg.contains("imap_host"));
            assert!(msg.contains("smtp_host"));
        }
    }

    #[test]
    fn test_reply_subject() {
        let email_config = EmailConfig::default();
        let config = Config::default();
        let handler = EmailHandler::new(email_config, config);

        assert_eq!(handler.reply_subject("Test"), "Re: Test");
        assert_eq!(handler.reply_subject("re: Test"), "re: Test");
        assert_eq!(handler.reply_subject("Re: Test"), "Re: Test");
        assert_eq!(handler.reply_subject(""), "Re: agent-diva reply");
    }

    #[test]
    fn test_is_allowed() {
        let mut email_config = EmailConfig::default();
        email_config.allow_from = vec!["user@example.com".to_string()];

        let config = Config::default();
        let handler = EmailHandler::new(email_config, config);

        assert!(handler.is_allowed("user@example.com"));
        assert!(!handler.is_allowed("other@example.com"));
    }

    #[test]
    fn test_html_to_text() {
        let email_config = EmailConfig::default();
        let config = Config::default();
        let handler = EmailHandler::new(email_config, config);

        let html = "<p>Hello<br>World</p>";
        let text = handler.html_to_text(html);
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
        assert!(!text.contains("<p>"));
    }

    #[test]
    fn test_parse_email() {
        let email_config = EmailConfig::default();
        let config = Config::default();
        let handler = EmailHandler::new(email_config, config);

        // Simple test email
        let email_bytes = b"From: test@example.com\r\n\
            Subject: Test Email\r\n\
            Message-ID: <12345@example.com>\r\n\
            Date: Mon, 01 Jan 2024 00:00:00 +0000\r\n\
            \r\n\
            This is a test email body.";

        let result = handler.parse_email(email_bytes, "uid123".to_string());
        assert!(result.is_some());

        let email = result.unwrap();
        assert_eq!(email.sender, "test@example.com");
        assert_eq!(email.subject, "Test Email");
        assert_eq!(email.uid, "uid123");
        assert!(email.content.contains("test email body"));
    }
}
