//! Durable projection storage for the Neuro-Link v1 state-sync contract.
//!
//! The journal is deliberately a projection, not a domain authority.  It
//! stores only versioned channel envelopes that are safe to replay.  Session,
//! planning, approval, and memory authorities remain owned by their existing
//! services and are used to build a fresh snapshot when replay is unavailable.

use agent_diva_core::channel::{
    ChannelEnvelopeV1, CursorV1, ProjectionEventV1, StateSyncModeV1, StateSyncResultV1,
};
use chrono::{Duration, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const JOURNAL_FILE_NAME: &str = "super-channel-events.db";
pub const MAX_EVENTS_PER_STREAM: i64 = 4096;
pub const RETENTION_DAYS: i64 = 7;

#[derive(Debug, Error)]
pub enum ProjectionJournalError {
    #[error("projection journal database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("projection journal serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("projection cursor stream mismatch: expected {expected}, got {actual}")]
    CursorInvalid { expected: String, actual: String },
    #[error("projection cursor is outside the retained range: {cursor:?} (head {head:?})")]
    CursorOutOfRange { cursor: CursorV1, head: CursorV1 },
    #[error("projection cursor {cursor:?} is ahead of head {head:?}")]
    CursorAhead { cursor: CursorV1, head: CursorV1 },
    #[error("projection event method is empty")]
    EmptyMethod,
    #[error("projection envelope session mismatch: expected {expected}, got {actual}")]
    SessionMismatch { expected: String, actual: String },
    #[error("idempotency key {client_message_id} is already bound to different parameters")]
    IdempotencyConflict { client_message_id: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdempotencyRecord {
    pub request_hash: String,
    pub result: Value,
}

#[derive(Debug, Clone)]
pub struct ProjectionJournal {
    pool: SqlitePool,
    path: PathBuf,
}

impl ProjectionJournal {
    /// Open the profile-local journal and apply its small schema.
    pub async fn open(data_root: impl AsRef<Path>) -> Result<Self, ProjectionJournalError> {
        let path = data_root.as_ref().join(JOURNAL_FILE_NAME);
        Self::open_path(path).await
    }

    /// Open an explicit path.  This is public for deterministic integration
    /// tests and keeps production path policy in [`Self::open`].
    pub async fn open_path(path: impl Into<PathBuf>) -> Result<Self, ProjectionJournalError> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|error| ProjectionJournalError::Database(sqlx::Error::Io(error)))?;
        }
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .min_connections(1)
            .connect_with(options)
            .await?;
        let journal = Self { pool, path };
        journal.migrate().await?;
        Ok(journal)
    }

    /// Open an in-memory journal for unit tests.
    pub async fn in_memory() -> Result<Self, ProjectionJournalError> {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true)
            .shared_cache(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        let journal = Self {
            pool,
            path: PathBuf::from(":memory:"),
        };
        journal.migrate().await?;
        Ok(journal)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    async fn migrate(&self) -> Result<(), ProjectionJournalError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS projection_events (
                stream TEXT NOT NULL,
                sequence INTEGER NOT NULL,
                method TEXT NOT NULL,
                envelope_id TEXT NOT NULL UNIQUE,
                request_id TEXT,
                occurred_at TEXT NOT NULL,
                envelope_json TEXT NOT NULL,
                durable INTEGER NOT NULL DEFAULT 1,
                terminal INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (stream, sequence)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE UNIQUE INDEX IF NOT EXISTS projection_terminal_request
            ON projection_events(stream, request_id)
            WHERE terminal = 1 AND request_id IS NOT NULL
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS command_idempotency (
                stream TEXT NOT NULL,
                client_message_id TEXT NOT NULL,
                request_hash TEXT NOT NULL,
                result_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (stream, client_message_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS projection_acks (
                frontend_instance_id TEXT NOT NULL,
                stream TEXT NOT NULL,
                sequence INTEGER NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (frontend_instance_id, stream)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS projection_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Append one durable projection and allocate its stream cursor in the
    /// same transaction as the row write.
    pub async fn append(
        &self,
        session_key: &str,
        method: impl Into<String>,
        mut envelope: ChannelEnvelopeV1,
        terminal: bool,
    ) -> Result<ProjectionEventV1, ProjectionJournalError> {
        let method = method.into();
        if method.trim().is_empty() {
            return Err(ProjectionJournalError::EmptyMethod);
        }
        if envelope.correlation.session_key != session_key {
            return Err(ProjectionJournalError::SessionMismatch {
                expected: session_key.to_owned(),
                actual: envelope.correlation.session_key,
            });
        }
        let mut tx = self.pool.begin().await?;
        let next_sequence: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM projection_events WHERE stream = ?",
        )
        .bind(session_key)
        .fetch_one(&mut *tx)
        .await?;
        let sequence = next_sequence.max(1) as u64;
        envelope.correlation.sequence = Some(sequence);
        let envelope_json = serde_json::to_string(&envelope)?;
        sqlx::query(
            "INSERT INTO projection_events (stream, sequence, method, envelope_id, request_id, occurred_at, envelope_json, terminal) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(session_key)
        .bind(sequence as i64)
        .bind(&method)
        .bind(envelope.envelope_id.to_string())
        .bind(envelope.correlation.request_id.as_deref())
        .bind(envelope.occurred_at.to_rfc3339())
        .bind(envelope_json)
        .bind(if terminal { 1_i64 } else { 0_i64 })
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        self.prune(session_key).await?;
        Ok(ProjectionEventV1 {
            method,
            cursor: CursorV1 {
                stream: session_key.to_owned(),
                sequence,
            },
            envelope,
        })
    }

    pub async fn head(&self, session_key: &str) -> Result<CursorV1, ProjectionJournalError> {
        let sequence: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(sequence), 0) FROM projection_events WHERE stream = ?",
        )
        .bind(session_key)
        .fetch_one(&self.pool)
        .await?;
        Ok(CursorV1 {
            stream: session_key.to_owned(),
            sequence: sequence.max(0) as u64,
        })
    }

    pub async fn replay(
        &self,
        session_key: &str,
        cursor: Option<&CursorV1>,
    ) -> Result<StateSyncResultV1, ProjectionJournalError> {
        let head = self.head(session_key).await?;
        let Some(cursor) = cursor else {
            return Ok(StateSyncResultV1 {
                mode: StateSyncModeV1::Snapshot,
                head,
                events: Vec::new(),
                snapshot: Vec::new(),
            });
        };
        if cursor.stream != session_key {
            return Err(ProjectionJournalError::CursorInvalid {
                expected: session_key.to_owned(),
                actual: cursor.stream.clone(),
            });
        }
        if cursor.sequence > head.sequence {
            return Err(ProjectionJournalError::CursorAhead {
                cursor: cursor.clone(),
                head,
            });
        }
        let min_sequence: Option<i64> =
            sqlx::query_scalar("SELECT MIN(sequence) FROM projection_events WHERE stream = ?")
                .bind(session_key)
                .fetch_one(&self.pool)
                .await?;
        if let Some(min_sequence) = min_sequence {
            if cursor.sequence.saturating_add(1) < min_sequence as u64 {
                return Err(ProjectionJournalError::CursorOutOfRange {
                    cursor: cursor.clone(),
                    head,
                });
            }
        }
        let rows = sqlx::query(
            "SELECT method, sequence, envelope_json FROM projection_events WHERE stream = ? AND sequence > ? ORDER BY sequence ASC",
        )
        .bind(session_key)
        .bind(cursor.sequence as i64)
        .fetch_all(&self.pool)
        .await?;
        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let method: String = row.try_get("method")?;
            let sequence: i64 = row.try_get("sequence")?;
            let envelope_json: String = row.try_get("envelope_json")?;
            events.push(ProjectionEventV1 {
                method,
                cursor: CursorV1 {
                    stream: session_key.to_owned(),
                    sequence: sequence as u64,
                },
                envelope: serde_json::from_str(&envelope_json)?,
            });
        }
        Ok(StateSyncResultV1 {
            mode: StateSyncModeV1::Replay,
            head,
            events,
            snapshot: Vec::new(),
        })
    }

    pub async fn acknowledge(
        &self,
        frontend_instance_id: &str,
        session_key: &str,
        cursor: &CursorV1,
    ) -> Result<(), ProjectionJournalError> {
        let head = self.head(session_key).await?;
        if cursor.stream != session_key {
            return Err(ProjectionJournalError::CursorInvalid {
                expected: session_key.to_owned(),
                actual: cursor.stream.clone(),
            });
        }
        if cursor.sequence > head.sequence {
            return Err(ProjectionJournalError::CursorAhead {
                cursor: cursor.clone(),
                head,
            });
        }
        sqlx::query(
            r#"INSERT INTO projection_acks (frontend_instance_id, stream, sequence, updated_at)
               VALUES (?, ?, ?, ?)
               ON CONFLICT(frontend_instance_id, stream) DO UPDATE SET
                 sequence = MAX(projection_acks.sequence, excluded.sequence),
                 updated_at = excluded.updated_at"#,
        )
        .bind(frontend_instance_id)
        .bind(session_key)
        .bind(cursor.sequence as i64)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn lookup_idempotency(
        &self,
        session_key: &str,
        client_message_id: &str,
    ) -> Result<Option<IdempotencyRecord>, ProjectionJournalError> {
        let row = sqlx::query(
            "SELECT request_hash, result_json FROM command_idempotency WHERE stream = ? AND client_message_id = ?",
        )
        .bind(session_key)
        .bind(client_message_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| {
            Ok(IdempotencyRecord {
                request_hash: row.try_get("request_hash")?,
                result: serde_json::from_str(row.try_get::<String, _>("result_json")?.as_str())?,
            })
        })
        .transpose()
    }

    pub async fn remember_idempotency(
        &self,
        session_key: &str,
        client_message_id: &str,
        request: &Value,
        result: &Value,
    ) -> Result<(), ProjectionJournalError> {
        let request_hash = stable_hash(request)?;
        if let Some(existing) = self
            .lookup_idempotency(session_key, client_message_id)
            .await?
        {
            if existing.request_hash != request_hash {
                return Err(ProjectionJournalError::IdempotencyConflict {
                    client_message_id: client_message_id.to_owned(),
                });
            }
            return Ok(());
        }
        sqlx::query(
            "INSERT INTO command_idempotency (stream, client_message_id, request_hash, result_json, created_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT(stream, client_message_id) DO UPDATE SET request_hash = excluded.request_hash, result_json = excluded.result_json",
        )
        .bind(session_key)
        .bind(client_message_id)
        .bind(request_hash)
        .bind(serde_json::to_string(result)?)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn clear_session(&self, session_key: &str) -> Result<(), ProjectionJournalError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM projection_events WHERE stream = ?")
            .bind(session_key)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM command_idempotency WHERE stream = ?")
            .bind(session_key)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM projection_acks WHERE stream = ?")
            .bind(session_key)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn prune(&self, session_key: &str) -> Result<(), ProjectionJournalError> {
        let cutoff = (Utc::now() - Duration::days(RETENTION_DAYS)).to_rfc3339();
        sqlx::query(
            "DELETE FROM projection_events WHERE stream = ? AND (occurred_at < ? OR sequence <= (SELECT MAX(sequence) - ? FROM projection_events WHERE stream = ?))",
        )
        .bind(session_key)
        .bind(cutoff)
        .bind(MAX_EVENTS_PER_STREAM)
        .bind(session_key)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

pub fn stable_hash(value: &Value) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::channel::{
        ChannelAddress, ChannelDirection, ChannelOrigin, ChannelPayloadV1, ContentPart, Correlation,
    };

    fn envelope(session: &str, request: &str, text: &str) -> ChannelEnvelopeV1 {
        let mut correlation = Correlation::new(session);
        correlation.request_id = Some(request.to_owned());
        ChannelEnvelopeV1::new(
            ChannelDirection::InternalProjection,
            ChannelAddress::new("neuro-link", session),
            correlation,
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Stream {
                phase: agent_diva_core::channel::StreamPhase::Delta,
                parts: vec![ContentPart::Text {
                    text: text.to_owned(),
                }],
            },
        )
    }

    #[tokio::test]
    async fn append_replay_and_ack_are_cursor_safe() {
        let journal = ProjectionJournal::in_memory().await.unwrap();
        let first = journal
            .append("s", "conversation/stream", envelope("s", "r1", "a"), false)
            .await
            .unwrap();
        let second = journal
            .append("s", "conversation/stream", envelope("s", "r1", "b"), false)
            .await
            .unwrap();
        assert_eq!(first.cursor.sequence, 1);
        assert_eq!(second.cursor.sequence, 2);
        let sync = journal.replay("s", Some(&first.cursor)).await.unwrap();
        assert_eq!(sync.events.len(), 1);
        assert_eq!(sync.events[0].cursor.sequence, 2);
        journal
            .acknowledge("frontend", "s", &second.cursor)
            .await
            .unwrap();
        assert!(matches!(
            journal
                .acknowledge("frontend", "other", &second.cursor)
                .await,
            Err(ProjectionJournalError::CursorInvalid { .. })
        ));
    }

    #[tokio::test]
    async fn idempotency_records_are_hash_stable_and_clear_with_session() {
        let journal = ProjectionJournal::in_memory().await.unwrap();
        let request = serde_json::json!({"session_key":"s","parts":["hello"]});
        let result = serde_json::json!({"request_id":"r1"});
        journal
            .remember_idempotency("s", "client-1", &request, &result)
            .await
            .unwrap();
        let record = journal
            .lookup_idempotency("s", "client-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(record.request_hash, stable_hash(&request).unwrap());
        assert_eq!(record.result, result);
        journal.clear_session("s").await.unwrap();
        assert!(journal
            .lookup_idempotency("s", "client-1")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn idempotency_conflict_does_not_overwrite_the_original_result() {
        let journal = ProjectionJournal::in_memory().await.unwrap();
        journal
            .remember_idempotency(
                "s",
                "client-1",
                &serde_json::json!({"text":"one"}),
                &serde_json::json!({"request_id":"r1"}),
            )
            .await
            .unwrap();
        assert!(matches!(
            journal
                .remember_idempotency(
                    "s",
                    "client-1",
                    &serde_json::json!({"text":"two"}),
                    &serde_json::json!({"request_id":"r2"}),
                )
                .await,
            Err(ProjectionJournalError::IdempotencyConflict { .. })
        ));
        assert_eq!(
            journal
                .lookup_idempotency("s", "client-1")
                .await
                .unwrap()
                .unwrap()
                .result,
            serde_json::json!({"request_id":"r1"})
        );
    }
}
