//! Matrix channel integration (polling sync + text/media delivery).

use crate::base::{BaseChannel, ChannelError, ChannelHandler, Result};
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::{Config, MatrixConfig};
use agent_diva_core::utils::safe_filename;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct MatrixHandler {
    config: MatrixConfig,
    base: BaseChannel,
    running: Arc<AtomicBool>,
    processed_ids: Arc<RwLock<VecDeque<String>>>,
    since_token: Arc<RwLock<Option<String>>>,
    client: reqwest::Client,
    sync_task: Option<JoinHandle<()>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
}

impl MatrixHandler {
    pub fn new(config: MatrixConfig, base_config: Config) -> Self {
        Self {
            base: BaseChannel::with_default_policy(
                "matrix",
                base_config,
                config.allow_from.clone(),
                true, // deny_by_default = true
            ),
            config,
            running: Arc::new(AtomicBool::new(false)),
            processed_ids: Arc::new(RwLock::new(VecDeque::with_capacity(2000))),
            since_token: Arc::new(RwLock::new(None)),
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(45))
                .build()
                .expect("failed to build matrix http client"),
            sync_task: None,
            shutdown_tx: None,
            inbound_tx: None,
        }
    }

    fn validate_config(&self) -> Result<()> {
        if self.config.user_id.trim().is_empty() {
            return Err(ChannelError::InvalidConfig(
                "Matrix user_id not configured".to_string(),
            ));
        }
        if self.config.access_token.trim().is_empty() {
            return Err(ChannelError::InvalidConfig(
                "Matrix access_token not configured".to_string(),
            ));
        }

        // Warn if not running end-to-end encryption and no explicit allow_from is provided.
        // It's dangerous to run an open bot on a public Matrix federation without E2EE or filters.
        if !self.config.e2ee_enabled && self.config.allow_from.is_empty() {
            warn!("Matrix is configured with e2e_enabled=false and an empty allow_from list! With deny-by-default, the bot will ignore ALL incoming messages.");
        }

        Ok(())
    }

    fn media_dir() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".agent-diva")
            .join("media")
            .join("matrix")
    }

    async fn is_processed(&self, event_id: &str) -> bool {
        let ids = self.processed_ids.read().await;
        ids.contains(&event_id.to_string())
    }

    async fn mark_processed(&self, event_id: String) {
        let mut ids = self.processed_ids.write().await;
        if ids.len() >= 2000 {
            ids.pop_front();
        }
        ids.push_back(event_id);
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.bearer_auth(self.config.access_token.clone())
    }

    fn matrix_client_url(&self, path: &str) -> String {
        format!(
            "{}/_matrix/client/v3/{}",
            self.config.homeserver.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    fn matrix_media_url(&self, path: &str) -> String {
        format!(
            "{}/_matrix/media/v3/{}",
            self.config.homeserver.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }

    async fn sync_once(&self) -> Result<()> {
        let mut req = self.auth(
            self.client
                .get(self.matrix_client_url("sync"))
                .query(&[("timeout", self.config.sync_timeout_ms.to_string())]),
        );

        if let Some(since) = self.since_token.read().await.clone() {
            req = req.query(&[("since", since)]);
        }

        let resp = req
            .send()
            .await
            .map_err(|e| ChannelError::ConnectionError(format!("Matrix sync failed: {}", e)))?;
        if !resp.status().is_success() {
            let code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ChannelError::ConnectionError(format!(
                "Matrix sync status {}: {}",
                code, body
            )));
        }
        let payload: Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::ConnectionError(format!("Matrix sync parse: {}", e)))?;

        if let Some(next_batch) = payload.get("next_batch").and_then(|v| v.as_str()) {
            *self.since_token.write().await = Some(next_batch.to_string());
        }

        let Some(rooms) = payload
            .get("rooms")
            .and_then(|v| v.get("join"))
            .and_then(|v| v.as_object())
        else {
            return Ok(());
        };

        for (room_id, room_data) in rooms {
            let events = room_data
                .get("timeline")
                .and_then(|v| v.get("events"))
                .and_then(|v| v.as_array());
            let Some(events) = events else { continue };

            for event in events {
                if event.get("type").and_then(|v| v.as_str()) != Some("m.room.message") {
                    continue;
                }

                let sender = event
                    .get("sender")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                if sender == self.config.user_id {
                    continue;
                }
                if !self.base.is_allowed(sender) {
                    continue;
                }
                if !self.config.group_allow_from.is_empty()
                    && !self.config.group_allow_from.contains(room_id)
                {
                    continue;
                }

                let event_id = event
                    .get("event_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                if event_id.is_empty() || self.is_processed(&event_id).await {
                    continue;
                }
                self.mark_processed(event_id.clone()).await;

                let msg_type = event
                    .get("content")
                    .and_then(|v| v.get("msgtype"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("m.text")
                    .to_string();
                let mut content = event
                    .get("content")
                    .and_then(|v| v.get("body"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let mut media = Vec::new();
                if let Some(mxc) = event
                    .get("content")
                    .and_then(|v| v.get("url"))
                    .and_then(|v| v.as_str())
                {
                    if let Some(path) = self.download_mxc(mxc, &msg_type).await {
                        media.push(path);
                    } else if content.is_empty() {
                        content = format!("[{}: download failed]", msg_type);
                    }
                }
                if content.is_empty() && !media.is_empty() {
                    content = format!("[{}]", msg_type);
                }
                if content.is_empty() {
                    continue;
                }

                let inbound = InboundMessage::new("matrix", sender, room_id, content)
                    .with_metadata("message_id", json!(event_id))
                    .with_metadata("msg_type", json!(msg_type));

                let mut inbound = inbound;
                for m in media {
                    inbound = inbound.with_media(m);
                }
                if let Some(tx) = &self.inbound_tx {
                    if let Err(e) = tx.send(inbound).await {
                        error!("Failed to publish Matrix inbound: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn download_mxc(&self, mxc: &str, msg_type: &str) -> Option<String> {
        let mxc = mxc.trim();
        let rest = mxc.strip_prefix("mxc://")?;
        let (server, media_id) = rest.split_once('/')?;

        let url = self.matrix_media_url(&format!("download/{}/{}", server, media_id));
        let resp = self.auth(self.client.get(url)).send().await.ok()?;
        if !resp.status().is_success() {
            return None;
        }
        if Self::is_media_too_large(resp.content_length(), None, self.config.max_media_bytes) {
            warn!(
                "Matrix inbound media exceeds max_media_bytes before download: {}",
                mxc
            );
            return None;
        }
        let bytes = resp.bytes().await.ok()?;
        if Self::is_media_too_large(None, Some(bytes.len()), self.config.max_media_bytes) {
            warn!(
                "Matrix inbound media exceeds max_media_bytes after download: {}",
                mxc
            );
            return None;
        }
        let ext = match msg_type {
            "m.image" => ".jpg",
            "m.audio" => ".ogg",
            "m.video" => ".mp4",
            _ => ".bin",
        };
        let filename = format!(
            "{}_{}{}",
            safe_filename(server),
            safe_filename(media_id),
            ext
        );
        let dir = Self::media_dir();
        tokio::fs::create_dir_all(&dir).await.ok()?;
        let path = dir.join(filename);
        tokio::fs::write(&path, &bytes).await.ok()?;
        Some(path.to_string_lossy().to_string())
    }

    fn is_media_too_large(
        content_length: Option<u64>,
        downloaded_len: Option<usize>,
        max_media_bytes: usize,
    ) -> bool {
        let max_bytes = max_media_bytes as u64;
        if content_length.is_some_and(|len| len > max_bytes) {
            return true;
        }
        downloaded_len.is_some_and(|len| len as u64 > max_bytes)
    }

    async fn send_text(&self, room_id: &str, text: &str) -> Result<()> {
        if text.trim().is_empty() {
            return Ok(());
        }
        let txn_id = Uuid::new_v4().to_string();
        let url =
            self.matrix_client_url(&format!("rooms/{}/send/m.room.message/{}", room_id, txn_id));
        let body = json!({
            "msgtype": "m.text",
            "body": text,
        });
        let resp = self
            .auth(self.client.put(url).json(&body))
            .send()
            .await
            .map_err(|e| {
                ChannelError::SendFailed(format!("Matrix text send request failed: {}", e))
            })?;
        if !resp.status().is_success() {
            let code = resp.status();
            let detail = resp.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed(format!(
                "Matrix text send failed {}: {}",
                code, detail
            )));
        }
        Ok(())
    }

    async fn upload_media(&self, path: &Path) -> Result<(String, String, String)> {
        let metadata = tokio::fs::metadata(path)
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Matrix media stat failed: {}", e)))?;
        if metadata.len() > self.config.max_media_bytes as u64 {
            return Err(ChannelError::SendFailed(format!(
                "Matrix media exceeds max_media_bytes: {}",
                path.display()
            )));
        }

        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Matrix media read failed: {}", e)))?;
        let filename = safe_filename(
            path.file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("attachment.bin"),
        );
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        let url = self.matrix_media_url("upload");
        let resp = self
            .auth(
                self.client
                    .post(url)
                    .query(&[("filename", filename.clone())])
                    .header("Content-Type", mime.clone())
                    .body(bytes),
            )
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Matrix media upload failed: {}", e)))?;

        if !resp.status().is_success() {
            let code = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed(format!(
                "Matrix media upload failed {}: {}",
                code, body
            )));
        }
        let payload: Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Matrix upload parse failed: {}", e)))?;
        let mxc = payload
            .get("content_uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ChannelError::SendFailed("Matrix upload missing content_uri".to_string())
            })?
            .to_string();
        Ok((mxc, filename, mime))
    }

    async fn send_media(&self, room_id: &str, media_path: &str) -> Result<()> {
        let path = PathBuf::from(media_path);
        if !path.exists() {
            return Err(ChannelError::SendFailed(format!(
                "Matrix media file does not exist: {}",
                media_path
            )));
        }
        let (mxc, filename, mime) = self.upload_media(&path).await?;
        let msgtype = if mime.starts_with("image/") {
            "m.image"
        } else if mime.starts_with("audio/") {
            "m.audio"
        } else if mime.starts_with("video/") {
            "m.video"
        } else {
            "m.file"
        };

        let txn_id = Uuid::new_v4().to_string();
        let url =
            self.matrix_client_url(&format!("rooms/{}/send/m.room.message/{}", room_id, txn_id));
        let body = json!({
            "msgtype": msgtype,
            "body": filename,
            "filename": filename,
            "url": mxc,
            "info": {"mimetype": mime}
        });
        let resp = self
            .auth(self.client.put(url).json(&body))
            .send()
            .await
            .map_err(|e| {
                ChannelError::SendFailed(format!("Matrix media send request failed: {}", e))
            })?;
        if !resp.status().is_success() {
            let code = resp.status();
            let detail = resp.text().await.unwrap_or_default();
            return Err(ChannelError::SendFailed(format!(
                "Matrix media send failed {}: {}",
                code, detail
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl ChannelHandler for MatrixHandler {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    async fn start(&mut self) -> Result<()> {
        self.validate_config()?;
        if self.running.load(Ordering::Acquire) {
            return Ok(());
        }
        self.running.store(true, Ordering::Release);

        if !self.config.e2ee_enabled {
            warn!("Matrix E2EE disabled; encrypted room payloads may be skipped.");
        }

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let running = self.running.clone();
        let client = self.client.clone();
        let config = self.config.clone();
        let base = BaseChannel::new(
            "matrix",
            self.base.config.clone(),
            self.base.allow_from.clone(),
        );
        let processed_ids = self.processed_ids.clone();
        let since_token = self.since_token.clone();
        let inbound_tx = self.inbound_tx.clone();
        self.sync_task = Some(tokio::spawn(async move {
            let handler = MatrixHandler {
                config,
                base,
                running,
                processed_ids,
                since_token,
                client,
                sync_task: None,
                shutdown_tx: None,
                inbound_tx,
            };

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => break,
                    _ = tokio::time::sleep(Duration::from_millis(200)) => {
                        if let Err(e) = handler.sync_once().await {
                            debug!("Matrix sync warning: {}", e);
                            tokio::time::sleep(Duration::from_secs(2)).await;
                        }
                    }
                }
            }
            handler.running.store(false, Ordering::Release);
            info!("Matrix sync loop stopped");
        }));

        info!("Matrix channel started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::Release);
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        if let Some(task) = self.sync_task.take() {
            let _ = tokio::time::timeout(
                Duration::from_secs(self.config.sync_stop_grace_seconds),
                task,
            )
            .await;
        }
        info!("Matrix channel stopped");
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<()> {
        if !self.running.load(Ordering::Acquire) {
            return Err(ChannelError::NotRunning(
                "Matrix channel not running".to_string(),
            ));
        }
        for media in &msg.media {
            if let Err(e) = self.send_media(&msg.chat_id, media).await {
                warn!("Matrix media send failed for {}: {}", media, e);
            }
        }
        self.send_text(&msg.chat_id, &msg.content).await?;
        Ok(())
    }

    fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.base.is_allowed(sender_id)
    }
}

#[cfg(test)]
mod tests {
    use super::MatrixHandler;

    #[test]
    fn test_matrix_media_limit_blocks_large_declared_length() {
        assert!(MatrixHandler::is_media_too_large(
            Some(1_048_577),
            None,
            1_048_576
        ));
    }

    #[test]
    fn test_matrix_media_limit_blocks_large_download_without_declared_length() {
        assert!(MatrixHandler::is_media_too_large(
            None,
            Some(1_048_577),
            1_048_576
        ));
    }

    #[test]
    fn test_matrix_media_limit_allows_within_limit() {
        assert!(!MatrixHandler::is_media_too_large(
            Some(1_048_576),
            Some(1_048_576),
            1_048_576
        ));
    }
}
