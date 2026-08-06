//! Profile-local typed SQLite storage for normalized Memory records.
//!
//! This module is deliberately not connected to the production provider,
//! proposal apply path, Manager, Tauri, or GUI. GMH-23C/23D own those seams.

use std::{
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration as StdDuration,
};

use agent_diva_core::{
    governance::{ApprovalReceipt, ApprovalRecord, GovernanceValidationError},
    memory::{MemoryRecord, MemoryRecordValidationError, MemoryScope},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteRow},
    Row, SqlitePool,
};

use crate::LaputaPaths;

const SCHEMA_VERSION: i64 = 1;
const COMPONENT: &str = "embedded_laputa";
const BUSY_TIMEOUT: StdDuration = StdDuration::from_secs(5);

/// Initial bounded-store record capacity.
pub const MAX_MEMORY_RECORDS: i64 = 10_000;
/// Initial bounded-store aggregate canonical content capacity.
pub const MAX_MEMORY_CONTENT_BYTES: i64 = 32 * 1024 * 1024;

/// Stable typed-store failures. Messages never contain Memory content.
#[derive(Debug, thiserror::Error)]
pub enum TypedMemoryStoreError {
    #[error("typed Memory record is invalid: {0}")]
    InvalidRecord(#[from] MemoryRecordValidationError),
    #[error("record belongs to workspace {actual}, expected {expected}")]
    WorkspaceMismatch { expected: String, actual: String },
    #[error("stored database belongs to workspace {actual}, expected {expected}")]
    DatabaseWorkspaceMismatch { expected: String, actual: String },
    #[error("unsupported Embedded Laputa schema version {actual}")]
    UnsupportedSchema { actual: i64 },
    #[error("Embedded Laputa store revision conflict: expected {expected}, actual {actual}")]
    StoreRevisionConflict { expected: i64, actual: i64 },
    #[error(
        "Memory record revision conflict for {record_id}: expected {expected:?}, actual {actual:?}"
    )]
    RecordRevisionConflict {
        record_id: String,
        expected: Option<i64>,
        actual: Option<i64>,
    },
    #[error("Embedded Laputa capacity exceeded: {records} records, {content_bytes} content bytes")]
    CapacityExceeded { records: i64, content_bytes: i64 },
    #[error("FTS5 is unavailable in the active SQLite runtime")]
    FtsUnavailable,
    #[error("backup path already exists: {0}")]
    BackupExists(PathBuf),
    #[error("backup is not a valid Embedded Laputa database")]
    InvalidBackup,
    #[error("Embedded Laputa persistence failed: {0}")]
    Persistence(#[from] sqlx::Error),
    #[error("Embedded Laputa filesystem operation failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("stored Memory row is corrupt")]
    CorruptRecord,
    #[error("governed apply receipt is invalid: {0}")]
    InvalidReceipt(#[from] GovernanceValidationError),
    #[error("governed apply idempotency key conflicts with an existing operation")]
    ApplyIdempotencyConflict,
    #[error("imported Memory record {record_id} conflicts with existing content")]
    ImportConflict { record_id: String },
    #[error("workspace identity migration only accepts the recognized legacy path identity")]
    IdentityMigrationRejected,
    #[error("workspace identity migration manifest failed: {0}")]
    IdentityManifest(String),
}

/// Current database identity and optimistic concurrency revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryStoreMetadata {
    pub schema_version: i64,
    pub store_revision: i64,
    pub workspace_id: String,
}

/// Canonical record plus the row revision owned by the store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredMemoryRecord {
    pub record: MemoryRecord,
    pub revision: i64,
}

/// Bounded FTS5 candidate. Ranking is not an authority decision.
#[derive(Debug, Clone, PartialEq)]
pub struct MemorySearchHit {
    pub stored: StoredMemoryRecord,
    pub bm25: f64,
}

/// Payload-free integrity summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryStoreIntegrity {
    pub schema_version: i64,
    pub store_revision: i64,
    pub record_count: i64,
    pub tombstone_count: i64,
    pub content_bytes: i64,
    pub fts_row_count: i64,
    pub supersedes_edge_count: i64,
    pub corrupt_record_ids: Vec<String>,
    pub orphan_fts_rows: i64,
}

/// Payload-free recovery record for the canonical workspace identity upgrade.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceIdentityMigrationManifest {
    pub version: u32,
    pub state: WorkspaceIdentityMigrationState,
    pub legacy_workspace_id: String,
    pub canonical_workspace_id: String,
    pub backup_path: PathBuf,
    pub store_revision: i64,
    pub record_count: i64,
}

/// Durable identity migration phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceIdentityMigrationState {
    Prepared,
    Applied,
    RolledBack,
}

/// Receipt-bound metadata for one non-production typed apply.
#[derive(Debug, Clone)]
pub struct GovernedMemoryApply<'a> {
    pub proposal_id: &'a str,
    pub idempotency_key: &'a str,
    pub request: &'a ApprovalRecord,
    pub receipt: &'a ApprovalReceipt,
    pub applied_at: chrono::DateTime<Utc>,
}

/// Replaceable async store scoped to exactly one workspace.
#[derive(Debug, Clone)]
pub struct TypedMemoryStore {
    pool: SqlitePool,
    path: PathBuf,
    workspace_id: String,
    write_lock: Arc<tokio::sync::Mutex<()>>,
}

impl TypedMemoryStore {
    /// Open the canonical store identity, upgrading the one recognized legacy
    /// path identity through a verified, content-preserving SQLite backup.
    pub async fn open_canonical(
        workspace_root: impl AsRef<Path>,
    ) -> Result<Self, TypedMemoryStoreError> {
        let workspace_root = workspace_root.as_ref();
        if LaputaPaths::new(workspace_root).memory_database().is_file() {
            return Self::open_existing_canonical(workspace_root).await;
        }
        let canonical = agent_diva_core::workspace_identity::canonical_workspace_id(workspace_root);
        Self::open(workspace_root, canonical).await
    }

    /// Open an existing canonical store, permitting only the recognized
    /// identity-only legacy upgrade. Missing stores remain fail-closed.
    pub async fn open_existing_canonical(
        workspace_root: impl AsRef<Path>,
    ) -> Result<Self, TypedMemoryStoreError> {
        let workspace_root = workspace_root.as_ref();
        let path = LaputaPaths::new(workspace_root).memory_database();
        if !path.is_file() {
            return Err(TypedMemoryStoreError::InvalidBackup);
        }
        let canonical = agent_diva_core::workspace_identity::canonical_workspace_id(workspace_root);
        match Self::open_path(path, canonical.clone()).await {
            Ok(store) => Ok(store),
            Err(TypedMemoryStoreError::DatabaseWorkspaceMismatch { actual, .. })
                if actual
                    == agent_diva_core::workspace_identity::legacy_path_workspace_id(
                        workspace_root,
                    ) =>
            {
                Self::migrate_legacy_workspace_identity(workspace_root, &actual, &canonical).await
            }
            Err(error) => Err(error),
        }
    }

    async fn migrate_legacy_workspace_identity(
        workspace_root: &Path,
        legacy: &str,
        canonical: &str,
    ) -> Result<Self, TypedMemoryStoreError> {
        let path = LaputaPaths::new(workspace_root).memory_database();
        let store = Self::open_path(path, legacy.to_string()).await?;
        let migration_dir = workspace_root
            .join(".laputa")
            .join("migrations")
            .join("workspace-identity-v1");
        tokio::fs::create_dir_all(&migration_dir)
            .await
            .map_err(|source| TypedMemoryStoreError::Io {
                path: migration_dir.clone(),
                source,
            })?;
        let backup = migration_dir.join("memory-before.sqlite3");
        let manifest_path = migration_dir.join("manifest.json");
        if !backup.exists() {
            store.backup(&backup).await?;
        } else {
            let validation = Self::open_path(backup.clone(), legacy.to_string()).await?;
            validation.pool.close().await;
        }
        let integrity = store.integrity().await?;
        let mut manifest = WorkspaceIdentityMigrationManifest {
            version: 1,
            state: WorkspaceIdentityMigrationState::Prepared,
            legacy_workspace_id: legacy.to_string(),
            canonical_workspace_id: canonical.to_string(),
            backup_path: backup.clone(),
            store_revision: integrity.store_revision,
            record_count: integrity.record_count,
        };
        crate::atomic_write_json(&manifest_path, &manifest)
            .map_err(|error| TypedMemoryStoreError::IdentityManifest(error.to_string()))?;

        let mut tx = store.pool.begin().await?;
        let rows = sqlx::query("SELECT memory_id, record_json FROM memory_records")
            .fetch_all(&mut *tx)
            .await?;
        for row in rows {
            let memory_id: String = row.get("memory_id");
            let record_json: String = row.get("record_json");
            let mut record: MemoryRecord = serde_json::from_str(&record_json)
                .map_err(|_| TypedMemoryStoreError::CorruptRecord)?;
            if record.scope.workspace_id != legacy {
                return Err(TypedMemoryStoreError::IdentityMigrationRejected);
            }
            record.scope.workspace_id = canonical.to_string();
            let migrated_json =
                serde_json::to_string(&record).map_err(|_| TypedMemoryStoreError::CorruptRecord)?;
            sqlx::query(
                "UPDATE memory_records SET workspace_id = ?, record_json = ? WHERE memory_id = ?",
            )
            .bind(canonical)
            .bind(migrated_json)
            .bind(memory_id)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query("UPDATE memory_fts SET workspace_id = ?")
            .bind(canonical)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE schema_meta SET workspace_id = ? WHERE component = ?")
            .bind(canonical)
            .bind(COMPONENT)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        store.pool.close().await;
        let migrated = Self::open_path(store.path, canonical.to_string()).await?;
        let migrated_integrity = migrated.integrity().await?;
        if migrated_integrity.store_revision != manifest.store_revision
            || migrated_integrity.record_count != manifest.record_count
            || !migrated_integrity.corrupt_record_ids.is_empty()
            || migrated_integrity.orphan_fts_rows != 0
        {
            return Err(TypedMemoryStoreError::CorruptRecord);
        }
        manifest.state = WorkspaceIdentityMigrationState::Applied;
        crate::atomic_write_json(&manifest_path, &manifest)
            .map_err(|error| TypedMemoryStoreError::IdentityManifest(error.to_string()))?;
        Ok(migrated)
    }

    /// Restore the verified pre-identity-migration database from its manifest.
    pub async fn rollback_canonical_identity(
        workspace_root: impl AsRef<Path>,
    ) -> Result<WorkspaceIdentityMigrationManifest, TypedMemoryStoreError> {
        let workspace_root = workspace_root.as_ref();
        let migration_dir = workspace_root
            .join(".laputa")
            .join("migrations")
            .join("workspace-identity-v1");
        let manifest_path = migration_dir.join("manifest.json");
        let bytes =
            tokio::fs::read(&manifest_path)
                .await
                .map_err(|source| TypedMemoryStoreError::Io {
                    path: manifest_path.clone(),
                    source,
                })?;
        let mut manifest: WorkspaceIdentityMigrationManifest = serde_json::from_slice(&bytes)
            .map_err(|error| TypedMemoryStoreError::IdentityManifest(error.to_string()))?;
        if manifest.version != 1
            || manifest.state != WorkspaceIdentityMigrationState::Applied
            || manifest.canonical_workspace_id
                != agent_diva_core::workspace_identity::canonical_workspace_id(workspace_root)
            || manifest.legacy_workspace_id
                != agent_diva_core::workspace_identity::legacy_path_workspace_id(workspace_root)
            || manifest.backup_path != migration_dir.join("memory-before.sqlite3")
        {
            return Err(TypedMemoryStoreError::IdentityMigrationRejected);
        }
        let current = Self::open_path(
            LaputaPaths::new(workspace_root).memory_database(),
            manifest.canonical_workspace_id.clone(),
        )
        .await?;
        let current_integrity = current.integrity().await?;
        if current_integrity.store_revision != manifest.store_revision
            || current_integrity.record_count != manifest.record_count
        {
            return Err(TypedMemoryStoreError::IdentityMigrationRejected);
        }
        current.pool.close().await;
        let backup_validation = Self::open_path(
            manifest.backup_path.clone(),
            manifest.legacy_workspace_id.clone(),
        )
        .await?;
        let backup_integrity = backup_validation.integrity().await?;
        backup_validation.pool.close().await;
        if backup_integrity.store_revision != manifest.store_revision
            || backup_integrity.record_count != manifest.record_count
            || !backup_integrity.corrupt_record_ids.is_empty()
        {
            return Err(TypedMemoryStoreError::InvalidBackup);
        }
        let database = LaputaPaths::new(workspace_root).memory_database();
        tokio::fs::copy(&manifest.backup_path, &database)
            .await
            .map_err(|source| TypedMemoryStoreError::Io {
                path: database,
                source,
            })?;
        manifest.state = WorkspaceIdentityMigrationState::RolledBack;
        crate::atomic_write_json(&manifest_path, &manifest)
            .map_err(|error| TypedMemoryStoreError::IdentityManifest(error.to_string()))?;
        Ok(manifest)
    }

    /// Open or initialize `<workspace>/.laputa/memory.sqlite3`.
    pub async fn open(
        workspace_root: impl AsRef<Path>,
        workspace_id: impl Into<String>,
    ) -> Result<Self, TypedMemoryStoreError> {
        let workspace_id = workspace_id.into();
        let path = LaputaPaths::new(workspace_root.as_ref()).memory_database();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|source| {
                TypedMemoryStoreError::Io {
                    path: parent.to_path_buf(),
                    source,
                }
            })?;
        }
        Self::open_path(path, workspace_id).await
    }

    /// Open an existing store without creating or migrating filesystem state.
    pub async fn open_existing(
        workspace_root: impl AsRef<Path>,
        workspace_id: impl Into<String>,
    ) -> Result<Self, TypedMemoryStoreError> {
        let workspace_id = workspace_id.into();
        let path = LaputaPaths::new(workspace_root.as_ref()).memory_database();
        if !path.is_file() {
            return Err(TypedMemoryStoreError::InvalidBackup);
        }
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .read_only(true)
            .foreign_keys(true)
            .busy_timeout(BUSY_TIMEOUT);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        let store = Self {
            pool,
            path,
            workspace_id,
            write_lock: Arc::new(tokio::sync::Mutex::new(())),
        };
        let metadata = store.metadata().await?;
        if metadata.schema_version != SCHEMA_VERSION {
            return Err(TypedMemoryStoreError::UnsupportedSchema {
                actual: metadata.schema_version,
            });
        }
        if metadata.workspace_id != store.workspace_id {
            return Err(TypedMemoryStoreError::DatabaseWorkspaceMismatch {
                expected: store.workspace_id.clone(),
                actual: metadata.workspace_id,
            });
        }
        Ok(store)
    }

    async fn open_path(path: PathBuf, workspace_id: String) -> Result<Self, TypedMemoryStoreError> {
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true)
            .busy_timeout(BUSY_TIMEOUT);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(options)
            .await?;
        let store = Self {
            pool,
            path,
            workspace_id,
            write_lock: Arc::new(tokio::sync::Mutex::new(())),
        };
        if let Err(error) = store.initialize().await {
            store.pool.close().await;
            return Err(error);
        }
        Ok(store)
    }

    async fn initialize(&self) -> Result<(), TypedMemoryStoreError> {
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS schema_meta (
               component TEXT PRIMARY KEY,
               schema_version INTEGER NOT NULL,
               store_revision INTEGER NOT NULL,
               record_count INTEGER NOT NULL,
               content_bytes INTEGER NOT NULL,
               workspace_id TEXT NOT NULL
             )",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memory_records (
               memory_id TEXT PRIMARY KEY,
               record_revision INTEGER NOT NULL,
               kind TEXT NOT NULL,
               tenant_id TEXT NOT NULL,
               workspace_id TEXT NOT NULL,
               session_id TEXT,
               trust TEXT NOT NULL,
               sensitivity TEXT NOT NULL,
               created_at TEXT NOT NULL,
               effective_at TEXT NOT NULL,
               expires_at TEXT,
               tombstone INTEGER NOT NULL CHECK(tombstone IN (0, 1)),
               content_bytes INTEGER NOT NULL,
               record_json TEXT NOT NULL
             )",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memory_supersedes (
               memory_id TEXT NOT NULL,
               superseded_id TEXT NOT NULL,
               PRIMARY KEY(memory_id, superseded_id),
               FOREIGN KEY(memory_id) REFERENCES memory_records(memory_id) ON DELETE CASCADE
             )",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS memory_apply_journal (
               idempotency_key TEXT PRIMARY KEY,
               proposal_id TEXT NOT NULL,
               request_id TEXT NOT NULL,
               content_digest TEXT NOT NULL,
               record_id TEXT NOT NULL,
               store_revision INTEGER NOT NULL,
               record_revision INTEGER NOT NULL,
               actor_id TEXT NOT NULL,
               applied_at TEXT NOT NULL
             )",
        )
        .execute(&mut *tx)
        .await?;
        let fts = sqlx::query(
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_fts USING fts5(
               memory_id UNINDEXED,
               tenant_id UNINDEXED,
               workspace_id UNINDEXED,
               session_id UNINDEXED,
               content,
               tokenize='unicode61'
             )",
        )
        .execute(&mut *tx)
        .await;
        if fts.is_err() {
            return Err(TypedMemoryStoreError::FtsUnavailable);
        }
        let existing = sqlx::query(
            "SELECT schema_version, store_revision, workspace_id
             FROM schema_meta WHERE component = ?",
        )
        .bind(COMPONENT)
        .fetch_optional(&mut *tx)
        .await?;
        match existing {
            None => {
                sqlx::query(
                    "INSERT INTO schema_meta(
                       component, schema_version, store_revision, record_count,
                       content_bytes, workspace_id
                     ) VALUES (?, ?, 0, 0, 0, ?)",
                )
                .bind(COMPONENT)
                .bind(SCHEMA_VERSION)
                .bind(&self.workspace_id)
                .execute(&mut *tx)
                .await?;
            }
            Some(row) => {
                let version: i64 = row.get("schema_version");
                if version != SCHEMA_VERSION {
                    return Err(TypedMemoryStoreError::UnsupportedSchema { actual: version });
                }
                let actual: String = row.get("workspace_id");
                if actual != self.workspace_id {
                    return Err(TypedMemoryStoreError::DatabaseWorkspaceMismatch {
                        expected: self.workspace_id.clone(),
                        actual,
                    });
                }
            }
        }
        tx.commit().await?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Gracefully close all SQLite connections.
    pub async fn close(self) {
        self.pool.close().await;
    }

    pub async fn metadata(&self) -> Result<MemoryStoreMetadata, TypedMemoryStoreError> {
        let row = sqlx::query(
            "SELECT schema_version, store_revision, workspace_id
             FROM schema_meta WHERE component = ?",
        )
        .bind(COMPONENT)
        .fetch_one(&self.pool)
        .await?;
        Ok(MemoryStoreMetadata {
            schema_version: row.get("schema_version"),
            store_revision: row.get("store_revision"),
            workspace_id: row.get("workspace_id"),
        })
    }

    pub async fn get(
        &self,
        memory_id: &str,
    ) -> Result<Option<StoredMemoryRecord>, TypedMemoryStoreError> {
        let row = sqlx::query(
            "SELECT record_revision, record_json FROM memory_records WHERE memory_id = ?",
        )
        .bind(memory_id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|row| decode_stored(&row)).transpose()
    }

    /// List records in deterministic ID order.
    pub async fn list(&self, limit: u32) -> Result<Vec<StoredMemoryRecord>, TypedMemoryStoreError> {
        let rows = sqlx::query(
            "SELECT record_revision, record_json FROM memory_records
             ORDER BY memory_id LIMIT ?",
        )
        .bind(i64::from(limit.min(MAX_MEMORY_RECORDS as u32)))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(|row| decode_stored(&row)).collect()
    }

    /// IDs targeted by any supersedes tombstone currently in the store.
    ///
    /// Read-side projections (startup L1 index, recall, search) use this to
    /// exclude records that a subsequent tombstone has deposed, even when
    /// the target record itself has no tombstone flag set.
    pub async fn superseded_target_ids(
        &self,
    ) -> Result<std::collections::HashSet<String>, TypedMemoryStoreError> {
        let rows = sqlx::query(
            "SELECT s.superseded_id
             FROM memory_supersedes s
             JOIN memory_records r ON r.memory_id = s.memory_id
             WHERE r.tombstone = 1",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut targets = std::collections::HashSet::new();
        for row in rows {
            let target: String = row.get("superseded_id");
            targets.insert(target);
        }
        Ok(targets)
    }

    /// Import a deterministic record set in one transaction.
    ///
    /// Records already present with identical canonical JSON are treated as
    /// idempotent replay. Any conflicting ID or validation failure aborts the
    /// complete import without changing the store.
    pub async fn import_records(
        &self,
        records: Vec<MemoryRecord>,
        expected_store_revision: i64,
    ) -> Result<MemoryStoreMetadata, TypedMemoryStoreError> {
        let _write_guard = self.write_lock.lock().await;
        let now = Utc::now();
        for record in &records {
            record.validate_at(now, chrono::Duration::minutes(5))?;
            record.validate_workspace(&self.workspace_id).map_err(|_| {
                TypedMemoryStoreError::WorkspaceMismatch {
                    expected: self.workspace_id.clone(),
                    actual: record.scope.workspace_id.clone(),
                }
            })?;
        }

        let mut tx = self.pool.begin().await?;
        let actual_store: i64 =
            sqlx::query_scalar("SELECT store_revision FROM schema_meta WHERE component = ?")
                .bind(COMPONENT)
                .fetch_one(&mut *tx)
                .await?;
        if actual_store != expected_store_revision {
            return Err(TypedMemoryStoreError::StoreRevisionConflict {
                expected: expected_store_revision,
                actual: actual_store,
            });
        }

        let mut inserts = Vec::new();
        for record in records {
            let canonical =
                serde_json::to_string(&record).map_err(|_| TypedMemoryStoreError::CorruptRecord)?;
            let existing: Option<String> =
                sqlx::query_scalar("SELECT record_json FROM memory_records WHERE memory_id = ?")
                    .bind(&record.id)
                    .fetch_optional(&mut *tx)
                    .await?;
            match existing {
                Some(existing) if existing == canonical => continue,
                Some(_) => {
                    return Err(TypedMemoryStoreError::ImportConflict {
                        record_id: record.id,
                    });
                }
                None => inserts.push((record, canonical)),
            }
        }

        let counters =
            sqlx::query("SELECT record_count, content_bytes FROM schema_meta WHERE component = ?")
                .bind(COMPONENT)
                .fetch_one(&mut *tx)
                .await?;
        let next_count = counters.get::<i64, _>("record_count") + inserts.len() as i64;
        let next_bytes = counters.get::<i64, _>("content_bytes")
            + inserts
                .iter()
                .map(|(record, _)| record.content.len() as i64)
                .sum::<i64>();
        if next_count > MAX_MEMORY_RECORDS || next_bytes > MAX_MEMORY_CONTENT_BYTES {
            return Err(TypedMemoryStoreError::CapacityExceeded {
                records: next_count,
                content_bytes: next_bytes,
            });
        }

        for (record, canonical) in &inserts {
            sqlx::query(
                "INSERT INTO memory_records(
                   memory_id, record_revision, kind, tenant_id, workspace_id, session_id,
                   trust, sensitivity, created_at, effective_at, expires_at, tombstone,
                   content_bytes, record_json
                 ) VALUES (?, 1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&record.id)
            .bind(json_atom(&record.kind)?)
            .bind(&record.scope.tenant_id)
            .bind(&record.scope.workspace_id)
            .bind(&record.scope.session_id)
            .bind(json_atom(&record.trust)?)
            .bind(json_atom(&record.sensitivity)?)
            .bind(record.created_at.to_rfc3339())
            .bind(record.effective_at.to_rfc3339())
            .bind(record.expires_at.map(|value| value.to_rfc3339()))
            .bind(i64::from(record.tombstone.is_some()))
            .bind(record.content.len() as i64)
            .bind(canonical)
            .execute(&mut *tx)
            .await?;
            for superseded_id in &record.supersedes {
                sqlx::query(
                    "INSERT INTO memory_supersedes(memory_id, superseded_id) VALUES (?, ?)",
                )
                .bind(&record.id)
                .bind(superseded_id)
                .execute(&mut *tx)
                .await?;
            }
            if record.tombstone.is_none() {
                sqlx::query(
                    "INSERT INTO memory_fts(memory_id, tenant_id, workspace_id, session_id, content)
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(&record.id)
                .bind(&record.scope.tenant_id)
                .bind(&record.scope.workspace_id)
                .bind(&record.scope.session_id)
                .bind(&record.content)
                .execute(&mut *tx)
                .await?;
            }
        }
        if !inserts.is_empty() {
            sqlx::query(
                "UPDATE schema_meta
                 SET store_revision = store_revision + ?, record_count = ?, content_bytes = ?
                 WHERE component = ?",
            )
            .bind(inserts.len() as i64)
            .bind(next_count)
            .bind(next_bytes)
            .bind(COMPONENT)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        self.metadata().await
    }

    /// Insert or replace one canonical record under store and row CAS.
    pub async fn put(
        &self,
        record: MemoryRecord,
        expected_store_revision: i64,
        expected_record_revision: Option<i64>,
    ) -> Result<StoredMemoryRecord, TypedMemoryStoreError> {
        self.put_inner(
            record,
            expected_store_revision,
            expected_record_revision,
            None,
        )
        .await
    }

    /// Apply one canonical record and payload-free journal row atomically.
    ///
    /// This seam remains unregistered until the GMH-24 write cutover.
    pub async fn put_governed(
        &self,
        record: MemoryRecord,
        expected_store_revision: i64,
        expected_record_revision: Option<i64>,
        governed: GovernedMemoryApply<'_>,
    ) -> Result<StoredMemoryRecord, TypedMemoryStoreError> {
        governed.request.validate_approve_once(governed.receipt)?;
        self.put_inner(
            record,
            expected_store_revision,
            expected_record_revision,
            Some(governed),
        )
        .await
    }

    /// Remove the record produced by a governed proposal during rollback.
    pub async fn rollback_governed(
        &self,
        proposal_id: &str,
        expected_store_revision: i64,
    ) -> Result<bool, TypedMemoryStoreError> {
        let _write_guard = self.write_lock.lock().await;
        let mut tx = self.pool.begin().await?;
        let actual_store: i64 =
            sqlx::query_scalar("SELECT store_revision FROM schema_meta WHERE component = ?")
                .bind(COMPONENT)
                .fetch_one(&mut *tx)
                .await?;
        if actual_store != expected_store_revision {
            return Err(TypedMemoryStoreError::StoreRevisionConflict {
                expected: expected_store_revision,
                actual: actual_store,
            });
        }
        let record_id: Option<String> =
            sqlx::query_scalar("SELECT record_id FROM memory_apply_journal WHERE proposal_id = ?")
                .bind(proposal_id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(record_id) = record_id else {
            tx.rollback().await?;
            return Ok(false);
        };
        sqlx::query("DELETE FROM memory_fts WHERE memory_id = ?")
            .bind(&record_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM memory_records WHERE memory_id = ?")
            .bind(&record_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM memory_apply_journal WHERE proposal_id = ?")
            .bind(proposal_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE schema_meta SET store_revision = store_revision + 1 WHERE component = ?",
        )
        .bind(COMPONENT)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(true)
    }

    async fn put_inner(
        &self,
        record: MemoryRecord,
        expected_store_revision: i64,
        expected_record_revision: Option<i64>,
        governed: Option<GovernedMemoryApply<'_>>,
    ) -> Result<StoredMemoryRecord, TypedMemoryStoreError> {
        let _write_guard = self.write_lock.lock().await;
        record.validate_at(Utc::now(), chrono::Duration::minutes(5))?;
        record.validate_workspace(&self.workspace_id).map_err(|_| {
            TypedMemoryStoreError::WorkspaceMismatch {
                expected: self.workspace_id.clone(),
                actual: record.scope.workspace_id.clone(),
            }
        })?;

        let mut tx = self.pool.begin().await?;
        if let Some(governed) = governed.as_ref() {
            let existing = sqlx::query(
                "SELECT proposal_id, request_id, content_digest, record_id
                 FROM memory_apply_journal WHERE idempotency_key = ?",
            )
            .bind(governed.idempotency_key)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(existing) = existing {
                if existing.get::<String, _>("proposal_id") != governed.proposal_id
                    || existing.get::<String, _>("request_id")
                        != governed.request.correlation.request_id
                    || existing.get::<String, _>("content_digest")
                        != governed.request.content_digest.value
                    || existing.get::<String, _>("record_id") != record.id
                {
                    return Err(TypedMemoryStoreError::ApplyIdempotencyConflict);
                }
                let row = sqlx::query(
                    "SELECT record_revision, record_json FROM memory_records WHERE memory_id = ?",
                )
                .bind(&record.id)
                .fetch_one(&mut *tx)
                .await?;
                return decode_stored(&row);
            }
        }
        let actual_store: i64 =
            sqlx::query_scalar("SELECT store_revision FROM schema_meta WHERE component = ?")
                .bind(COMPONENT)
                .fetch_one(&mut *tx)
                .await?;
        if actual_store != expected_store_revision {
            return Err(TypedMemoryStoreError::StoreRevisionConflict {
                expected: expected_store_revision,
                actual: actual_store,
            });
        }
        let reserved = sqlx::query(
            "UPDATE schema_meta SET store_revision = store_revision + 1
             WHERE component = ? AND store_revision = ?",
        )
        .bind(COMPONENT)
        .bind(expected_store_revision)
        .execute(&mut *tx)
        .await;
        let reserved = match reserved {
            Ok(reserved) => reserved,
            Err(error) if is_sqlite_busy(&error) => {
                tx.rollback().await?;
                let actual = self.metadata().await?.store_revision;
                return Err(TypedMemoryStoreError::StoreRevisionConflict {
                    expected: expected_store_revision,
                    actual,
                });
            }
            Err(error) => return Err(error.into()),
        };
        if reserved.rows_affected() != 1 {
            let actual =
                sqlx::query_scalar("SELECT store_revision FROM schema_meta WHERE component = ?")
                    .bind(COMPONENT)
                    .fetch_one(&mut *tx)
                    .await?;
            return Err(TypedMemoryStoreError::StoreRevisionConflict {
                expected: expected_store_revision,
                actual,
            });
        }
        let actual_record: Option<i64> =
            sqlx::query_scalar("SELECT record_revision FROM memory_records WHERE memory_id = ?")
                .bind(&record.id)
                .fetch_optional(&mut *tx)
                .await?;
        if actual_record != expected_record_revision {
            return Err(TypedMemoryStoreError::RecordRevisionConflict {
                record_id: record.id.clone(),
                expected: expected_record_revision,
                actual: actual_record,
            });
        }

        let counters =
            sqlx::query("SELECT record_count, content_bytes FROM schema_meta WHERE component = ?")
                .bind(COMPONENT)
                .fetch_one(&mut *tx)
                .await?;
        let current_count: i64 = counters.get("record_count");
        let current_bytes: i64 = counters.get("content_bytes");
        let replaced_bytes: i64 = sqlx::query_scalar(
            "SELECT COALESCE((SELECT content_bytes FROM memory_records WHERE memory_id = ?), 0)",
        )
        .bind(&record.id)
        .fetch_one(&mut *tx)
        .await?;
        let next_count = current_count + i64::from(actual_record.is_none());
        let next_bytes = current_bytes - replaced_bytes + record.content.len() as i64;
        if next_count > MAX_MEMORY_RECORDS || next_bytes > MAX_MEMORY_CONTENT_BYTES {
            return Err(TypedMemoryStoreError::CapacityExceeded {
                records: next_count,
                content_bytes: next_bytes,
            });
        }

        let next_record_revision = actual_record.unwrap_or(0) + 1;
        let record_json =
            serde_json::to_string(&record).map_err(|_| TypedMemoryStoreError::CorruptRecord)?;
        sqlx::query(
            "INSERT INTO memory_records(
               memory_id, record_revision, kind, tenant_id, workspace_id, session_id,
               trust, sensitivity, created_at, effective_at, expires_at, tombstone,
               content_bytes, record_json
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(memory_id) DO UPDATE SET
               record_revision=excluded.record_revision, kind=excluded.kind,
               tenant_id=excluded.tenant_id, workspace_id=excluded.workspace_id,
               session_id=excluded.session_id, trust=excluded.trust,
               sensitivity=excluded.sensitivity, created_at=excluded.created_at,
               effective_at=excluded.effective_at, expires_at=excluded.expires_at,
               tombstone=excluded.tombstone, content_bytes=excluded.content_bytes,
               record_json=excluded.record_json",
        )
        .bind(&record.id)
        .bind(next_record_revision)
        .bind(json_atom(&record.kind)?)
        .bind(&record.scope.tenant_id)
        .bind(&record.scope.workspace_id)
        .bind(&record.scope.session_id)
        .bind(json_atom(&record.trust)?)
        .bind(json_atom(&record.sensitivity)?)
        .bind(record.created_at.to_rfc3339())
        .bind(record.effective_at.to_rfc3339())
        .bind(record.expires_at.map(|value| value.to_rfc3339()))
        .bind(i64::from(record.tombstone.is_some()))
        .bind(record.content.len() as i64)
        .bind(record_json)
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM memory_supersedes WHERE memory_id = ?")
            .bind(&record.id)
            .execute(&mut *tx)
            .await?;
        for superseded_id in &record.supersedes {
            sqlx::query("INSERT INTO memory_supersedes(memory_id, superseded_id) VALUES (?, ?)")
                .bind(&record.id)
                .bind(superseded_id)
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM memory_fts WHERE memory_id = ?")
            .bind(&record.id)
            .execute(&mut *tx)
            .await?;
        let suppressed_by_tombstone: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM memory_supersedes s
             JOIN memory_records r ON r.memory_id = s.memory_id
             WHERE s.superseded_id = ? AND r.tombstone = 1",
        )
        .bind(&record.id)
        .fetch_one(&mut *tx)
        .await?;
        if record.tombstone.is_none() && suppressed_by_tombstone == 0 {
            sqlx::query(
                "INSERT INTO memory_fts(memory_id, tenant_id, workspace_id, session_id, content)
                 VALUES (?, ?, ?, ?, ?)",
            )
            .bind(&record.id)
            .bind(&record.scope.tenant_id)
            .bind(&record.scope.workspace_id)
            .bind(&record.scope.session_id)
            .bind(&record.content)
            .execute(&mut *tx)
            .await?;
        }
        if record.tombstone.is_some() {
            for superseded_id in &record.supersedes {
                sqlx::query("DELETE FROM memory_fts WHERE memory_id = ?")
                    .bind(superseded_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
        sqlx::query(
            "UPDATE schema_meta SET record_count = ?, content_bytes = ?
             WHERE component = ?",
        )
        .bind(next_count)
        .bind(next_bytes)
        .bind(COMPONENT)
        .execute(&mut *tx)
        .await?;
        if let Some(governed) = governed {
            sqlx::query(
                "INSERT INTO memory_apply_journal(
                   idempotency_key, proposal_id, request_id, content_digest,
                   record_id, store_revision, record_revision, actor_id, applied_at
                 ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(governed.idempotency_key)
            .bind(governed.proposal_id)
            .bind(&governed.request.correlation.request_id)
            .bind(&governed.request.content_digest.value)
            .bind(&record.id)
            .bind(expected_store_revision + 1)
            .bind(next_record_revision)
            .bind(&governed.receipt.decided_by.id)
            .bind(governed.applied_at.to_rfc3339())
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(StoredMemoryRecord {
            record,
            revision: next_record_revision,
        })
    }

    /// Return FTS candidates restricted to an exact logical scope.
    pub async fn search(
        &self,
        query: &str,
        scope: &MemoryScope,
        limit: u32,
    ) -> Result<Vec<MemorySearchHit>, TypedMemoryStoreError> {
        if scope.workspace_id != self.workspace_id {
            return Err(TypedMemoryStoreError::WorkspaceMismatch {
                expected: self.workspace_id.clone(),
                actual: scope.workspace_id.clone(),
            });
        }
        let rows = sqlx::query(
            "SELECT r.record_revision, r.record_json, bm25(memory_fts) AS rank
             FROM memory_fts
             JOIN memory_records r ON r.memory_id = memory_fts.memory_id
             WHERE memory_fts MATCH ?
               AND memory_fts.tenant_id = ?
               AND memory_fts.workspace_id = ?
               AND ((? IS NULL AND memory_fts.session_id IS NULL) OR memory_fts.session_id = ?)
               AND r.tombstone = 0
             ORDER BY rank, r.memory_id
             LIMIT ?",
        )
        .bind(query)
        .bind(&scope.tenant_id)
        .bind(&scope.workspace_id)
        .bind(&scope.session_id)
        .bind(&scope.session_id)
        .bind(i64::from(limit.min(100)))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(MemorySearchHit {
                    stored: decode_stored(&row)?,
                    bm25: row.get("rank"),
                })
            })
            .collect()
    }

    /// Return workspace-global records plus records owned by the exact session.
    ///
    /// This is the visibility shape required by Recall. It intentionally does
    /// not change the exact-scope semantics of [`Self::search`].
    pub async fn search_visible(
        &self,
        query: &str,
        scope: &MemoryScope,
        limit: u32,
    ) -> Result<Vec<MemorySearchHit>, TypedMemoryStoreError> {
        if scope.workspace_id != self.workspace_id {
            return Err(TypedMemoryStoreError::WorkspaceMismatch {
                expected: self.workspace_id.clone(),
                actual: scope.workspace_id.clone(),
            });
        }
        let rows = sqlx::query(
            "SELECT r.record_revision, r.record_json, bm25(memory_fts) AS rank
             FROM memory_fts
             JOIN memory_records r ON r.memory_id = memory_fts.memory_id
             WHERE memory_fts MATCH ?
               AND memory_fts.tenant_id = ?
               AND memory_fts.workspace_id = ?
               AND (memory_fts.session_id IS NULL OR memory_fts.session_id = ?)
               AND r.tombstone = 0
             ORDER BY rank, r.effective_at DESC, r.memory_id
             LIMIT ?",
        )
        .bind(query)
        .bind(&scope.tenant_id)
        .bind(&scope.workspace_id)
        .bind(&scope.session_id)
        .bind(i64::from(limit.min(100)))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(MemorySearchHit {
                    stored: decode_stored(&row)?,
                    bm25: row.get("rank"),
                })
            })
            .collect()
    }

    pub async fn integrity(&self) -> Result<MemoryStoreIntegrity, TypedMemoryStoreError> {
        let metadata = self.metadata().await?;
        let record_count = scalar(&self.pool, "SELECT COUNT(*) FROM memory_records").await?;
        let tombstone_count = scalar(
            &self.pool,
            "SELECT COUNT(*) FROM memory_records WHERE tombstone = 1",
        )
        .await?;
        let content_bytes = scalar(
            &self.pool,
            "SELECT COALESCE(SUM(content_bytes), 0) FROM memory_records",
        )
        .await?;
        let fts_row_count = scalar(&self.pool, "SELECT COUNT(*) FROM memory_fts").await?;
        let supersedes_edge_count =
            scalar(&self.pool, "SELECT COUNT(*) FROM memory_supersedes").await?;
        let orphan_fts_rows = scalar(
            &self.pool,
            "SELECT COUNT(*) FROM memory_fts f
             LEFT JOIN memory_records r ON r.memory_id = f.memory_id
             WHERE r.memory_id IS NULL OR r.tombstone = 1",
        )
        .await?;
        let rows = sqlx::query(
            "SELECT memory_id, workspace_id, tombstone, content_bytes, record_json
             FROM memory_records ORDER BY memory_id",
        )
        .fetch_all(&self.pool)
        .await?;
        let corrupt_record_ids = rows
            .into_iter()
            .filter_map(|row| {
                let id: String = row.get("memory_id");
                let json: String = row.get("record_json");
                let valid = serde_json::from_str::<MemoryRecord>(&json)
                    .ok()
                    .filter(|record| {
                        record.id == id
                            && record.scope.workspace_id == row.get::<String, _>("workspace_id")
                            && i64::from(record.tombstone.is_some())
                                == row.get::<i64, _>("tombstone")
                            && record.content.len() as i64 == row.get::<i64, _>("content_bytes")
                            && record
                                .validate_at(Utc::now(), chrono::Duration::minutes(5))
                                .is_ok()
                            && record.validate_workspace(&self.workspace_id).is_ok()
                    })
                    .is_some();
                (!valid).then_some(id)
            })
            .collect();
        Ok(MemoryStoreIntegrity {
            schema_version: metadata.schema_version,
            store_revision: metadata.store_revision,
            record_count,
            tombstone_count,
            content_bytes,
            fts_row_count,
            supersedes_edge_count,
            corrupt_record_ids,
            orphan_fts_rows,
        })
    }

    /// Create a transactionally consistent SQLite snapshot using `VACUUM INTO`.
    pub async fn backup(&self, destination: impl AsRef<Path>) -> Result<(), TypedMemoryStoreError> {
        let destination = destination.as_ref();
        if destination.exists() {
            return Err(TypedMemoryStoreError::BackupExists(
                destination.to_path_buf(),
            ));
        }
        let escaped = destination.to_string_lossy().replace('\'', "''");
        sqlx::query(&format!("VACUUM INTO '{escaped}'"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Close this store, validate a backup, restore it, and reopen the same identity.
    pub async fn restore(self, backup: impl AsRef<Path>) -> Result<Self, TypedMemoryStoreError> {
        let backup = backup.as_ref().to_path_buf();
        if !backup.is_file() {
            return Err(TypedMemoryStoreError::InvalidBackup);
        }
        let validation = Self::open_path(backup.clone(), self.workspace_id.clone()).await;
        let valid = validation.map_err(|_| TypedMemoryStoreError::InvalidBackup)?;
        valid.pool.close().await;
        self.pool.close().await;
        tokio::fs::copy(&backup, &self.path)
            .await
            .map_err(|source| TypedMemoryStoreError::Io {
                path: self.path.clone(),
                source,
            })?;
        Self::open_path(self.path, self.workspace_id).await
    }
}

fn json_atom<T: Serialize>(value: &T) -> Result<String, TypedMemoryStoreError> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .ok_or(TypedMemoryStoreError::CorruptRecord)
}

fn decode_stored(row: &SqliteRow) -> Result<StoredMemoryRecord, TypedMemoryStoreError> {
    let json: String = row.get("record_json");
    let record = serde_json::from_str(&json).map_err(|_| TypedMemoryStoreError::CorruptRecord)?;
    Ok(StoredMemoryRecord {
        record,
        revision: row.get("record_revision"),
    })
}

async fn scalar(pool: &SqlitePool, query: &str) -> Result<i64, TypedMemoryStoreError> {
    Ok(sqlx::query_scalar(query).fetch_one(pool).await?)
}

fn is_sqlite_busy(error: &sqlx::Error) -> bool {
    matches!(
        error,
        sqlx::Error::Database(database)
            if matches!(database.code().as_deref(), Some("5" | "6" | "261" | "517"))
    )
}
