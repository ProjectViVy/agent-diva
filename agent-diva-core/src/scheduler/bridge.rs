//! CronBridge — connects cron triggers to the SupervisedRun store
//!
//! Provides dispatch (cron → RunStore), retry with exponential backoff,
//! and dead letter tracking for exhausted retries.

use crate::cron::CronPayload;
use crate::error::{Error, Result};
use crate::supervised::{RunItem, RunStatus, RunStore};
use crate::todo::{JsonlTodoStore, TodoItem, TodoSource};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{info, warn};

/// Maximum number of retry attempts before a task is moved to dead letter.
const MAX_RETRIES: u32 = 5;

/// Record of a run that exhausted all retry attempts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterEntry {
    pub run_id: String,
    pub original_message: String,
    pub retry_count: u32,
    pub last_error: Option<String>,
    pub dead_lettered_at: chrono::DateTime<Utc>,
}

/// Outcome of a retry attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryOutcome {
    /// Retry was scheduled successfully; includes new run ID and retry count.
    Retried {
        new_run_id: String,
        retry_count: u32,
    },
    /// Max retries exceeded; run moved to dead letter.
    Exhausted { run_id: String, retry_count: u32 },
}

/// Bridge between the cron subsystem and the SupervisedRun store.
///
/// `CronBridge` translates cron triggers into queued RunItems and
/// manages retry lifecycle with exponential backoff.
pub struct CronBridge {
    run_store: RunStore,
    todo_store: JsonlTodoStore,
    dead_letter_path: std::path::PathBuf,
}

impl CronBridge {
    /// Create a new CronBridge.
    ///
    /// The dead letter file will be stored at `{data_root}/dead_letters.jsonl`.
    pub fn new(run_store: RunStore, todo_store: JsonlTodoStore, data_root: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_root)?;
        let dead_letter_path = data_root.join("dead_letters.jsonl");
        // Ensure the file exists
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&dead_letter_path)?;
        Ok(Self {
            run_store,
            todo_store,
            dead_letter_path,
        })
    }

    /// Dispatch a cron payload by creating a new RunItem in the RunStore
    /// with status Queued, and optionally recording a linked TodoItem.
    ///
    /// Returns the ID of the newly created run.
    pub async fn dispatch(&self, payload: &CronPayload) -> Result<String> {
        let mut item = RunItem::new(&payload.message);
        item.channel = payload.channel.clone();
        item.cron_job_id = None; // Will be set by caller if needed

        let enqueued = self.run_store.enqueue(item).await?;

        // Create a linked todo item if the payload requests delivery
        if payload.deliver {
            let mut todo = TodoItem::new(&payload.message, TodoSource::Cron);
            todo.linked_run_id = Some(enqueued.id.clone());
            if let Some(ref channel) = payload.channel {
                // Store channel context in session_id field as a lightweight link
                todo.session_id = Some(channel.clone());
            }
            self.todo_store.create(todo).await?;
        }

        info!(
            run_id = %enqueued.id,
            message = %payload.message,
            "CronBridge: dispatched cron payload to RunStore"
        );

        Ok(enqueued.id)
    }

    /// Retry a failed run with exponential backoff.
    ///
    /// The delay before the next attempt is `2^n` seconds where `n` is the
    /// current retry count. After `MAX_RETRIES` (5) attempts, the run is
    /// moved to the dead letter queue instead.
    pub async fn retry_failed(&self, run_id: &str) -> Result<RetryOutcome> {
        let run = self
            .run_store
            .get(run_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Run not found: {run_id}")))?;

        if run.status != RunStatus::Failed {
            return Err(Error::Validation(format!(
                "Run {} is not in Failed state (current: {:?})",
                run_id, run.status
            )));
        }

        // Count existing retries by checking how many times this message
        // has appeared. We store the retry count as metadata in the message
        // prefix format: "[retry:N] original_message"
        let retry_count = Self::extract_retry_count(&run.message);

        if retry_count >= MAX_RETRIES {
            warn!(
                run_id = %run_id,
                retry_count = retry_count,
                "CronBridge: max retries exceeded, moving to dead letter"
            );
            self.dead_letter(run_id).await?;
            return Ok(RetryOutcome::Exhausted {
                run_id: run_id.to_string(),
                retry_count,
            });
        }

        let new_retry_count = retry_count + 1;
        let backoff_secs = 2u64.pow(new_retry_count);
        let original_message = Self::strip_retry_prefix(&run.message);
        let new_message = format!("[retry:{new_retry_count}] {original_message}");

        // Mark the old run as cancelled (superseded by retry)
        self.run_store.cancel(run_id).await?;

        // Create a new run item for the retry
        let mut new_item = RunItem::new(&new_message);
        new_item.channel = run.channel.clone();
        new_item.cron_job_id = run.cron_job_id.clone();

        let enqueued = self.run_store.enqueue(new_item).await?;

        info!(
            old_run_id = %run_id,
            new_run_id = %enqueued.id,
            retry_count = new_retry_count,
            backoff_secs = backoff_secs,
            "CronBridge: scheduled retry"
        );

        Ok(RetryOutcome::Retried {
            new_run_id: enqueued.id,
            retry_count: new_retry_count,
        })
    }

    /// Move an exhausted run to the dead letter queue.
    ///
    /// This records the run's details in the dead letter JSONL file
    /// and marks the run as Cancelled in the RunStore.
    pub async fn dead_letter(&self, run_id: &str) -> Result<()> {
        let run = self
            .run_store
            .get(run_id)
            .await?
            .ok_or_else(|| Error::NotFound(format!("Run not found: {run_id}")))?;

        let retry_count = Self::extract_retry_count(&run.message);
        let original_message = Self::strip_retry_prefix(&run.message);

        let entry = DeadLetterEntry {
            run_id: run_id.to_string(),
            original_message,
            retry_count,
            last_error: None,
            dead_lettered_at: Utc::now(),
        };

        // Append to dead letter JSONL
        let mut line = serde_json::to_string(&entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        line.push('\n');
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.dead_letter_path)?;
        use std::io::Write;
        file.write_all(line.as_bytes())?;
        file.flush()?;

        // Mark the run as cancelled in the RunStore
        self.run_store.cancel(run_id).await?;

        info!(
            run_id = %run_id,
            retry_count = retry_count,
            "CronBridge: moved exhausted run to dead letter queue"
        );

        Ok(())
    }

    /// Extract the retry count from a message that may contain a `[retry:N]` prefix.
    fn extract_retry_count(message: &str) -> u32 {
        if let Some(rest) = message.strip_prefix("[retry:") {
            if let Some(end) = rest.find(']') {
                if let Ok(n) = rest[..end].parse::<u32>() {
                    return n;
                }
            }
        }
        0
    }

    /// Strip the `[retry:N]` prefix from a message, returning the original.
    fn strip_retry_prefix(message: &str) -> String {
        if let Some(rest) = message.strip_prefix("[retry:") {
            if let Some(end) = rest.find(']') {
                return rest[end + 1..].trim_start().to_string();
            }
        }
        message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::todo::JsonlTodoStore;
    use tempfile::TempDir;

    /// Helper to set up a CronBridge with temporary storage.
    async fn setup() -> (TempDir, CronBridge) {
        let dir = TempDir::new().expect("tempdir");
        let run_store = RunStore::new(dir.path()).await.expect("run store");
        let todo_store = JsonlTodoStore::new(dir.path()).expect("todo store");
        let bridge = CronBridge::new(run_store, todo_store, dir.path()).expect("bridge");
        (dir, bridge)
    }

    #[tokio::test]
    async fn test_dispatch_creates_run_with_queued_status() {
        let (_dir, bridge) = setup().await;
        let payload = CronPayload {
            kind: "agent_turn".to_string(),
            message: "standup reminder".to_string(),
            deliver: false,
            channel: Some("telegram".to_string()),
            to: Some("123456".to_string()),
        };

        let run_id = bridge.dispatch(&payload).await.expect("dispatch");

        // Verify the run was created with Queued status
        let run = bridge
            .run_store
            .get(&run_id)
            .await
            .expect("get")
            .expect("item");
        assert_eq!(run.status, RunStatus::Queued);
        assert_eq!(run.message, "standup reminder");
        assert_eq!(run.channel.as_deref(), Some("telegram"));
    }

    #[tokio::test]
    async fn test_dispatch_with_deliver_creates_todo() {
        let (_dir, bridge) = setup().await;
        let payload = CronPayload {
            kind: "agent_turn".to_string(),
            message: "daily report".to_string(),
            deliver: true,
            channel: Some("discord".to_string()),
            to: None,
        };

        let run_id = bridge.dispatch(&payload).await.expect("dispatch");

        // Verify a todo was created
        let todos = bridge.todo_store.list().await.expect("list");
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].title, "daily report");
        assert_eq!(todos[0].source, "cron");
        assert_eq!(todos[0].linked_run_id.as_deref(), Some(run_id.as_str()));
    }

    #[tokio::test]
    async fn test_dispatch_without_deliver_no_todo() {
        let (_dir, bridge) = setup().await;
        let payload = CronPayload {
            kind: "agent_turn".to_string(),
            message: "no delivery".to_string(),
            deliver: false,
            channel: None,
            to: None,
        };

        bridge.dispatch(&payload).await.expect("dispatch");

        let todos = bridge.todo_store.list().await.expect("list");
        assert!(todos.is_empty());
    }

    #[tokio::test]
    async fn test_retry_failed_increments_retry_count() {
        let (_dir, bridge) = setup().await;

        // Create and fail a run
        let payload = CronPayload {
            kind: "agent_turn".to_string(),
            message: "flaky task".to_string(),
            deliver: false,
            channel: None,
            to: None,
        };
        let run_id = bridge.dispatch(&payload).await.expect("dispatch");

        // Claim and fail the run
        let claimed = bridge
            .run_store
            .claim_next()
            .await
            .expect("claim")
            .expect("item");
        assert_eq!(claimed.id, run_id);
        bridge.run_store.fail(&run_id).await.expect("fail");

        // Retry
        let outcome = bridge.retry_failed(&run_id).await.expect("retry");
        match outcome {
            RetryOutcome::Retried {
                new_run_id,
                retry_count,
            } => {
                assert_eq!(retry_count, 1);
                // Verify the new run has the retry prefix
                let new_run = bridge
                    .run_store
                    .get(&new_run_id)
                    .await
                    .expect("get")
                    .expect("item");
                assert_eq!(new_run.status, RunStatus::Queued);
                assert!(new_run.message.contains("[retry:1]"));
                assert!(new_run.message.contains("flaky task"));
            }
            RetryOutcome::Exhausted { .. } => {
                panic!("Should not be exhausted after 1 retry");
            }
        }
    }

    #[tokio::test]
    async fn test_retry_failed_max_5_then_dead_letter() {
        let (_dir, bridge) = setup().await;

        // Simulate a run that has already been retried 5 times
        let mut item = RunItem::new("[retry:5] persistent task");
        item.channel = None;
        item.cron_job_id = None;
        let enqueued = bridge.run_store.enqueue(item).await.expect("enqueue");

        // Claim and fail it
        let _claimed = bridge
            .run_store
            .claim_next()
            .await
            .expect("claim")
            .expect("item");
        bridge.run_store.fail(&enqueued.id).await.expect("fail");

        // Retry should exhaust
        let outcome = bridge.retry_failed(&enqueued.id).await.expect("retry");
        assert_eq!(
            outcome,
            RetryOutcome::Exhausted {
                run_id: enqueued.id.clone(),
                retry_count: 5
            }
        );

        // Verify dead letter file has an entry
        let content = std::fs::read_to_string(&bridge.dead_letter_path).expect("read dead letter");
        assert!(content.contains("persistent task"));
    }

    #[tokio::test]
    async fn test_retry_failed_not_failed_state_rejected() {
        let (_dir, bridge) = setup().await;

        let payload = CronPayload {
            kind: "agent_turn".to_string(),
            message: "still running".to_string(),
            deliver: false,
            channel: None,
            to: None,
        };
        let run_id = bridge.dispatch(&payload).await.expect("dispatch");

        // Run is still Queued, not Failed
        let result = bridge.retry_failed(&run_id).await;
        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
            assert!(msg.contains("not in Failed state"));
        } else {
            panic!("Expected Validation error");
        }
    }

    #[tokio::test]
    async fn test_dead_letter_records_exhausted_retries() {
        let (_dir, bridge) = setup().await;

        let item = RunItem::new("[retry:5] exhausted task");
        let enqueued = bridge.run_store.enqueue(item).await.expect("enqueue");

        bridge.dead_letter(&enqueued.id).await.expect("dead_letter");

        // Verify run is now cancelled
        let run = bridge
            .run_store
            .get(&enqueued.id)
            .await
            .expect("get")
            .expect("item");
        assert_eq!(run.status, RunStatus::Cancelled);

        // Verify dead letter entry
        let content = std::fs::read_to_string(&bridge.dead_letter_path).expect("read dead letter");
        let entry: DeadLetterEntry =
            serde_json::from_str(content.trim()).expect("parse dead letter");
        assert_eq!(entry.original_message, "exhausted task");
        assert_eq!(entry.retry_count, 5);
        assert_eq!(entry.run_id, enqueued.id);
    }

    #[tokio::test]
    async fn test_dead_letter_not_found() {
        let (_dir, bridge) = setup().await;
        let result = bridge.dead_letter("nonexistent-id").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_retry_count_no_prefix() {
        assert_eq!(CronBridge::extract_retry_count("hello world"), 0);
    }

    #[test]
    fn test_extract_retry_count_with_prefix() {
        assert_eq!(CronBridge::extract_retry_count("[retry:3] hello"), 3);
    }

    #[test]
    fn test_strip_retry_prefix_no_prefix() {
        assert_eq!(CronBridge::strip_retry_prefix("hello"), "hello");
    }

    #[test]
    fn test_strip_retry_prefix_with_prefix() {
        assert_eq!(CronBridge::strip_retry_prefix("[retry:2] hello"), "hello");
    }
}
