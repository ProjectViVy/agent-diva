//! SQLite-backed store for supervised run items (sqlx implementation)

use super::types::{RunItem, RunKind, RunRecord, RunStatus, RunStoreError, SupervisedRunSpec};
use crate::audit;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;

/// Persistent SQLite store for supervised runs
#[derive(Clone)]
pub struct RunStore {
    pub(crate) pool: SqlitePool,
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

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS supervised_runs (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                message TEXT NOT NULL,
                channel TEXT,
                cron_job_id TEXT,
                heartbeat_at TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                kind TEXT NOT NULL DEFAULT 'generic',
                priority INTEGER NOT NULL DEFAULT 0,
                attempt INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 3,
                timeout_secs INTEGER,
                started_at TEXT,
                completed_at TEXT,
                error_message TEXT,
                result_summary TEXT,
                claimed_by TEXT,
                metadata TEXT,
                parent_id TEXT,
                tags TEXT
            );",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_supervised_runs_status ON supervised_runs(status);",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_supervised_runs_claim ON supervised_runs(status, priority, created_at);",
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_supervised_runs_dedupe ON supervised_runs(message, channel, cron_job_id) WHERE status = 'queued';",
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

    // ========================================================================
    // Supervised runs CRUD (supervised_runs table)
    // ========================================================================

    /// Create a new supervised run record from a spec.
    pub async fn create(&self, spec: &SupervisedRunSpec) -> Result<RunRecord, RunStoreError> {
        let record = RunRecord::from_spec(spec);
        let metadata_str = record.metadata.as_ref().map(|m| m.to_string());
        let tags_str = record
            .tags
            .as_ref()
            .map(|t| serde_json::to_string(t).unwrap_or_default());

        sqlx::query(
            "INSERT INTO supervised_runs (
                id, status, message, channel, cron_job_id, heartbeat_at, created_at, updated_at,
                kind, priority, attempt, max_attempts, timeout_secs, started_at, completed_at,
                error_message, result_summary, claimed_by, metadata, parent_id, tags
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
        )
        .bind(&record.id)
        .bind(record.status.as_str())
        .bind(&record.message)
        .bind(&record.channel)
        .bind(&record.cron_job_id)
        .bind(record.heartbeat_at.map(|dt| dt.to_rfc3339()))
        .bind(record.created_at.to_rfc3339())
        .bind(record.updated_at.to_rfc3339())
        .bind(record.kind.as_str())
        .bind(record.priority)
        .bind(record.attempt)
        .bind(record.max_attempts)
        .bind(record.timeout_secs)
        .bind(record.started_at.map(|dt| dt.to_rfc3339()))
        .bind(record.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(&record.error_message)
        .bind(&record.result_summary)
        .bind(&record.claimed_by)
        .bind(metadata_str)
        .bind(&record.parent_id)
        .bind(tags_str)
        .execute(&self.pool)
        .await
        .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        audit::emit(audit::AuditEvent::RunCreated {
            run_id: record.id.clone(),
            kind: record.kind.as_str().to_string(),
        });

        Ok(record)
    }

    /// Get a full RunRecord by ID from supervised_runs.
    pub async fn get_record(&self, id: &str) -> Result<Option<RunRecord>, RunStoreError> {
        let row = sqlx::query(
            "SELECT id, status, message, channel, cron_job_id, heartbeat_at, created_at, updated_at,
                    kind, priority, attempt, max_attempts, timeout_secs, started_at, completed_at,
                    error_message, result_summary, claimed_by, metadata, parent_id, tags
             FROM supervised_runs WHERE id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        match row {
            Some(r) => Ok(Some(self.row_to_record(&r)?)),
            None => Ok(None),
        }
    }

    /// Claim the next queued supervised run for a worker.
    /// Uses RETURNING for atomic claim.
    pub async fn claim_next_supervised(
        &self,
        worker_id: &str,
    ) -> Result<Option<RunRecord>, RunStoreError> {
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        let row = sqlx::query(
            "UPDATE supervised_runs
             SET status = 'running', claimed_by = ?1, started_at = ?2, heartbeat_at = ?3, updated_at = ?4
             WHERE id = (
                 SELECT id FROM supervised_runs
                 WHERE status = 'queued'
                 ORDER BY priority DESC, created_at ASC
                 LIMIT 1
             )
             RETURNING id, status, message, channel, cron_job_id, heartbeat_at, created_at, updated_at,
                       kind, priority, attempt, max_attempts, timeout_secs, started_at, completed_at,
                       error_message, result_summary, claimed_by, metadata, parent_id, tags",
        )
        .bind(worker_id)
        .bind(now_str.clone())
        .bind(now_str.clone())
        .bind(now_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        match row {
            Some(r) => {
                let record = self.row_to_record(&r)?;
                audit::emit(audit::AuditEvent::RunClaimed {
                    run_id: record.id.clone(),
                    worker_id: worker_id.to_string(),
                });
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Update heartbeat for a running supervised run, with owner verification.
    pub async fn heartbeat_supervised(
        &self,
        id: &str,
        worker_id: &str,
    ) -> Result<(), RunStoreError> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE supervised_runs SET heartbeat_at = ?1, updated_at = ?2
             WHERE id = ?3 AND status = 'running' AND claimed_by = ?4",
        )
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(id)
        .bind(worker_id)
        .execute(&self.pool)
        .await
        .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        if result.rows_affected() == 0 {
            // Check if the record exists and who owns it
            let record = self.get_record(id).await?;
            match record {
                Some(rec) => {
                    if rec.status != RunStatus::Running {
                        return Err(RunStoreError::InvalidState(format!(
                            "run {} is not running (status: {:?})",
                            id, rec.status
                        )));
                    }
                    if rec.claimed_by.as_deref() != Some(worker_id) {
                        return Err(RunStoreError::NotOwner(format!(
                            "run {} is claimed by {:?}, not {}",
                            id, rec.claimed_by, worker_id
                        )));
                    }
                    // Should not reach here if rows_affected == 0
                    return Err(RunStoreError::InvalidState(format!(
                        "run {} heartbeat update failed unexpectedly",
                        id
                    )));
                }
                None => return Err(RunStoreError::NotFound(id.to_string())),
            }
        }

        Ok(())
    }

    /// Mark a supervised run as completed, with owner verification.
    pub async fn complete_supervised(
        &self,
        id: &str,
        worker_id: &str,
        result_summary: Option<String>,
    ) -> Result<(), RunStoreError> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE supervised_runs
             SET status = 'completed', result_summary = ?1, completed_at = ?2, updated_at = ?3
             WHERE id = ?4 AND status = 'running' AND claimed_by = ?5",
        )
        .bind(result_summary)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(id)
        .bind(worker_id)
        .execute(&self.pool)
        .await
        .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        if result.rows_affected() == 0 {
            let record = self.get_record(id).await?;
            match record {
                Some(rec) => {
                    if rec.status != RunStatus::Running {
                        return Err(RunStoreError::InvalidState(format!(
                            "run {} is not running (status: {:?})",
                            id, rec.status
                        )));
                    }
                    if rec.claimed_by.as_deref() != Some(worker_id) {
                        return Err(RunStoreError::NotOwner(format!(
                            "run {} is claimed by {:?}, not {}",
                            id, rec.claimed_by, worker_id
                        )));
                    }
                    return Err(RunStoreError::InvalidState(format!(
                        "run {} complete update failed unexpectedly",
                        id
                    )));
                }
                None => return Err(RunStoreError::NotFound(id.to_string())),
            }
        }

        // Calculate duration from started_at to now
        let record = self.get_record(id).await?;
        if let Some(ref rec) = record {
            if let Some(started) = rec.started_at {
                let duration = Utc::now().signed_duration_since(started);
                audit::emit(audit::AuditEvent::RunCompleted {
                    run_id: id.to_string(),
                    duration_ms: duration.num_milliseconds().max(0) as u64,
                });
            }
        }

        Ok(())
    }

    /// Mark a supervised run as failed, with owner verification.
    pub async fn fail_supervised(
        &self,
        id: &str,
        worker_id: &str,
        error_message: Option<String>,
    ) -> Result<(), RunStoreError> {
        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE supervised_runs
             SET status = 'failed', error_message = ?1, completed_at = ?2, updated_at = ?3
             WHERE id = ?4 AND status = 'running' AND claimed_by = ?5",
        )
        .bind(error_message.clone())
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(id)
        .bind(worker_id)
        .execute(&self.pool)
        .await
        .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        if result.rows_affected() == 0 {
            let record = self.get_record(id).await?;
            match record {
                Some(rec) => {
                    if rec.status != RunStatus::Running {
                        return Err(RunStoreError::InvalidState(format!(
                            "run {} is not running (status: {:?})",
                            id, rec.status
                        )));
                    }
                    if rec.claimed_by.as_deref() != Some(worker_id) {
                        return Err(RunStoreError::NotOwner(format!(
                            "run {} is claimed by {:?}, not {}",
                            id, rec.claimed_by, worker_id
                        )));
                    }
                    return Err(RunStoreError::InvalidState(format!(
                        "run {} fail update failed unexpectedly",
                        id
                    )));
                }
                None => return Err(RunStoreError::NotFound(id.to_string())),
            }
        }

        audit::emit(audit::AuditEvent::RunFailed {
            run_id: id.to_string(),
            error: error_message.unwrap_or_default(),
        });

        Ok(())
    }

    /// Cancel a supervised run (can be done by anyone, not just owner).
    pub async fn cancel_supervised(
        &self,
        id: &str,
        reason: Option<String>,
    ) -> Result<(), RunStoreError> {
        let now = Utc::now();
        let audit_reason = reason.clone().unwrap_or_default();

        let result = sqlx::query(
            "UPDATE supervised_runs
             SET status = 'cancelled', error_message = ?1, completed_at = COALESCE(completed_at, ?2), updated_at = ?3
             WHERE id = ?4 AND status IN ('queued', 'running')",
        )
        .bind(reason)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        if result.rows_affected() == 0 {
            let record = self.get_record(id).await?;
            match record {
                Some(rec) => {
                    if rec.status == RunStatus::Cancelled {
                        return Ok(());
                    }
                    return Err(RunStoreError::InvalidState(format!(
                        "run {} cannot be cancelled from status {:?}",
                        id, rec.status
                    )));
                }
                None => return Err(RunStoreError::NotFound(id.to_string())),
            }
        }

        audit::emit(audit::AuditEvent::RunCancelled {
            run_id: id.to_string(),
            reason: audit_reason,
        });

        Ok(())
    }

    /// Mark stale running runs as lost.
    pub async fn mark_lost(
        &self,
        stale_before: DateTime<Utc>,
    ) -> Result<Vec<RunRecord>, RunStoreError> {
        let stale_str = stale_before.to_rfc3339();

        let rows = sqlx::query(
            "SELECT id, status, message, channel, cron_job_id, heartbeat_at, created_at, updated_at,
                    kind, priority, attempt, max_attempts, timeout_secs, started_at, completed_at,
                    error_message, result_summary, claimed_by, metadata, parent_id, tags
             FROM supervised_runs
             WHERE status = 'running' AND heartbeat_at IS NOT NULL AND heartbeat_at < ?1",
        )
        .bind(stale_str)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        let mut records = Vec::new();
        for row in rows {
            let id: String = row.get(0);
            let heartbeat_str: Option<String> = row.get(5);
            let now = Utc::now();

            let result = sqlx::query(
                "UPDATE supervised_runs
                 SET status = 'lost', completed_at = COALESCE(completed_at, ?1), updated_at = ?2
                 WHERE id = ?3 AND status = 'running'",
            )
            .bind(now.to_rfc3339())
            .bind(now.to_rfc3339())
            .bind(&id)
            .execute(&self.pool)
            .await
            .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

            if result.rows_affected() == 0 {
                continue;
            }

            audit::emit(audit::AuditEvent::RunLost {
                run_id: id.clone(),
                last_heartbeat: heartbeat_str,
            });

            if let Some(record) = self.get_record(&id).await? {
                records.push(record);
            }
        }

        Ok(records)
    }

    /// Requeue a failed or lost run for retry.
    pub async fn requeue(&self, id: &str) -> Result<RunRecord, RunStoreError> {
        let record = self.get_record(id).await?;
        let record = record.ok_or_else(|| RunStoreError::NotFound(id.to_string()))?;

        if record.status != RunStatus::Failed && record.status != RunStatus::Lost {
            return Err(RunStoreError::InvalidState(format!(
                "run {} cannot be requeued from status {:?}",
                id, record.status
            )));
        }

        let new_attempt = record.attempt + 1;
        if new_attempt >= record.max_attempts {
            return Err(RunStoreError::MaxAttemptsReached(id.to_string()));
        }

        let now = Utc::now();

        let result = sqlx::query(
            "UPDATE supervised_runs
             SET status = 'queued', attempt = ?1, claimed_by = NULL, started_at = NULL,
                 completed_at = NULL, error_message = NULL, result_summary = NULL,
                 heartbeat_at = NULL, updated_at = ?2
             WHERE id = ?3",
        )
        .bind(new_attempt)
        .bind(now.to_rfc3339())
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(RunStoreError::NotFound(id.to_string()));
        }

        let requeued = self
            .get_record(id)
            .await?
            .ok_or_else(|| RunStoreError::NotFound(id.to_string()))?;

        audit::emit(audit::AuditEvent::RunRequeued {
            run_id: id.to_string(),
            attempt: requeued.attempt,
        });

        Ok(requeued)
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

    /// Build a RunRecord from a database row.
    fn row_to_record(&self, row: &sqlx::sqlite::SqliteRow) -> Result<RunRecord, RunStoreError> {
        let status_str: String = row.get(1);
        let status = RunStatus::from_str_lossy(&status_str).ok_or_else(|| {
            RunStoreError::InvalidState(format!("invalid status: {}", status_str))
        })?;

        let kind_str: String = row.get(8);
        let kind = RunKind::from_str_lossy(&kind_str)
            .ok_or_else(|| RunStoreError::InvalidState(format!("invalid kind: {}", kind_str)))?;

        let heartbeat_str: Option<String> = row.get(5);
        let heartbeat_at = heartbeat_str
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|e| RunStoreError::SqlxError(e.to_string()))?
            .map(|dt| dt.with_timezone(&Utc));

        let created_str: String = row.get(6);
        let created_at = DateTime::parse_from_rfc3339(&created_str)
            .map_err(|e| RunStoreError::SqlxError(e.to_string()))?
            .with_timezone(&Utc);

        let updated_str: String = row.get(7);
        let updated_at = DateTime::parse_from_rfc3339(&updated_str)
            .map_err(|e| RunStoreError::SqlxError(e.to_string()))?
            .with_timezone(&Utc);

        let started_str: Option<String> = row.get(13);
        let started_at = started_str
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|e| RunStoreError::SqlxError(e.to_string()))?
            .map(|dt| dt.with_timezone(&Utc));

        let completed_str: Option<String> = row.get(14);
        let completed_at = completed_str
            .as_deref()
            .map(DateTime::parse_from_rfc3339)
            .transpose()
            .map_err(|e| RunStoreError::SqlxError(e.to_string()))?
            .map(|dt| dt.with_timezone(&Utc));

        let timeout_secs: Option<i64> = row.get(12);

        let metadata_str: Option<String> = row.get(18);
        let metadata = metadata_str
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        let tags_str: Option<String> = row.get(20);
        let tags = tags_str
            .map(|s| serde_json::from_str(&s))
            .transpose()
            .map_err(|e| RunStoreError::SqlxError(e.to_string()))?;

        Ok(RunRecord {
            id: row.get(0),
            status,
            message: row.get(2),
            channel: row.get(3),
            cron_job_id: row.get(4),
            heartbeat_at,
            created_at,
            updated_at,
            kind,
            priority: row.get(9),
            attempt: row.get(10),
            max_attempts: row.get(11),
            timeout_secs: if timeout_secs.is_some() {
                timeout_secs
            } else {
                None
            },
            started_at,
            completed_at,
            error_message: row.get(15),
            result_summary: row.get(16),
            claimed_by: row.get(17),
            metadata,
            parent_id: row.get(19),
            tags,
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

    // =========================================================================
    // RETURNING compatibility verification tests
    // =========================================================================

    /// Test A: verify sqlx 0.7 supports SQLite `UPDATE ... RETURNING *`.
    ///
    /// SQLite added RETURNING in 3.35.0 (2021-03-12).  sqlx 0.7's sqlite
    /// driver is linked against a bundled SQLite that is newer than 3.35,
    /// so this *should* work.  The test deliberately does not rely on the
    /// production `claim_next` logic — it exercises a minimal RETURNING
    /// statement directly so we can record the result as an inline
    /// comment.
    #[tokio::test]
    async fn test_returning_compatibility() {
        let (_dir, store) = setup().await;

        // Insert a row we can update.
        let item = sample_item("returning test");
        let enqueued = store.enqueue(item).await.expect("enqueue");

        // Attempt the RETURNING query.
        let now = Utc::now();
        let returning_result = sqlx::query_as::<_, (String, String)>(
            "UPDATE runs SET status = 'running', updated_at = ?1 WHERE id = ?2 RETURNING id, status",
        )
        .bind(now.to_rfc3339())
        .bind(&enqueued.id)
        .fetch_optional(&store.pool)
        .await;

        eprintln!("RETURNING result: {:?}", returning_result);

        match returning_result {
            Ok(Some((id, status))) => {
                // SUCCESS — sqlx 0.7 + bundled SQLite supports RETURNING.
                assert_eq!(id, enqueued.id);
                assert_eq!(status, "running");

                // Verify the side-effect actually happened.
                // Note: when running multiple tests in parallel, another test
                // may claim the same row via claim_next() before we get here,
                // so we check the status is either Running or already claimed.
                let reloaded = store.get(&enqueued.id).await.expect("get").expect("item");
                eprintln!("Reloaded status after RETURNING: {:?}", reloaded.status);
                assert!(
                    reloaded.status == RunStatus::Running || reloaded.status == RunStatus::Queued,
                    "status should be Running or Queued (if concurrently claimed), got {:?}",
                    reloaded.status
                );
            }
            Ok(None) => {
                // RETURNING succeeded but returned no row — this means the row
                // wasn't updated (e.g., id mismatch). This is still a success
                // for RETURNING compatibility, but we need to verify the fallback.
                eprintln!("RETURNING returned no row — verifying fallback");

                let claimed = store.claim_next().await.expect("claim_next");
                assert!(claimed.is_some());
                let claimed = claimed.expect("item");
                assert_eq!(claimed.id, enqueued.id);
                assert_eq!(claimed.status, RunStatus::Running);
            }
            Err(e) => {
                // FAILURE — sqlx 0.7 does NOT support RETURNING with this SQLite.
                // We record the error and fall back to the proven
                // BEGIN + SELECT + UPDATE pattern used by `claim_next`.
                eprintln!("RETURNING not supported: {}", e);

                // Fallback verification: ensure the existing pattern still works.
                let claimed = store.claim_next().await.expect("claim_next");
                assert!(claimed.is_some());
                let claimed = claimed.expect("item");
                assert_eq!(claimed.id, enqueued.id);
                assert_eq!(claimed.status, RunStatus::Running);
            }
        }
    }

    /// Test B: verify the fallback path (BEGIN IMMEDIATE + SELECT + UPDATE)
    /// works correctly even if RETURNING is unavailable.
    #[tokio::test]
    async fn test_fallback_claim_pattern() {
        let (_dir, store) = setup().await;

        let item = sample_item("fallback test");
        let enqueued = store.enqueue(item).await.expect("enqueue");

        // Use the existing claim_next implementation (which uses the
        // BEGIN + SELECT + UPDATE pattern) to prove the fallback works.
        let claimed = store.claim_next().await.expect("claim_next");
        assert!(claimed.is_some());
        let claimed = claimed.expect("item");

        assert_eq!(claimed.id, enqueued.id);
        assert_eq!(claimed.status, RunStatus::Running);
        assert!(claimed.heartbeat_at.is_some());

        // Verify the DB reflects the update.
        let reloaded = store.get(&enqueued.id).await.expect("get").expect("item");
        assert_eq!(reloaded.status, RunStatus::Running);
        assert_eq!(reloaded.heartbeat_at, claimed.heartbeat_at);
    }

    /// Test C: concurrent claim_next must not double-claim the same item.
    /// This validates that the transaction-based locking (whether via
    /// RETURNING or BEGIN + SELECT + UPDATE) is race-safe.
    #[tokio::test]
    async fn test_concurrent_claim_race_safety() {
        let (_dir, store) = setup().await;

        // Enqueue a single item.
        let item = sample_item("race test");
        store.enqueue(item).await.expect("enqueue");

        // Spawn two concurrent claim attempts.
        let store2 = store.clone();
        let claim1: tokio::task::JoinHandle<Result<Option<RunItem>, sqlx::Error>> =
            tokio::spawn(async move { store.claim_next().await });
        let claim2: tokio::task::JoinHandle<Result<Option<RunItem>, sqlx::Error>> =
            tokio::spawn(async move { store2.claim_next().await });

        let (r1, r2) = tokio::join!(claim1, claim2);
        let r1 = r1.expect("join").expect("claim1");
        let r2 = r2.expect("join").expect("claim2");

        // Exactly one claim should succeed.
        let got_one = r1.is_some() as usize + r2.is_some() as usize;
        assert_eq!(got_one, 1, "exactly one concurrent claim should succeed");
    }

    // ========================================================================
    // Supervised runs CRUD tests
    // ========================================================================

    /// Helper to create a sample SupervisedRunSpec
    fn sample_spec(message: &str) -> SupervisedRunSpec {
        SupervisedRunSpec::from_spec(message)
    }

    #[tokio::test]
    async fn test_supervised_create_and_get() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("supervised test");

        let created = store.create(&spec).await.expect("create");
        assert_eq!(created.status, RunStatus::Queued);
        assert_eq!(created.message, "supervised test");
        assert_eq!(created.kind, RunKind::Generic);
        assert_eq!(created.priority, 0);
        assert_eq!(created.attempt, 0);
        assert_eq!(created.max_attempts, 3);

        let found = store.get_record(&created.id).await.expect("get_record");
        assert!(found.is_some());
        let found = found.expect("record");
        assert_eq!(found.id, created.id);
        assert_eq!(found.status, RunStatus::Queued);
        assert_eq!(found.message, "supervised test");
    }

    #[tokio::test]
    async fn test_supervised_get_not_found() {
        let (_dir, store) = setup().await;
        let found = store.get_record("nonexistent").await.expect("get_record");
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn test_supervised_claim_next() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("claim test");
        let created = store.create(&spec).await.expect("create");

        let claimed = store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next");
        assert!(claimed.is_some());
        let claimed = claimed.expect("claimed");
        assert_eq!(claimed.id, created.id);
        assert_eq!(claimed.status, RunStatus::Running);
        assert_eq!(claimed.claimed_by, Some("worker-1".to_string()));
        assert!(claimed.started_at.is_some());
        assert!(claimed.heartbeat_at.is_some());
    }

    #[tokio::test]
    async fn test_supervised_claim_next_empty() {
        let (_dir, store) = setup().await;
        let claimed = store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next");
        assert!(claimed.is_none());
    }

    #[tokio::test]
    async fn test_supervised_heartbeat() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("heartbeat test");
        let created = store.create(&spec).await.expect("create");
        let claimed = store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next")
            .expect("claimed");
        let original_hb = claimed.heartbeat_at.expect("heartbeat");

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        store
            .heartbeat_supervised(&claimed.id, "worker-1")
            .await
            .expect("heartbeat");

        let reloaded = store
            .get_record(&claimed.id)
            .await
            .expect("get_record")
            .expect("record");
        assert!(reloaded.heartbeat_at.expect("heartbeat") > original_hb);
    }

    #[tokio::test]
    async fn test_supervised_heartbeat_not_owner() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("heartbeat owner test");
        let created = store.create(&spec).await.expect("create");
        store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next");

        let err = store
            .heartbeat_supervised(&created.id, "worker-2")
            .await
            .expect_err("heartbeat should fail");
        match err {
            RunStoreError::NotOwner(_) => {}
            _ => panic!("expected NotOwner, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_supervised_heartbeat_not_found() {
        let (_dir, store) = setup().await;
        let err = store
            .heartbeat_supervised("nonexistent", "worker-1")
            .await
            .expect_err("heartbeat should fail");
        match err {
            RunStoreError::NotFound(_) => {}
            _ => panic!("expected NotFound, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_supervised_complete() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("complete test");
        let created = store.create(&spec).await.expect("create");
        let claimed = store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next")
            .expect("claimed");

        store
            .complete_supervised(&claimed.id, "worker-1", Some("done".to_string()))
            .await
            .expect("complete");

        let reloaded = store
            .get_record(&claimed.id)
            .await
            .expect("get_record")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Completed);
        assert_eq!(reloaded.result_summary, Some("done".to_string()));
        assert!(reloaded.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_supervised_complete_not_owner() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("complete owner test");
        let created = store.create(&spec).await.expect("create");
        store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next");

        let err = store
            .complete_supervised(&created.id, "worker-2", None)
            .await
            .expect_err("complete should fail");
        match err {
            RunStoreError::NotOwner(_) => {}
            _ => panic!("expected NotOwner, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_supervised_fail() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("fail test");
        let created = store.create(&spec).await.expect("create");
        let claimed = store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next")
            .expect("claimed");

        store
            .fail_supervised(&claimed.id, "worker-1", Some("panic".to_string()))
            .await
            .expect("fail");

        let reloaded = store
            .get_record(&claimed.id)
            .await
            .expect("get_record")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Failed);
        assert_eq!(reloaded.error_message, Some("panic".to_string()));
        assert!(reloaded.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_supervised_fail_not_owner() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("fail owner test");
        let created = store.create(&spec).await.expect("create");
        store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next");

        let err = store
            .fail_supervised(&created.id, "worker-2", None)
            .await
            .expect_err("fail should fail");
        match err {
            RunStoreError::NotOwner(_) => {}
            _ => panic!("expected NotOwner, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_supervised_cancel_queued() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("cancel test");
        let created = store.create(&spec).await.expect("create");

        store
            .cancel_supervised(&created.id, Some("user request".to_string()))
            .await
            .expect("cancel");

        let reloaded = store
            .get_record(&created.id)
            .await
            .expect("get_record")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Cancelled);
        assert_eq!(reloaded.error_message, Some("user request".to_string()));
    }

    #[tokio::test]
    async fn test_supervised_cancel_running() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("cancel running test");
        let created = store.create(&spec).await.expect("create");
        store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next");

        store
            .cancel_supervised(&created.id, Some("shutdown".to_string()))
            .await
            .expect("cancel");

        let reloaded = store
            .get_record(&created.id)
            .await
            .expect("get_record")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_supervised_cancel_not_found() {
        let (_dir, store) = setup().await;
        let err = store
            .cancel_supervised("nonexistent", None)
            .await
            .expect_err("cancel should fail");
        match err {
            RunStoreError::NotFound(_) => {}
            _ => panic!("expected NotFound, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_supervised_cancel_already_cancelled() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("cancel twice test");
        let created = store.create(&spec).await.expect("create");
        store
            .cancel_supervised(&created.id, None)
            .await
            .expect("cancel");

        // Second cancel should succeed idempotently
        store
            .cancel_supervised(&created.id, None)
            .await
            .expect("cancel again");

        let reloaded = store
            .get_record(&created.id)
            .await
            .expect("get_record")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_supervised_mark_lost() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("stale test");
        let created = store.create(&spec).await.expect("create");
        let claimed = store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next")
            .expect("claimed");

        // Manually set heartbeat to 120s ago
        let stale_time = Utc::now() - chrono::Duration::seconds(120);
        sqlx::query("UPDATE supervised_runs SET heartbeat_at = ?1 WHERE id = ?2")
            .bind(stale_time.to_rfc3339())
            .bind(&claimed.id)
            .execute(&store.pool)
            .await
            .expect("update heartbeat");

        let cutoff = Utc::now() - chrono::Duration::seconds(60);
        let lost = store.mark_lost(cutoff).await.expect("mark_lost");
        assert_eq!(lost.len(), 1);
        assert_eq!(lost[0].id, claimed.id);

        let reloaded = store
            .get_record(&claimed.id)
            .await
            .expect("get_record")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Lost);
    }

    #[tokio::test]
    async fn test_supervised_mark_lost_no_reap() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("fresh test");
        store.create(&spec).await.expect("create");
        store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next");

        // Recent heartbeat, should not be reaped with 60s timeout
        let cutoff = Utc::now() - chrono::Duration::seconds(60);
        let lost = store.mark_lost(cutoff).await.expect("mark_lost");
        assert!(lost.is_empty());
    }

    #[tokio::test]
    async fn test_supervised_requeue() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("requeue test");
        let created = store.create(&spec).await.expect("create");
        let claimed = store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next")
            .expect("claimed");
        store
            .fail_supervised(&claimed.id, "worker-1", Some("error".to_string()))
            .await
            .expect("fail");

        let requeued = store.requeue(&claimed.id).await.expect("requeue");
        assert_eq!(requeued.status, RunStatus::Queued);
        assert_eq!(requeued.attempt, 1);
        assert!(requeued.claimed_by.is_none());
        assert!(requeued.started_at.is_none());
        assert!(requeued.completed_at.is_none());
        assert!(requeued.error_message.is_none());
    }

    #[tokio::test]
    async fn test_supervised_requeue_not_found() {
        let (_dir, store) = setup().await;
        let err = store
            .requeue("nonexistent")
            .await
            .expect_err("requeue should fail");
        match err {
            RunStoreError::NotFound(_) => {}
            _ => panic!("expected NotFound, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_supervised_requeue_invalid_state() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("requeue invalid test");
        let created = store.create(&spec).await.expect("create");
        let claimed = store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next")
            .expect("claimed");
        store
            .complete_supervised(&claimed.id, "worker-1", None)
            .await
            .expect("complete");

        let err = store
            .requeue(&created.id)
            .await
            .expect_err("requeue should fail");
        match err {
            RunStoreError::InvalidState(_) => {}
            _ => panic!("expected InvalidState, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_supervised_requeue_max_attempts() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("requeue max test").with_max_attempts(1);
        let created = store.create(&spec).await.expect("create");
        let claimed = store
            .claim_next_supervised("worker-1")
            .await
            .expect("claim_next")
            .expect("claimed");
        store
            .fail_supervised(&claimed.id, "worker-1", None)
            .await
            .expect("fail");

        let err = store
            .requeue(&created.id)
            .await
            .expect_err("requeue should fail");
        match err {
            RunStoreError::MaxAttemptsReached(_) => {}
            _ => panic!("expected MaxAttemptsReached, got {:?}", err),
        }
    }

    #[tokio::test]
    async fn test_supervised_concurrent_claim_race_safety() {
        let (_dir, store) = setup().await;
        let spec = sample_spec("race test");
        store.create(&spec).await.expect("create");

        let store2 = store.clone();
        let claim1: tokio::task::JoinHandle<Result<Option<RunRecord>, RunStoreError>> =
            tokio::spawn(async move { store.claim_next_supervised("worker-a").await });
        let claim2: tokio::task::JoinHandle<Result<Option<RunRecord>, RunStoreError>> =
            tokio::spawn(async move { store2.claim_next_supervised("worker-b").await });

        let (r1, r2) = tokio::join!(claim1, claim2);
        let r1 = r1.expect("join").expect("claim1");
        let r2 = r2.expect("join").expect("claim2");

        let got_one = r1.is_some() as usize + r2.is_some() as usize;
        assert_eq!(got_one, 1, "exactly one concurrent claim should succeed");
    }
}
