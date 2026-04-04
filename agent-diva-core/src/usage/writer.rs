//! Token usage writer service with async background writing

use crate::usage::types::{get_model_pricing, TokenUsageRecord};
use crate::Result;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Batch insert threshold
const BATCH_THRESHOLD: usize = 10;
/// Timeout for flushing batch (seconds)
const FLUSH_TIMEOUT_SECS: u64 = 2;

/// Token usage writer with background async processing
pub struct TokenUsageWriter {
    /// Channel sender for enqueueing records
    queue_tx: mpsc::UnboundedSender<TokenUsageRecord>,
    /// Database path for persistence
    db_path: PathBuf,
    /// Flag indicating if the writer is running
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl TokenUsageWriter {
    /// Create and start the token usage writer
    pub fn start(db_path: PathBuf) -> Self {
        let (queue_tx, queue_rx) = mpsc::unbounded_channel();
        let running = Arc::new(std::sync::atomic::AtomicBool::new(true));

        let running_clone = running.clone();
        let db_path_clone = db_path.clone();

        tokio::spawn(async move {
            Self::writer_loop(db_path_clone, queue_rx, running_clone).await;
        });

        info!("Token usage writer started at {:?}", db_path);
        Self {
            queue_tx,
            db_path,
            running,
        }
    }

    /// Record token usage (non-blocking)
    pub fn record(&self, record: TokenUsageRecord) {
        let record = if record.estimated_cost == 0.0 {
            let pricing = get_model_pricing(&record.model);
            let cost = pricing.calculate_cost(
                record.input_tokens,
                record.output_tokens,
                record.cache_creation_tokens,
                record.cache_read_tokens,
            );
            record.with_cost(cost)
        } else {
            record
        };

        if let Err(e) = self.queue_tx.send(record) {
            warn!("Failed to enqueue token usage record: {}", e);
        }
    }

    /// Check if writer is running
    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Stop the writer and flush remaining records
    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
        info!("Token usage writer stopped");
    }

    /// Get the database path
    pub fn db_path(&self) -> &PathBuf {
        &self.db_path
    }

    async fn writer_loop(
        db_path: PathBuf,
        mut queue_rx: mpsc::UnboundedReceiver<TokenUsageRecord>,
        running: Arc<std::sync::atomic::AtomicBool>,
    ) {
        let db_path_for_init = db_path.clone();
        let init_result =
            tokio::task::spawn_blocking(move || Self::init_database(&db_path_for_init)).await;

        match init_result {
            Ok(Ok(_)) => debug!("Database initialized"),
            Ok(Err(e)) => {
                error!("Failed to initialize token usage database: {}", e);
                return;
            }
            Err(e) => {
                error!("Task join error: {}", e);
                return;
            }
        }

        let mut batch: Vec<TokenUsageRecord> = Vec::new();

        loop {
            if !running.load(std::sync::atomic::Ordering::Relaxed) && batch.is_empty() {
                break;
            }

            match tokio::time::timeout(
                std::time::Duration::from_secs(FLUSH_TIMEOUT_SECS),
                queue_rx.recv(),
            )
            .await
            {
                Ok(Some(record)) => {
                    batch.push(record);
                    if batch.len() >= BATCH_THRESHOLD {
                        let db_path_clone = db_path.clone();
                        let batch_clone = std::mem::take(&mut batch);
                        if let Err(e) = tokio::task::spawn_blocking(move || {
                            Self::flush_batch_sync(&db_path_clone, &batch_clone)
                        })
                        .await
                        {
                            error!("Flush task error: {}", e);
                        }
                    }
                }
                Ok(None) => {
                    if !batch.is_empty() {
                        let db_path_clone = db_path.clone();
                        let batch_clone = batch.clone();
                        if let Err(e) = tokio::task::spawn_blocking(move || {
                            Self::flush_batch_sync(&db_path_clone, &batch_clone)
                        })
                        .await
                        {
                            error!("Final flush error: {}", e);
                        }
                    }
                    break;
                }
                Err(_) => {
                    if !batch.is_empty() {
                        let db_path_clone = db_path.clone();
                        let batch_clone = std::mem::take(&mut batch);
                        if let Err(e) = tokio::task::spawn_blocking(move || {
                            Self::flush_batch_sync(&db_path_clone, &batch_clone)
                        })
                        .await
                        {
                            error!("Timeout flush error: {}", e);
                        }
                    }
                }
            }
        }

        debug!("Token usage writer loop exited");
    }

    fn init_database(db_path: &PathBuf) -> Result<()> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-64000;",
        )?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS token_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                session_id TEXT NOT NULL,
                endpoint_name TEXT NOT NULL,
                model TEXT NOT NULL,
                operation_type TEXT NOT NULL,
                operation_detail TEXT,
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                cache_creation_tokens INTEGER DEFAULT 0,
                cache_read_tokens INTEGER DEFAULT 0,
                context_tokens INTEGER DEFAULT 0,
                iteration INTEGER DEFAULT 0,
                channel TEXT,
                user_id TEXT,
                agent_profile_id TEXT DEFAULT 'default',
                estimated_cost REAL DEFAULT 0
            );

            CREATE INDEX IF NOT EXISTS idx_token_usage_ts ON token_usage(timestamp);
            CREATE INDEX IF NOT EXISTS idx_token_usage_session ON token_usage(session_id);
            CREATE INDEX IF NOT EXISTS idx_token_usage_endpoint ON token_usage(endpoint_name);
            CREATE INDEX IF NOT EXISTS idx_token_usage_model ON token_usage(model);
            CREATE INDEX IF NOT EXISTS idx_token_usage_op ON token_usage(operation_type);
            CREATE INDEX IF NOT EXISTS idx_token_usage_channel ON token_usage(channel);",
        )?;

        debug!("Token usage database initialized at {:?}", db_path);
        Ok(())
    }

    fn flush_batch_sync(db_path: &PathBuf, batch: &[TokenUsageRecord]) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        let mut conn = Connection::open(db_path)?;
        let tx = conn.transaction()?;

        for record in batch {
            tx.execute(
                "INSERT INTO token_usage (
                    timestamp, session_id, endpoint_name, model, operation_type,
                    operation_detail, input_tokens, output_tokens,
                    cache_creation_tokens, cache_read_tokens, context_tokens,
                    iteration, channel, user_id, agent_profile_id, estimated_cost
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
                rusqlite::params![
                    record.timestamp.to_rfc3339(),
                    record.session_id,
                    record.endpoint_name,
                    record.model,
                    record.operation_type.to_string(),
                    record.operation_detail,
                    record.input_tokens,
                    record.output_tokens,
                    record.cache_creation_tokens,
                    record.cache_read_tokens,
                    record.context_tokens,
                    record.iteration,
                    record.channel,
                    record.user_id,
                    record.agent_profile_id,
                    record.estimated_cost,
                ],
            )?;
        }

        tx.commit()?;
        debug!("Flushed {} token usage records", batch.len());
        Ok(())
    }
}

impl Drop for TokenUsageWriter {
    fn drop(&mut self) {
        self.stop();
    }
}

/// In-memory token usage aggregator for real-time stats
pub struct InMemoryAggregator {
    total_tokens: std::sync::atomic::AtomicI64,
    input_tokens: std::sync::atomic::AtomicI64,
    output_tokens: std::sync::atomic::AtomicI64,
    request_count: std::sync::atomic::AtomicU64,
    total_cost: std::sync::atomic::AtomicU64,
}

impl InMemoryAggregator {
    pub fn new() -> Self {
        Self {
            total_tokens: std::sync::atomic::AtomicI64::new(0),
            input_tokens: std::sync::atomic::AtomicI64::new(0),
            output_tokens: std::sync::atomic::AtomicI64::new(0),
            request_count: std::sync::atomic::AtomicU64::new(0),
            total_cost: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn add(&self, input: i64, output: i64, cost: f64) {
        self.input_tokens
            .fetch_add(input, std::sync::atomic::Ordering::Relaxed);
        self.output_tokens
            .fetch_add(output, std::sync::atomic::Ordering::Relaxed);
        self.total_tokens
            .fetch_add(input + output, std::sync::atomic::Ordering::Relaxed);
        self.request_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.total_cost
            .fetch_add((cost * 100.0) as u64, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> InMemoryStats {
        InMemoryStats {
            total_tokens: self.total_tokens.load(std::sync::atomic::Ordering::Relaxed),
            input_tokens: self.input_tokens.load(std::sync::atomic::Ordering::Relaxed),
            output_tokens: self
                .output_tokens
                .load(std::sync::atomic::Ordering::Relaxed),
            request_count: self
                .request_count
                .load(std::sync::atomic::Ordering::Relaxed),
            total_cost: self.total_cost.load(std::sync::atomic::Ordering::Relaxed) as f64 / 100.0,
        }
    }

    pub fn reset(&self) {
        self.total_tokens
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.input_tokens
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.output_tokens
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.request_count
            .store(0, std::sync::atomic::Ordering::Relaxed);
        self.total_cost
            .store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Default for InMemoryAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct InMemoryStats {
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub request_count: u64,
    pub total_cost: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::types::OperationType;
    use tempfile::TempDir;

    #[test]
    fn test_in_memory_aggregator() {
        let agg = InMemoryAggregator::new();
        agg.add(1000, 500, 0.01);
        agg.add(2000, 1000, 0.05);

        let stats = agg.get_stats();
        assert_eq!(stats.total_tokens, 4500);
        assert_eq!(stats.input_tokens, 3000);
        assert_eq!(stats.output_tokens, 1500);
        assert_eq!(stats.request_count, 2);
        assert!((stats.total_cost - 0.06).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_writer_basic() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("token_usage.db");

        let writer = TokenUsageWriter::start(db_path.clone());

        let record = TokenUsageRecord::new(
            "test:session-1",
            "openrouter",
            "claude-3-5-sonnet",
            OperationType::Chat,
            1000,
            500,
        );

        writer.record(record);

        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        writer.stop();

        assert!(db_path.exists());
    }
}
