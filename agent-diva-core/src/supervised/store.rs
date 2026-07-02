//! SQLite-backed store for supervised run items (sqlx implementation)

use super::types::{RunItem, RunStatus};
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;

/// Persistent SQLite store for supervised runs
pub struct RunStore {
    pool: SqlitePool,
}

impl RunStore {
    /// Open (or create) the runs database at `{data_root}/runs.db`.
    ///
    /// Enables WAL mode for concurrent read/write safety and creates
    /// the `runs` table if it does not exist.
    pub async fn new(data_root: &Path) -> Result<Self, sqlx::Error> {
        std::fs::create_dir_all(data_root).map_err(|e| sqlx::Error::Configuration(e.into()))?;
        let db_path = data_root.join("runs.db");
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new().connect_with(options).await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS runs (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                message TEXT NOT NULL,
                channel TEXT,
                cron_job_id TEXT,
                heartbeat_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    /// Insert a new run item with status Queued.
    pub async fn enqueue(&self, item: RunItem) -> Result<RunItem, sqlx::Error> {
        let status_str = item.status.as_str();
        let heartbeat_str = item.heartbeat_at.map(|dt| dt.to_rfc3339());
        let id = item.id.clone();
        sqlx::query(
            "INSERT INTO runs (id, status, message, channel, cron_job_id, heartbeat_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        )
        .bind(id)
        .bind(status_str)
        .bind(&item.message)
        .bind(&item.channel)
        .bind(&item.cron_job_id)
        .bind(heartbeat_str)
        .bind(item.created_at.to_rfc3339())
        .bind(item.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(item)
    }

    /// Claim the next Queued item for execution.
    ///
    /// Uses an immediate transaction to ensure only one consumer can claim a
    /// given task. Sets status to Running and heartbeat_at to now.
    pub async fn claim_next(&self) -> Result<Option<RunItem>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        // Acquire exclusive lock immediately (equivalent to BEGIN IMMEDIATE)
        sqlx::query("UPDATE runs SET id = id WHERE 0 = 1")
            .execute(&mut *tx)
            .await?;

        let row = sqlx::query(
            "SELECT id, status, message, channel, cron_job_id, heartbeat_at, created_at, updated_at
             FROM runs WHERE status = 'queued' ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(row) = row {
            let id: String = row.get(0);
            let now = Utc::now();
            sqlx::query(
                "UPDATE runs SET status = 'running', heartbeat_at = ?1, updated_at = ?2 WHERE id = ?3",
            )
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .bind(&id)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;

            let item = self.row_to_item(&row, RunStatus::Running, Some(now), now)?;
            Ok(Some(item))
        } else {
            tx.commit().await?;
            Ok(None)
        }
    }

    /// Update the heartbeat timestamp for a running task.
    pub async fn heartbeat(&self, id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let result = sqlx::query(
            "UPDATE runs SET heartbeat_at = ?1, updated_at = ?2 WHERE id = ?3 AND status = 'running'",
        )
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    /// Mark a run as Completed.
    pub async fn complete(&self, id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let result =
            sqlx::query("UPDATE runs SET status = 'completed', updated_at = ?1 WHERE id = ?2")
                .bind(now.to_rfc3339())
                .bind(id)
                .execute(&self.pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    /// Mark a run as Failed.
    pub async fn fail(&self, id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let result =
            sqlx::query("UPDATE runs SET status = 'failed', updated_at = ?1 WHERE id = ?2")
                .bind(now.to_rfc3339())
                .bind(id)
                .execute(&self.pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    /// Mark a run as Cancelled.
    pub async fn cancel(&self, id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now();
        let result =
            sqlx::query("UPDATE runs SET status = 'cancelled', updated_at = ?1 WHERE id = ?2")
                .bind(now.to_rfc3339())
                .bind(id)
                .execute(&self.pool)
                .await?;

        if result.rows_affected() == 0 {
            return Err(sqlx::Error::RowNotFound);
        }
        Ok(())
    }

    /// Reap stale running items whose heartbeat is older than `timeout_secs`.
    ///
    /// Marks them as Lost and returns the list of reaped IDs.
    pub async fn reap_stale(&self, timeout_secs: u64) -> Result<Vec<String>, sqlx::Error> {
        let cutoff = Utc::now() - chrono::Duration::seconds(timeout_secs as i64);
        let cutoff_str = cutoff.to_rfc3339();
        let now = Utc::now();

        let rows = sqlx::query(
            "SELECT id FROM runs WHERE status = 'running' AND heartbeat_at IS NOT NULL AND heartbeat_at < ?1",
        )
        .bind(&cutoff_str)
        .fetch_all(&self.pool)
        .await?;

        let ids: Vec<String> = rows.iter().map(|r| r.get::<String, _>(0)).collect();

        for id in &ids {
            sqlx::query("UPDATE runs SET status = 'lost', updated_at = ?1 WHERE id = ?2")
                .bind(now.to_rfc3339())
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        Ok(ids)
    }

    /// Get a run item by ID.
    pub async fn get(&self, id: &str) -> Result<Option<RunItem>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, status, message, channel, cron_job_id, heartbeat_at, created_at, updated_at
             FROM runs WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(self.row_to_item_from_db(&r)?)),
            None => Ok(None),
        }
    }

    /// List all run items with the given status.
    pub async fn list_by_status(&self, status: RunStatus) -> Result<Vec<RunItem>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, status, message, channel, cron_job_id, heartbeat_at, created_at, updated_at
             FROM runs WHERE status = ?1 ORDER BY created_at ASC",
        )
        .bind(status.as_str())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(|r| self.row_to_item_from_db(r)).collect()
    }

    // -- private helpers --

    /// Build a RunItem from a database row, parsing all fields from strings.
    fn row_to_item_from_db(&self, row: &sqlx::sqlite::SqliteRow) -> Result<RunItem, sqlx::Error> {
        let status_str: String = row.get(1);
        let status =
            RunStatus::from_str_lossy(&status_str).ok_or_else(|| sqlx::Error::ColumnDecode {
                index: "1".into(),
                source: format!("invalid status: {status_str}").into(),
            })?;

        let heartbeat_str: Option<String> = row.get(5);
        let heartbeat_at = heartbeat_str
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "5".into(),
                source: e.to_string().into(),
            })?
            .map(|dt| dt.with_timezone(&Utc));

        let created_str: String = row.get(6);
        let created_at = DateTime::parse_from_rfc3339(&created_str)
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "6".into(),
                source: e.to_string().into(),
            })?
            .with_timezone(&Utc);

        let updated_str: String = row.get(7);
        let updated_at = DateTime::parse_from_rfc3339(&updated_str)
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "7".into(),
                source: e.to_string().into(),
            })?
            .with_timezone(&Utc);

        Ok(RunItem {
            id: row.get(0),
            status,
            message: row.get(2),
            channel: row.get(3),
            cron_job_id: row.get(4),
            heartbeat_at,
            created_at,
            updated_at,
        })
    }

    /// Build a RunItem from a row with known overrides (used by claim_next).
    fn row_to_item(
        &self,
        row: &sqlx::sqlite::SqliteRow,
        status: RunStatus,
        heartbeat_at: Option<DateTime<Utc>>,
        updated_at: DateTime<Utc>,
    ) -> Result<RunItem, sqlx::Error> {
        let created_str: String = row.get(6);
        let created_at = DateTime::parse_from_rfc3339(&created_str)
            .map_err(|e| sqlx::Error::ColumnDecode {
                index: "6".into(),
                source: e.to_string().into(),
            })?
            .with_timezone(&Utc);

        Ok(RunItem {
            id: row.get(0),
            status,
            message: row.get(2),
            channel: row.get(3),
            cron_job_id: row.get(4),
            heartbeat_at,
            created_at,
            updated_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper to create a store backed by a temp directory
    async fn setup() -> (TempDir, RunStore) {
        let dir = TempDir::new().expect("tempdir");
        let store = RunStore::new(dir.path()).await.expect("store creation");
        (dir, store)
    }

    /// Helper to create a sample RunItem
    fn sample_item(message: &str) -> RunItem {
        RunItem::new(message)
    }

    #[tokio::test]
    async fn test_enqueue_and_claim() {
        let (_dir, store) = setup().await;
        let item = sample_item("test task");
        let enqueued = store.enqueue(item).await.expect("enqueue");
        assert_eq!(enqueued.status, RunStatus::Queued);

        let claimed = store.claim_next().await.expect("claim_next");
        assert!(claimed.is_some());
        let claimed = claimed.expect("item");
        assert_eq!(claimed.id, enqueued.id);
        assert_eq!(claimed.status, RunStatus::Running);
        assert!(claimed.heartbeat_at.is_some());
    }

    #[tokio::test]
    async fn test_claim_next_empty() {
        let (_dir, store) = setup().await;
        let result = store.claim_next().await.expect("claim_next");
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_heartbeat() {
        let (_dir, store) = setup().await;
        let item = sample_item("heartbeat test");
        store.enqueue(item).await.expect("enqueue");
        let claimed = store.claim_next().await.expect("claim_next").expect("item");
        let original_hb = claimed.heartbeat_at.expect("heartbeat");

        // Small sleep to ensure time difference
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        store.heartbeat(&claimed.id).await.expect("heartbeat");

        let reloaded = store.get(&claimed.id).await.expect("get").expect("item");
        assert!(reloaded.heartbeat_at.expect("heartbeat") > original_hb);
    }

    #[tokio::test]
    async fn test_complete() {
        let (_dir, store) = setup().await;
        let item = sample_item("complete test");
        store.enqueue(item).await.expect("enqueue");
        let claimed = store.claim_next().await.expect("claim_next").expect("item");

        store.complete(&claimed.id).await.expect("complete");

        let reloaded = store.get(&claimed.id).await.expect("get").expect("item");
        assert_eq!(reloaded.status, RunStatus::Completed);
    }

    #[tokio::test]
    async fn test_fail() {
        let (_dir, store) = setup().await;
        let item = sample_item("fail test");
        store.enqueue(item).await.expect("enqueue");
        let claimed = store.claim_next().await.expect("claim_next").expect("item");

        store.fail(&claimed.id).await.expect("fail");

        let reloaded = store.get(&claimed.id).await.expect("get").expect("item");
        assert_eq!(reloaded.status, RunStatus::Failed);
    }

    #[tokio::test]
    async fn test_cancel() {
        let (_dir, store) = setup().await;
        let item = sample_item("cancel test");
        let enqueued = store.enqueue(item).await.expect("enqueue");

        store.cancel(&enqueued.id).await.expect("cancel");

        let reloaded = store.get(&enqueued.id).await.expect("get").expect("item");
        assert_eq!(reloaded.status, RunStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_reap_stale() {
        let (_dir, store) = setup().await;
        let item = sample_item("stale test");
        store.enqueue(item).await.expect("enqueue");
        let claimed = store.claim_next().await.expect("claim_next").expect("item");

        // Manually set heartbeat_at to 120s ago
        let stale_time = Utc::now() - chrono::Duration::seconds(120);
        sqlx::query("UPDATE runs SET heartbeat_at = ?1 WHERE id = ?2")
            .bind(stale_time.to_rfc3339())
            .bind(&claimed.id)
            .execute(&store.pool)
            .await
            .expect("update heartbeat");

        let reaped = store.reap_stale(60).await.expect("reap_stale");
        assert_eq!(reaped.len(), 1);
        assert_eq!(reaped[0], claimed.id);

        let reloaded = store.get(&claimed.id).await.expect("get").expect("item");
        assert_eq!(reloaded.status, RunStatus::Lost);
    }

    #[tokio::test]
    async fn test_reap_stale_no_reap() {
        let (_dir, store) = setup().await;
        let item = sample_item("fresh test");
        store.enqueue(item).await.expect("enqueue");
        store.claim_next().await.expect("claim_next").expect("item");

        // Item has a recent heartbeat, should not be reaped with 60s timeout
        let reaped = store.reap_stale(60).await.expect("reap_stale");
        assert!(reaped.is_empty());
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let (_dir, store) = setup().await;
        store.enqueue(sample_item("task 1")).await.expect("enqueue");
        store.enqueue(sample_item("task 2")).await.expect("enqueue");
        store.enqueue(sample_item("task 3")).await.expect("enqueue");

        // Claim one — it moves from Queued to Running
        let claimed = store.claim_next().await.expect("claim_next").expect("item");

        let queued = store.list_by_status(RunStatus::Queued).await.expect("list");
        assert_eq!(queued.len(), 2);

        let running = store
            .list_by_status(RunStatus::Running)
            .await
            .expect("list");
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].id, claimed.id);
    }

    #[tokio::test]
    async fn test_get() {
        let (_dir, store) = setup().await;
        let item = sample_item("get test");
        let enqueued = store.enqueue(item).await.expect("enqueue");

        let found = store.get(&enqueued.id).await.expect("get");
        assert!(found.is_some());
        let found = found.expect("item");
        assert_eq!(found.id, enqueued.id);
        assert_eq!(found.message, "get test");
        assert_eq!(found.status, RunStatus::Queued);
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let (_dir, store) = setup().await;
        let found = store.get("nonexistent-id").await.expect("get");
        assert!(found.is_none());
    }
}
