//! # 频道文件管理模块 - Channel File Management Module
//!
//! 本模块实现了**频道文件上传支持**，允许在逻辑上隔离的文件管理。
//! 一个文件可以属于多个频道，实现共享和复用。
//!
//! ## 核心概念
//!
//! ### 频道 vs 文件
//!
//! 在这个实现中，**文件存储是全局的**（复用 SHA256 去重），
//! 但每个文件可以与多个**频道**关联。
//!
//! ```text
//! 全局文件存储:
//!   sha256:abc123 → /data/ab/c123 (物理存储)
//!                      ↑
//!                      │ 引用计数 ref_count = 3
//!
//! 频道关联:
//!   channel:telegram:chat_1 → sha256:abc123 (通过 channel_files 表)
//!   channel:discord:server_2 → sha256:abc123
//! ```
//!
//! 这样，同一个文件在物理上只存储一份，但在逻辑上可以属于多个频道。
//!
//! ### 数据库设计
//!
//! 新增 `channel_files` 关联表：
//!
//! | 字段 | 类型 | 说明 |
//! |------|------|------|
//! | id | INTEGER | 主键 |
//! | channel_id | TEXT | 频道标识符 |
//! | file_id | TEXT | 关联的文件ID (外键) |
//! | uploaded_by | TEXT | 上传者标识 |
//! | uploaded_at | TEXT | 上传时间 |
//! | message_id | TEXT | 关联的消息ID (可选) |
//!
//! ## 使用场景
//!
//! ### 场景1: Telegram 群组文件管理
//!
//! ```ignore
//! // 上传文件到 Telegram 频道
//! let handle = channel_manager
//!     .upload_to_channel("telegram:chat_123", data, metadata)
//!     .await?;
//!
//! // 列出该群组的所有文件
//! let files = channel_manager.list_channel_files("telegram:chat_123").await?;
//!
//! // 获取特定文件
//! let file = channel_manager.get_channel_file("telegram:chat_123", &handle.id).await?;
//! ```
//!
//! ### 场景2: Discord 服务器文件共享
//!
//! ```ignore
//! // 在 Discord 服务器上传文件
//! let handle = channel_manager
//!     .upload_to_channel("discord:server_456:channel_789", data, metadata)
//!     .await?;
//!
//! // 如果同一个文件已在其他频道存在，直接创建关联（节省存储）
//! // 文件ID相同，但 channel_files 表中会新增一条记录
//! ```
//!
//! ### 场景3: 清理频道时保留共享文件
//!
//! ```ignore
//! // 删除频道时，cleanup=false 只移除关联，不删除物理文件
//! channel_manager.delete_channel("discord:server_456:channel_789", false).await?;
//!
//! // 如果其他频道还在用这个文件，物理文件不会被删除
//! // 只有当 ref_count = 0 且没有任何频道关联时，才会被清理
//! ```
//!
//! ## ChannelManager 使用方法
//!
//! ### 基本用法
//!
//! ```rust,ignore
//! use agent_diva_files::{FileManager, channel::{ChannelManager, ChannelFileInfo}};
//!
//! // 创建 ChannelManager（需要已有的 FileManager）
//! let channel_manager = ChannelManager::new(file_manager.clone());
//!
//! // 上传文件到频道
//! let handle = channel_manager
//!     .upload_to_channel("my-channel", b"hello world", metadata)
//!     .await?;
//!
//! // 列出频道文件
//! let files = channel_manager.list_channel_files("my-channel").await?;
//! for file_info in files {
//!     println!("File: {} (uploaded by {:?})",
//!              file_info.file.id,
//!              file_info.uploaded_by);
//! }
//! ```
//!
//! ## 频道ID格式约定
//!
//! 频道ID的格式由应用层决定，建议格式：
//!
//! | 平台 | 格式 | 示例 |
//! |------|------|------|
//! | Telegram | `telegram:chat_{id}` | `telegram:chat_123456` |
//! | Discord | `discord:{server}:{channel}` | `discord:987654:channel_111` |
//! | Slack | `slack:{team}:{channel}` | `slack:T01234:C56789` |
//! | UI | `ui:project_{id}` | `ui:project_42` |
//!
//! 格式约定的目的是：
//! - 避免不同平台的文件冲突
//! - 便于调试和追踪
//! - 但 ChannelManager 本身不强制验证格式

use crate::handle::{FileHandle, FileIndexEntry, FileMetadata};
use crate::manager::FileManager;
use crate::Result;
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::PathBuf;
use std::sync::Arc;

/// Channel file information - combines file entry with channel-specific metadata
#[derive(Debug, Clone)]
pub struct ChannelFileInfo {
    /// The file entry (shared across all channels)
    pub file: FileIndexEntry,

    /// Who uploaded this file to this channel
    pub uploaded_by: Option<String>,

    /// When the file was uploaded to this channel
    pub uploaded_at: DateTime<Utc>,

    /// Associated message ID (platform-specific)
    pub message_id: Option<String>,
}

/// Channel statistics
#[derive(Debug, Clone)]
pub struct ChannelStats {
    /// The channel ID
    pub channel_id: String,

    /// Total number of file references in this channel
    pub total_files: usize,

    /// Total size of all files in this channel (accounting for sharing)
    pub total_size: u64,

    /// Number of files that are unique to this channel
    /// (files not shared with any other channel)
    pub unique_files: usize,
}

/// Channel manager - handles channel-specific file operations
///
/// Provides logical isolation and tracking for files across different channels.
/// The actual file storage is global and shared via SHA256 deduplication.
pub struct ChannelManager {
    /// Shared file manager for actual storage
    file_manager: Arc<FileManager>,

    /// Database pool for channel-specific metadata
    pool: SqlitePool,

    /// Database path (for debugging/reconnection)
    #[allow(dead_code)]
    db_path: PathBuf,
}

impl ChannelManager {
    /// Create a new channel manager
    ///
    /// # Arguments
    /// * `file_manager` - Shared FileManager for file storage
    /// * `db_path` - Path to the channel metadata database
    ///
    /// # Example
    /// ```ignore
    /// use agent_diva_files::FileManager;
    /// use agent_diva_files::channel::ChannelManager;
    ///
    /// let file_manager = FileManager::new(config).await?;
    /// let channel_manager = ChannelManager::new(
    ///     Arc::new(file_manager),
    ///     PathBuf::from("channels.db"),
    /// ).await?;
    /// ```
    pub async fn new(file_manager: Arc<FileManager>, db_path: PathBuf) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(3)
            .connect_with(options)
            .await?;

        let manager = Self {
            file_manager,
            pool,
            db_path: db_path.clone(),
        };

        manager.init_schema().await?;

        tracing::info!("ChannelManager initialized with database at {:?}", db_path);
        Ok(manager)
    }

    /// Initialize the database schema
    async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS channel_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_id TEXT NOT NULL,
                file_id TEXT NOT NULL,
                uploaded_by TEXT,
                uploaded_at TEXT NOT NULL,
                message_id TEXT,
                UNIQUE(channel_id, file_id)
            );

            CREATE INDEX IF NOT EXISTS idx_channel_files_channel ON channel_files(channel_id);
            CREATE INDEX IF NOT EXISTS idx_channel_files_file ON channel_files(file_id);
            CREATE INDEX IF NOT EXISTS idx_channel_files_uploaded_at ON channel_files(uploaded_at);
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ==========================================================================
    // File Upload Operations
    // ==========================================================================

    /// Upload a file to a specific channel
    ///
    /// If the file content already exists (same SHA256 hash), it will be
    /// deduplicated and associated with the new channel instead of storing
    /// duplicate data.
    ///
    /// # Arguments
    /// * `channel_id` - The channel identifier (e.g., "telegram:chat_123")
    /// * `data` - File content bytes
    /// * `metadata` - File metadata (name, size, mime_type, etc.)
    /// * `uploaded_by` - Optional uploader identifier
    /// * `message_id` - Optional associated message ID
    ///
    /// # Returns
    /// A `FileHandle` for the stored file
    ///
    /// # Example
    /// ```ignore
    /// let handle = channel_manager
    ///     .upload_to_channel(
    ///         "telegram:chat_123",
    ///         b"file content",
    ///         FileMetadata { name: "document.pdf", .. },
    ///         Some("user_456"),
    ///         Some("msg_789"),
    ///     )
    ///     .await?;
    /// ```
    pub async fn upload_to_channel(
        &self,
        channel_id: &str,
        data: &[u8],
        metadata: FileMetadata,
        uploaded_by: Option<&str>,
        message_id: Option<&str>,
    ) -> Result<FileHandle> {
        // Step 1: Store the file globally (deduplication happens here)
        let handle = self.file_manager.store(data, metadata).await?;

        // Step 2: Create channel association
        self.add_file_to_channel(channel_id, &handle.id, uploaded_by, message_id)
            .await?;

        tracing::info!(
            "Uploaded file {} to channel {} (uploaded by {:?})",
            handle.id,
            channel_id,
            uploaded_by
        );

        Ok(handle)
    }

    /// Add an existing file to a channel
    ///
    /// Use this when a file has already been uploaded and you just want
    /// to associate it with a channel.
    ///
    /// # Returns
    /// `Ok(true)` if a new association was created, `Ok(false)` if it already existed
    pub async fn add_file_to_channel(
        &self,
        channel_id: &str,
        file_id: &str,
        uploaded_by: Option<&str>,
        message_id: Option<&str>,
    ) -> Result<bool> {
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO channel_files (channel_id, file_id, uploaded_by, uploaded_at, message_id)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(channel_id)
        .bind(file_id)
        .bind(uploaded_by)
        .bind(Utc::now().to_rfc3339())
        .bind(message_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==========================================================================
    // File Listing and Retrieval
    // ==========================================================================

    /// List all files in a channel
    ///
    /// Returns files with their channel-specific metadata (uploader, upload time, etc.)
    ///
    /// # Arguments
    /// * `channel_id` - The channel to list files for
    ///
    /// # Returns
    /// List of files in the channel, newest first
    ///
    /// # Example
    /// ```ignore
    /// let files = channel_manager.list_channel_files("telegram:chat_123").await?;
    /// for info in files {
    ///     println!("{} - uploaded by {:?} at {}",
    ///              info.file.id, info.uploaded_by, info.uploaded_at);
    /// }
    /// ```
    pub async fn list_channel_files(&self, channel_id: &str) -> Result<Vec<ChannelFileInfo>> {
        let rows = sqlx::query(
            r#"
            SELECT
                cf.channel_id, cf.file_id, cf.uploaded_by, cf.uploaded_at, cf.message_id,
                f.id, f.path, f.size, f.ref_count, f.created_at, f.last_accessed_at, f.metadata_json
            FROM channel_files cf
            JOIN files f ON cf.file_id = f.id
            WHERE cf.channel_id = ? AND f.deleted_at IS NULL
            ORDER BY cf.uploaded_at DESC
            "#,
        )
        .bind(channel_id)
        .fetch_all(&self.pool)
        .await?;

        let mut results = Vec::new();
        for row in rows {
            let file = self.row_to_entry(&row)?;
            let channel_info = ChannelFileInfo {
                file,
                uploaded_by: row.get("uploaded_by"),
                uploaded_at: DateTime::parse_from_rfc3339(&row.get::<String, _>("uploaded_at"))
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
                message_id: row.get("message_id"),
            };
            results.push(channel_info);
        }

        Ok(results)
    }

    /// Get a specific file from a channel
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID
    /// * `file_id` - The file ID to retrieve
    ///
    /// # Returns
    /// `FileHandle` if found, error otherwise
    pub async fn get_channel_file(&self, channel_id: &str, file_id: &str) -> Result<FileHandle> {
        // First check if this file is associated with this channel
        let exists = sqlx::query(
            r#"
            SELECT 1 FROM channel_files
            WHERE channel_id = ? AND file_id = ?
            "#,
        )
        .bind(channel_id)
        .bind(file_id)
        .fetch_optional(&self.pool)
        .await?;

        if exists.is_none() {
            return Err(crate::FileError::NotFound(format!(
                "File {} not found in channel {}",
                file_id, channel_id
            )));
        }

        // Get the file handle from the file manager
        self.file_manager.get(file_id).await
    }

    /// List all channels a file belongs to
    ///
    /// # Returns
    /// List of channel IDs that contain this file
    pub async fn list_file_channels(&self, file_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT channel_id FROM channel_files
            WHERE file_id = ?
            ORDER BY uploaded_at DESC
            "#,
        )
        .bind(file_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| r.get("channel_id")).collect())
    }

    // ==========================================================================
    // File Removal
    // ==========================================================================

    /// Remove a file from a channel (but don't delete the actual file)
    ///
    /// This only removes the channel association, not the physical file.
    /// The file will continue to exist if other channels reference it.
    ///
    /// # Arguments
    /// * `channel_id` - The channel to remove from
    /// * `file_id` - The file to remove
    ///
    /// # Returns
    /// `Ok(true)` if association was removed, `Ok(false)` if it didn't exist
    pub async fn remove_from_channel(&self, channel_id: &str, file_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM channel_files
            WHERE channel_id = ? AND file_id = ?
            "#,
        )
        .bind(channel_id)
        .bind(file_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() > 0 {
            tracing::info!("Removed file {} from channel {}", file_id, channel_id);
        }

        Ok(result.rows_affected() > 0)
    }

    /// Delete an entire channel
    ///
    /// # Arguments
    /// * `channel_id` - The channel to delete
    /// * `cleanup` - If `true`, also delete files that are only in this channel
    ///   If `false`, only remove the channel associations
    ///
    /// # Behavior
    ///
    /// With `cleanup = false`:
    /// - Only removes channel_file associations
    /// - Physical files remain and can still be accessed via other channels
    ///
    /// With `cleanup = true`:
    /// - Removes channel_file associations
    /// - For files unique to this channel, also soft-deletes them
    /// - Files shared with other channels are NOT deleted
    ///
    /// # Example
    /// ```ignore
    /// // Delete channel but keep shared files
    /// channel_manager.delete_channel("temp_channel", false).await?;
    ///
    /// // Delete channel and cleanup unique files
    /// channel_manager.delete_channel("archive_channel", true).await?;
    /// ```
    pub async fn delete_channel(&self, channel_id: &str, cleanup: bool) -> Result<usize> {
        if cleanup {
            // Find files unique to this channel
            let unique_files = self.find_unique_channel_files(channel_id).await?;
            let mut deleted = 0;

            // Soft delete unique files
            for file_id in unique_files {
                if self
                    .file_manager
                    .soft_delete(&file_id, Some(channel_id))
                    .await?
                {
                    deleted += 1;
                }
            }

            // Remove all channel associations
            sqlx::query("DELETE FROM channel_files WHERE channel_id = ?")
                .bind(channel_id)
                .execute(&self.pool)
                .await?;

            tracing::info!(
                "Deleted channel {} and soft-deleted {} unique files",
                channel_id,
                deleted
            );

            Ok(deleted)
        } else {
            // Just remove associations
            let result = sqlx::query("DELETE FROM channel_files WHERE channel_id = ?")
                .bind(channel_id)
                .execute(&self.pool)
                .await?;

            tracing::info!(
                "Deleted channel {} associations (cleanup=false, files preserved)",
                channel_id
            );

            Ok(result.rows_affected() as usize)
        }
    }

    /// Find files that are unique to a channel (not in any other channel)
    async fn find_unique_channel_files(&self, channel_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT cf.file_id
            FROM channel_files cf
            WHERE cf.channel_id = ?
            AND cf.file_id NOT IN (
                SELECT file_id FROM channel_files WHERE channel_id != ?
            )
            "#,
        )
        .bind(channel_id)
        .bind(channel_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| r.get("file_id")).collect())
    }

    // ==========================================================================
    // Statistics
    // ==========================================================================

    /// Get statistics for a channel
    ///
    /// # Returns
    /// `ChannelStats` with total files, total size, and unique files count
    pub async fn channel_stats(&self, channel_id: &str) -> Result<ChannelStats> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_files,
                COALESCE(SUM(f.size), 0) as total_size,
                COUNT(DISTINCT cf.file_id) as unique_files
            FROM channel_files cf
            JOIN files f ON cf.file_id = f.id
            WHERE cf.channel_id = ? AND f.deleted_at IS NULL
            "#,
        )
        .bind(channel_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(ChannelStats {
            channel_id: channel_id.to_string(),
            total_files: row.get::<i64, _>("total_files") as usize,
            total_size: row.get::<i64, _>("total_size") as u64,
            unique_files: row.get::<i64, _>("unique_files") as usize,
        })
    }

    /// List all channels that have files
    ///
    /// # Returns
    /// List of channel IDs with at least one file
    pub async fn list_channels(&self) -> Result<Vec<String>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT channel_id FROM channel_files
            ORDER BY channel_id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| r.get("channel_id")).collect())
    }

    // ==========================================================================
    // Helper Methods
    // ==========================================================================

    /// Convert a database row to FileIndexEntry
    ///
    /// The row must contain columns: id, path, size, ref_count, created_at, last_accessed_at, metadata_json
    fn row_to_entry(&self, row: &sqlx::sqlite::SqliteRow) -> Result<FileIndexEntry> {
        use crate::handle::FileMetadata;
        use chrono::DateTime;

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

    /// Close the channel database connection
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FileConfig;
    use tempfile::TempDir;

    async fn create_test_managers() -> (ChannelManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();

        // Create FileManager
        let config = FileConfig::with_path(temp_dir.path().join("files"));
        let file_manager = FileManager::new(config).await.unwrap();

        // Create ChannelManager using the SAME database as FileManager
        // FileManager stores index at storage_path.join("index.db")
        let channel_db = temp_dir.path().join("files/index.db");
        let channel_manager = ChannelManager::new(Arc::new(file_manager), channel_db)
            .await
            .unwrap();

        (channel_manager, temp_dir)
    }

    fn create_test_metadata(name: &str) -> FileMetadata {
        FileMetadata {
            name: name.to_string(),
            size: 100,
            mime_type: Some("text/plain".to_string()),
            source: Some("test".to_string()),
            created_at: chrono::Utc::now(),
            last_accessed_at: None,
            preview: None,
        }
    }

    #[tokio::test]
    async fn test_upload_to_channel() {
        let (manager, _temp) = create_test_managers().await;

        let data = b"hello channel";
        let metadata = create_test_metadata("test.txt");

        // Upload to first channel
        let handle = manager
            .upload_to_channel("channel:1", data, metadata.clone(), Some("user1"), None)
            .await
            .unwrap();

        assert!(handle.id.starts_with("sha256:"));

        // List files in channel
        let files = manager.list_channel_files("channel:1").await.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].uploaded_by, Some("user1".to_string()));
    }

    #[tokio::test]
    async fn test_channel_file_deduplication() {
        let (manager, _temp) = create_test_managers().await;

        let data = b"shared content";
        let metadata = create_test_metadata("shared.txt");

        // Upload to two different channels
        let handle1 = manager
            .upload_to_channel("channel:A", data, metadata.clone(), Some("user1"), None)
            .await
            .unwrap();

        let handle2 = manager
            .upload_to_channel("channel:B", data, metadata.clone(), Some("user2"), None)
            .await
            .unwrap();

        // Same content = same file ID
        assert_eq!(handle1.id, handle2.id);

        // But channel associations are different
        let files_a = manager.list_channel_files("channel:A").await.unwrap();
        let files_b = manager.list_channel_files("channel:B").await.unwrap();

        assert_eq!(files_a.len(), 1);
        assert_eq!(files_b.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_from_channel() {
        let (manager, _temp) = create_test_managers().await;

        let data = b"removable content";
        let metadata = create_test_metadata("remove_me.txt");

        let handle = manager
            .upload_to_channel("channel:X", data, metadata, Some("user1"), None)
            .await
            .unwrap();

        // Remove from channel
        let removed = manager
            .remove_from_channel("channel:X", &handle.id)
            .await
            .unwrap();
        assert!(removed);

        // List should be empty
        let files = manager.list_channel_files("channel:X").await.unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_delete_channel_cleanup() {
        let (manager, _temp) = create_test_managers().await;

        let data = b"cleanup content";
        let metadata = create_test_metadata("cleanup.txt");

        let _handle = manager
            .upload_to_channel("cleanup_channel", data, metadata, Some("user1"), None)
            .await
            .unwrap();

        // Delete channel with cleanup=true
        let deleted = manager
            .delete_channel("cleanup_channel", true)
            .await
            .unwrap();
        assert_eq!(deleted, 1);

        // File should be soft-deleted (exists but not in normal list)
        let files = manager.list_channel_files("cleanup_channel").await.unwrap();
        assert!(files.is_empty());
    }

    #[tokio::test]
    async fn test_list_file_channels() {
        let (manager, _temp) = create_test_managers().await;

        let data = b"multi-channel file";
        let metadata = create_test_metadata("multi.txt");

        let handle = manager
            .upload_to_channel("ch:A", data, metadata, Some("user1"), None)
            .await
            .unwrap();

        // Add to another channel
        manager
            .add_file_to_channel("ch:B", &handle.id, Some("user2"), None)
            .await
            .unwrap();

        // List all channels for this file
        let channels = manager.list_file_channels(&handle.id).await.unwrap();
        assert!(channels.contains(&"ch:A".to_string()));
        assert!(channels.contains(&"ch:B".to_string()));
    }

    #[tokio::test]
    async fn test_channel_stats() {
        let (manager, _temp) = create_test_managers().await;

        // Upload multiple files to a channel
        for i in 0..3 {
            let data = format!("content {}", i);
            let metadata = create_test_metadata(&format!("file{}.txt", i));
            manager
                .upload_to_channel(
                    "stats_channel",
                    data.as_bytes(),
                    metadata,
                    Some("user1"),
                    None,
                )
                .await
                .unwrap();
        }

        let stats = manager.channel_stats("stats_channel").await.unwrap();
        assert_eq!(stats.total_files, 3);
    }
}
