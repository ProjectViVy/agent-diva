//! SQLite-based file index
//!
//! This module provides a persistent file index using SQLite.
//! It replaces the in-memory HashMap with a proper database for:
//! - Better query performance with large datasets
//! - ACID transactions
//! - SQL-based filtering and aggregation
//! - Automatic persistence

use crate::handle::{FileIndexEntry, FileMetadata};
use crate::Result;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};

/// SQLite-based file index
pub struct SqliteIndex {
    pool: SqlitePool,
    db_path: PathBuf,
}

impl SqliteIndex {
    /// Create a new SQLite index at the given path
    pub async fn new(db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Use SqliteConnectOptions for proper path handling on Windows
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let index = Self { pool, db_path };
        index.init().await?;

        Ok(index)
    }

    /// Initialize the database schema
    async fn init(&self) -> Result<()> {
        // Create main files table with soft delete support
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS files (
                id TEXT PRIMARY KEY NOT NULL,
                path TEXT NOT NULL,
                size INTEGER NOT NULL DEFAULT 0,
                ref_count INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                last_accessed_at TEXT,
                deleted_at TEXT,          -- NEW: soft delete timestamp
                deleted_by TEXT,          -- NEW: who deleted the file
                metadata_json TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_files_ref_count ON files(ref_count);
            CREATE INDEX IF NOT EXISTS idx_files_last_accessed ON files(last_accessed_at);
            CREATE INDEX IF NOT EXISTS idx_files_deleted_at ON files(deleted_at) WHERE deleted_at IS NOT NULL;
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Migration: Add deleted_at column if not exists (for existing databases)
        let columns: Vec<String> = sqlx::query_scalar("SELECT name FROM pragma_table_info('files')")
            .fetch_all(&self.pool)
            .await?;
        
        if !columns.contains(&"deleted_at".to_string()) {
            let _ = sqlx::query("ALTER TABLE files ADD COLUMN deleted_at TEXT")
                .execute(&self.pool)
                .await;
        }
        if !columns.contains(&"deleted_by".to_string()) {
            let _ = sqlx::query("ALTER TABLE files ADD COLUMN deleted_by TEXT")
                .execute(&self.pool)
                .await;
        }

        tracing::info!("SQLite index initialized at {:?}", self.db_path);
        Ok(())
    }

    /// Load index from existing database (alias for new)
    pub async fn load(db_path: PathBuf) -> Result<Self> {
        Self::new(db_path).await
    }

    /// Get an entry by ID
    ///
    /// # Arguments
    /// * `id` - The file ID
    /// * `include_deleted` - If true, include soft-deleted files
    pub async fn get(&self, id: &str, include_deleted: bool) -> Result<Option<FileIndexEntry>> {
        let row = if include_deleted {
            sqlx::query(
                r#"
                SELECT id, path, size, ref_count, created_at, last_accessed_at, metadata_json
                FROM files WHERE id = ?
                "#,
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query(
                r#"
                SELECT id, path, size, ref_count, created_at, last_accessed_at, metadata_json
                FROM files WHERE id = ? AND deleted_at IS NULL
                "#,
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
        };

        match row {
            Some(row) => Ok(Some(self.row_to_entry(&row)?)),
            None => Ok(None),
        }
    }

    /// Soft delete a file by ID
    ///
    /// Marks the file as deleted but does not remove it physically.
    /// The file can be restored until the retention period expires.
    pub async fn soft_delete(&self, id: &str, deleted_by: Option<&str>) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE files
            SET deleted_at = ?, deleted_by = ?
            WHERE id = ? AND deleted_at IS NULL
            "#,
        )
        .bind(Utc::now().to_rfc3339())
        .bind(deleted_by)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Restore a soft-deleted file
    ///
    /// Clears the deleted_at timestamp, making the file accessible again.
    pub async fn restore(&self, id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE files
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = ? AND deleted_at IS NOT NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List all soft-deleted files
    pub async fn list_deleted(&self) -> Result<Vec<FileIndexEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT id, path, size, ref_count, created_at, last_accessed_at, metadata_json
            FROM files
            WHERE deleted_at IS NOT NULL
            ORDER BY deleted_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(self.row_to_entry(&row)?);
        }

        Ok(entries)
    }

    /// Get soft-deleted files that are ready for permanent deletion
    ///
    /// # Arguments
    /// * `retention_days` - Files deleted more than this many days ago
    pub async fn get_expired_deletions(&self, retention_days: u32) -> Result<Vec<FileIndexEntry>> {
        let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let rows = sqlx::query(
            r#"
            SELECT id, path, size, ref_count, created_at, last_accessed_at, metadata_json
            FROM files
            WHERE deleted_at IS NOT NULL AND deleted_at < ?
            "#,
        )
        .bind(cutoff_str)
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(self.row_to_entry(&row)?);
        }

        Ok(entries)
    }

    /// Hard delete a file (physical removal from database)
    ///
    /// Use with caution - this permanently removes the index entry.
    /// The actual file data should be deleted separately.
    pub async fn hard_delete(&self, id: &str) -> Result<Option<FileIndexEntry>> {
        // First get the entry to return it
        let entry = self.get(id, true).await?;

        if entry.is_some() {
            sqlx::query("DELETE FROM files WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        Ok(entry)
    }

    /// Insert or update an entry
    pub async fn insert(&self, entry: FileIndexEntry) -> Result<()> {
        let metadata_json = serde_json::to_string(&entry.metadata)?;

        sqlx::query(
            r#"
            INSERT INTO files (id, path, size, ref_count, created_at, last_accessed_at, metadata_json)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                path = excluded.path,
                size = excluded.size,
                ref_count = excluded.ref_count,
                last_accessed_at = excluded.last_accessed_at,
                metadata_json = excluded.metadata_json,
                deleted_at = NULL,
                deleted_by = NULL
            "#,
        )
        .bind(&entry.id)
        .bind(entry.path.to_string_lossy().to_string())
        .bind(entry.size as i64)
        .bind(entry.ref_count as i64)
        .bind(entry.created_at.to_rfc3339())
        .bind(entry.last_accessed_at.map(|t| t.to_rfc3339()))
        .bind(metadata_json)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove an entry by ID
    ///
    /// This physically removes the entry from the database.
    /// Note: For soft delete, use `soft_delete()` instead.
    pub async fn remove(&self, id: &str) -> Result<Option<FileIndexEntry>> {
        // First get the entry to return it (include deleted since we're removing it)
        let entry = self.get(id, true).await?;

        if entry.is_some() {
            sqlx::query("DELETE FROM files WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await?;
        }

        Ok(entry)
    }

    /// Update reference count for an entry
    ///
    /// # Arguments
    /// * `id` - The file ID
    /// * `delta` - Change in reference count (+1 to add, -1 to release)
    ///
    /// # Returns
    /// The new reference count, or None if the entry doesn't exist
    pub async fn update_ref_count(&self, id: &str, delta: i32) -> Result<Option<usize>> {
        // Get the current entry (only non-deleted files should have their ref count updated)
        let current = self.get(id, false).await?;

        if let Some(entry) = current {
            let new_count = if delta < 0 {
                entry
                    .ref_count
                    .saturating_sub(delta.unsigned_abs() as usize)
            } else {
                entry.ref_count.saturating_add(delta as usize)
            };

            sqlx::query(
                r#"
                UPDATE files
                SET ref_count = ?, last_accessed_at = ?
                WHERE id = ?
                "#,
            )
            .bind(new_count as i64)
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .execute(&self.pool)
            .await?;

            Ok(Some(new_count))
        } else {
            Ok(None)
        }
    }

    /// Get entries suitable for cleanup
    pub async fn get_candidates_for_cleanup(
        &self,
        threshold: i32,
        max_age_days: u32,
    ) -> Result<Vec<FileIndexEntry>> {
        let cutoff = Utc::now() - chrono::Duration::days(max_age_days as i64);
        let cutoff_str = cutoff.to_rfc3339();

        let rows = sqlx::query(
            r#"
            SELECT id, path, size, ref_count, created_at, last_accessed_at, metadata_json
            FROM files
            WHERE ref_count <= ?
              AND (last_accessed_at IS NULL OR last_accessed_at < ?)
            "#,
        )
        .bind(threshold as i64)
        .bind(cutoff_str)
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(self.row_to_entry(&row)?);
        }

        Ok(entries)
    }

    /// Get total entry count
    pub async fn len(&self) -> Result<usize> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
            .fetch_one(&self.pool)
            .await?;
        Ok(count as usize)
    }

    /// Check if index is empty
    pub async fn is_empty(&self) -> Result<bool> {
        Ok(self.len().await? == 0)
    }

    /// Get all entries (use sparingly with large datasets)
    pub async fn entries(&self) -> Result<Vec<FileIndexEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT id, path, size, ref_count, created_at, last_accessed_at, metadata_json
            FROM files
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(self.row_to_entry(&row)?);
        }

        Ok(entries)
    }

    /// Get statistics about the index
    pub async fn stats(&self) -> Result<IndexStats> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_files,
                COALESCE(SUM(size), 0) as total_size,
                COALESCE(SUM(ref_count), 0) as total_refs
            FROM files
            "#,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(IndexStats {
            total_files: row.get::<i64, _>("total_files") as usize,
            total_size: row.get::<i64, _>("total_size") as u64,
            total_refs: row.get::<i64, _>("total_refs") as usize,
        })
    }

    /// Close the database connection
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Convert a database row to FileIndexEntry
    fn row_to_entry(&self, row: &sqlx::sqlite::SqliteRow) -> Result<FileIndexEntry> {
        let metadata_json: String = row.get("metadata_json");
        let metadata: FileMetadata = serde_json::from_str(&metadata_json)?;

        Ok(FileIndexEntry {
            id: row.get("id"),
            path: PathBuf::from(row.get::<String, _>("path")),
            size: row.get::<i64, _>("size") as u64,
            ref_count: row.get::<i64, _>("ref_count") as usize,
            created_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("created_at"))?
                .with_timezone(&Utc),
            last_accessed_at: row
                .get::<Option<String>, _>("last_accessed_at")
                .map(|s| DateTime::parse_from_rfc3339(&s).map(|dt| dt.with_timezone(&Utc)))
                .transpose()?,
            metadata,
        })
    }

    /// Migrate from old JSONL format
    pub async fn migrate_from_jsonl(&self, jsonl_path: &Path) -> Result<usize> {
        if !jsonl_path.exists() {
            return Ok(0);
        }

        let content = tokio::fs::read_to_string(jsonl_path).await?;
        let mut count = 0;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(entry) = serde_json::from_str::<FileIndexEntry>(line) {
                self.insert(entry).await?;
                count += 1;
            } else {
                tracing::warn!("Failed to parse index entry during migration: {}", line);
            }
        }

        tracing::info!("Migrated {} entries from JSONL to SQLite", count);
        Ok(count)
    }
}

/// Statistics for the index
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    pub total_files: usize,
    pub total_size: u64,
    pub total_refs: usize,
}

/// Backward-compatible FileIndex wrapper
///
/// This type alias allows existing code to continue working while
/// migrating from the old in-memory HashMap implementation.
pub type FileIndex = SqliteIndex;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_metadata() -> FileMetadata {
        FileMetadata {
            name: "test.txt".to_string(),
            size: 100,
            mime_type: Some("text/plain".to_string()),
            source: Some("test".to_string()),
            created_at: Utc::now(),
            last_accessed_at: None,
            preview: None,
        }
    }

    #[tokio::test]
    async fn test_sqlite_index_basic() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("index.db");

        let index = SqliteIndex::new(db_path).await.unwrap();

        // Insert
        let entry = FileIndexEntry {
            id: "abc123".to_string(),
            path: PathBuf::from("ab/c123"),
            size: 100,
            ref_count: 1,
            created_at: Utc::now(),
            last_accessed_at: None,
            metadata: create_test_metadata(),
        };

        index.insert(entry).await.unwrap();
        assert_eq!(index.len().await.unwrap(), 1);

        // Get
        let retrieved = index.get("abc123", false).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, "abc123");
        assert_eq!(retrieved.ref_count, 1);

        // Update ref count
        let new_count = index.update_ref_count("abc123", 1).await.unwrap();
        assert_eq!(new_count, Some(2));

        let updated = index.get("abc123", false).await.unwrap().unwrap();
        assert_eq!(updated.ref_count, 2);

        // Remove
        let removed = index.remove("abc123").await.unwrap();
        assert!(removed.is_some());
        assert_eq!(index.len().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_sqlite_index_stats() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("index.db");

        let index = SqliteIndex::new(db_path).await.unwrap();

        // Insert multiple entries
        for i in 0..5 {
            let entry = FileIndexEntry {
                id: format!("file{}", i),
                path: PathBuf::from(format!("f{}/{}", i, i)),
                size: 100 * (i + 1) as u64,
                ref_count: i + 1,
                created_at: Utc::now(),
                last_accessed_at: None,
                metadata: create_test_metadata(),
            };
            index.insert(entry).await.unwrap();
        }

        let stats = index.stats().await.unwrap();
        assert_eq!(stats.total_files, 5);
        assert_eq!(stats.total_size, 100 + 200 + 300 + 400 + 500);
        assert_eq!(stats.total_refs, 1 + 2 + 3 + 4 + 5);
    }

    #[tokio::test]
    async fn test_sqlite_index_cleanup_candidates() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("index.db");

        let index = SqliteIndex::new(db_path).await.unwrap();

        // Insert entry with old access time and low ref count
        let old_entry = FileIndexEntry {
            id: "old_file".to_string(),
            path: PathBuf::from("old/path"),
            size: 100,
            ref_count: 0,
            created_at: Utc::now() - chrono::Duration::days(100),
            last_accessed_at: Some(Utc::now() - chrono::Duration::days(100)),
            metadata: create_test_metadata(),
        };
        index.insert(old_entry).await.unwrap();

        // Insert entry with recent access time
        let recent_entry = FileIndexEntry {
            id: "recent_file".to_string(),
            path: PathBuf::from("recent/path"),
            size: 100,
            ref_count: 0,
            created_at: Utc::now(),
            last_accessed_at: Some(Utc::now()),
            metadata: create_test_metadata(),
        };
        index.insert(recent_entry).await.unwrap();

        // Get cleanup candidates (older than 30 days, ref_count <= 1)
        let candidates = index.get_candidates_for_cleanup(1, 30).await.unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, "old_file");
    }

    #[tokio::test]
    async fn test_migration_from_jsonl() {
        let temp_dir = TempDir::new().unwrap();
        let jsonl_path = temp_dir.path().join("index.jsonl");
        let db_path = temp_dir.path().join("index.db");

        // Create JSONL file
        let entry1 = serde_json::json!({
            "id": "file1",
            "path": "ab/c1",
            "size": 100,
            "ref_count": 1,
            "created_at": Utc::now().to_rfc3339(),
            "last_accessed_at": null,
            "metadata": {
                "name": "test1.txt",
                "size": 100,
                "mime_type": null,
                "source": null,
                "created_at": Utc::now().to_rfc3339(),
                "last_accessed_at": null,
                "preview": null
            }
        });

        let entry2 = serde_json::json!({
            "id": "file2",
            "path": "ab/c2",
            "size": 200,
            "ref_count": 2,
            "created_at": Utc::now().to_rfc3339(),
            "last_accessed_at": null,
            "metadata": {
                "name": "test2.txt",
                "size": 200,
                "mime_type": null,
                "source": null,
                "created_at": Utc::now().to_rfc3339(),
                "last_accessed_at": null,
                "preview": null
            }
        });

        tokio::fs::write(&jsonl_path, format!("{}\n{}\n", entry1, entry2))
            .await
            .unwrap();

        // Migrate
        let index = SqliteIndex::new(db_path).await.unwrap();
        let migrated = index.migrate_from_jsonl(&jsonl_path).await.unwrap();
        assert_eq!(migrated, 2);

        // Verify
        assert_eq!(index.len().await.unwrap(), 2);
        let file1 = index.get("file1", false).await.unwrap().unwrap();
        assert_eq!(file1.size, 100);
        let file2 = index.get("file2", false).await.unwrap().unwrap();
        assert_eq!(file2.size, 200);
    }
}
