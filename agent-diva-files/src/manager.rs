//! FileManager - Core file management system
//!
//! Provides content-addressed storage with deduplication, reference counting,
//! and lazy cleanup. This is the main interface for storing and retrieving files.

use crate::config::{CleanupStrategy, FileConfig};
use crate::handle::{FileHandle, FileIndexEntry, FileMetadata};
use crate::hooks::HookRegistry;
use crate::index::SqliteIndex;
use crate::storage::{compute_hash, FileStorage, StorageStats};
use crate::{FileError, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// File manager - main interface for file operations
pub struct FileManager {
    config: FileConfig,
    storage: FileStorage,
    index: Arc<SqliteIndex>,
    /// Hook registry for extending file operations (storage hooks, read hooks, etc.)
    hooks: HookRegistry,
}

impl FileManager {
    /// Create a new file manager with default (empty) hooks.
    ///
    /// For hooks integration, use [`FileManager::new_with_hooks`] instead.
    pub async fn new(config: FileConfig) -> Result<Self> {
        Self::new_with_hooks(config, HookRegistry::new()).await
    }

    /// Create a new file manager with the given configuration and hooks.
    ///
    /// # Arguments
    /// * `config` - File manager configuration
    /// * `hooks` - Hook registry containing storage, read, metadata, and cleanup hooks
    ///
    /// # Example
    /// ```ignore
    /// use agent_diva_files::{FileManager, FileConfig, hooks::{HookRegistry, LoggingStorageHook}};
    ///
    /// let mut hooks = HookRegistry::new();
    /// hooks.register_storage_hook(Box::new(LoggingStorageHook));
    ///
    /// let manager = FileManager::new_with_hooks(FileConfig::default(), hooks).await?;
    /// ```
    pub async fn new_with_hooks(config: FileConfig, hooks: HookRegistry) -> Result<Self> {
        // Ensure storage directory exists
        tokio::fs::create_dir_all(&config.storage_path).await?;

        // Initialize storage
        let storage = FileStorage::new(config.clone());
        storage.initialize().await?;

        // Load or create index
        let index_path = config.index_path().with_extension("db");
        let index = SqliteIndex::new(index_path).await?;

        // Migrate from old JSONL format if exists
        let jsonl_path = config.index_path();
        if jsonl_path.exists() {
            let migrated = index.migrate_from_jsonl(&jsonl_path).await?;
            if migrated > 0 {
                // Backup old index file
                let backup_path = jsonl_path.with_extension("jsonl.bak");
                tokio::fs::rename(&jsonl_path, &backup_path).await?;
                info!("Migrated {} entries and backed up old index", migrated);
            }
        }

        info!(
            "FileManager initialized with {} storage hooks, {} read hooks",
            hooks.hook_counts().storage,
            hooks.hook_counts().read
        );

        Ok(Self {
            config,
            storage,
            index: Arc::new(index),
            hooks,
        })
    }

    /// Create with default configuration
    pub async fn default() -> Result<Self> {
        Self::new(FileConfig::default()).await
    }

    /// Store file data and return a handle
    ///
    /// If the file already exists (based on hash), returns an existing handle
    /// and increments the reference count.
    ///
    /// # Hook Integration
    /// - Calls `StorageHook::before_store` hooks before storing
    /// - Calls `StorageHook::after_store` hooks after successful storage
    /// - Calls `MetadataHook::extract_metadata` for additional metadata
    pub async fn store(&self, data: &[u8], metadata: FileMetadata) -> Result<FileHandle> {
        // Check file size limit
        let data_size = data.len() as u64;
        if data_size > self.config.max_file_size {
            return Err(FileError::TooLarge(data_size, self.config.max_file_size));
        }

        // ========================================
        // STEP 1: Run before_store hooks
        // ========================================
        // Hooks can modify the data (e.g., compress, encrypt) or reject the storage
        let (processed_data, should_continue) =
            self.hooks.run_before_store(data, &metadata).await?;

        if !should_continue {
            // A hook requested to stop without error (e.g., cache hit)
            return Err(FileError::Storage("Storage stopped by hook".to_string()));
        }

        // Use processed data (might be modified by hooks)
        let final_data = if processed_data != data {
            // Hook modified the data
            debug!(
                "Storage hook modified data: {} -> {} bytes",
                data.len(),
                processed_data.len()
            );
            processed_data
        } else {
            data.to_vec()
        };

        // Re-compute hash from potentially modified data
        let hash = compute_hash(&final_data);
        let hash_id = format!("sha256:{}", hash);

        // ========================================
        // STEP 2: Run metadata validation hooks
        // ========================================
        self.hooks.run_validate_metadata(&metadata).await?;

        // Check if file already exists (don't include deleted files in dedup check)
        if let Some(entry) = self.index.get(&hash_id, false).await? {
            // Clone what we need before modifying index
            let handle = entry.to_handle();
            let new_count = entry.ref_count + 1;

            // File exists, increment ref count
            self.index.update_ref_count(&hash_id, 1).await?;
            debug!(
                "File with hash {} already exists, incrementing ref count to {}",
                hash_id, new_count
            );

            return Ok(handle);
        }

        // ========================================
        // STEP 3: Store file data
        // ========================================
        let relative_path = self.storage.store_data(&hash, &final_data).await?;

        // ========================================
        // STEP 4: Create index entry
        // ========================================
        let entry = FileIndexEntry {
            id: hash_id.clone(),
            path: relative_path.clone(),
            size: final_data.len() as u64,
            ref_count: 1,
            created_at: metadata.created_at,
            last_accessed_at: Some(chrono::Utc::now()),
            metadata: metadata.clone(),
        };

        self.index.insert(entry).await?;

        // Create handle for after_store hooks
        let handle = FileHandle::new(hash_id.clone(), relative_path.clone(), metadata.clone());

        // ========================================
        // STEP 5: Run after_store hooks
        // ========================================
        // Fire-and-forget: after_store hooks shouldn't block the result
        // In production, consider spawning these as detached tasks
        let _ = self.hooks.run_after_store(&handle).await;

        info!(
            "Stored new file {} ({}, {} bytes)",
            metadata.name,
            hash_id,
            final_data.len()
        );

        Ok(handle)
    }

    /// Store file from path
    pub async fn store_from_path(
        &self,
        source_path: &PathBuf,
        metadata: Option<FileMetadata>,
    ) -> Result<FileHandle> {
        let data = tokio::fs::read(source_path).await?;

        let meta = metadata.unwrap_or_else(|| FileMetadata {
            name: source_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            size: data.len() as u64,
            mime_type: None,
            source: None,
            created_at: chrono::Utc::now(),
            last_accessed_at: None,
            preview: None,
        });

        self.store(&data, meta).await
    }

    /// Get a file handle by ID
    ///
    /// Increments the reference count if found.
    /// Note: This only returns non-deleted files.
    pub async fn get(&self, id: &str) -> Result<FileHandle> {
        // Don't include deleted files in normal get operations
        if let Some(entry) = self.index.get(id, false).await? {
            // Clone what we need before modifying index
            let handle = entry.to_handle();

            // Update access time and ref count
            self.index.update_ref_count(id, 0).await?; // Just updates access time

            return Ok(handle);
        }

        Err(FileError::NotFound(format!(
            "File with ID {} not found",
            id
        )))
    }

    /// Clone a file handle (increment reference count)
    pub async fn clone_ref(&self, handle: &FileHandle) -> Result<FileHandle> {
        if let Some(new_count) = self.index.update_ref_count(&handle.id, 1).await? {
            let cloned = handle.clone();
            cloned.increment_ref();

            debug!(
                "Cloned file {} reference, new count: {}",
                handle.id, new_count
            );
            Ok(cloned)
        } else {
            Err(FileError::InvalidHandle(format!(
                "Handle {} not found in index",
                handle.id
            )))
        }
    }

    /// Release a file handle (decrement reference count)
    ///
    /// Does not actually delete the file - cleanup is done separately
    /// based on the cleanup strategy.
    pub async fn release(&self, handle: &FileHandle) -> Result<()> {
        if let Some(new_count) = self.index.update_ref_count(&handle.id, -1).await? {
            debug!(
                "Released file {} reference, new count: {}",
                handle.id, new_count
            );

            // Immediate cleanup if configured
            if self.config.cleanup.strategy == CleanupStrategy::Immediate && new_count == 0 {
                self.cleanup_single(&handle.id).await?;
            }

            Ok(())
        } else {
            Err(FileError::InvalidHandle(format!(
                "Handle {} not found in index",
                handle.id
            )))
        }
    }

    // ==========================================================================
    // Soft Delete Operations (Steam-style)
    // ==========================================================================

    /// Soft delete a file - marks it as deleted but doesn't remove physically
    ///
    /// The file enters a "deleted" state but can be recovered within the retention period.
    /// After `retention_days` expire, the file becomes eligible for permanent deletion
    /// via [`FileManager::purge_expired`].
    ///
    /// # Arguments
    /// * `id` - File ID to delete
    /// * `deleted_by` - Optional identifier of who/what deleted the file
    ///
    /// # Example
    /// ```ignore
    /// // Soft delete a file
    /// manager.soft_delete(&file_id, Some("user@example.com")).await?;
    ///
    /// // List deleted files
    /// let deleted = manager.list_deleted().await?;
    ///
    /// // Restore if needed
    /// manager.restore(&file_id).await?;
    /// ```
    pub async fn soft_delete(&self, id: &str, deleted_by: Option<&str>) -> Result<bool> {
        let deleted = self.index.soft_delete(id, deleted_by).await?;

        if deleted {
            info!("Soft deleted file {} (by {:?})", id, deleted_by);
        } else {
            debug!("File {} not found or already deleted", id);
        }

        Ok(deleted)
    }

    /// Restore a soft-deleted file
    ///
    /// Makes the file accessible again by clearing the deleted timestamp.
    /// Only works on files that were soft-deleted and haven't expired yet.
    ///
    /// # Arguments
    /// * `id` - File ID to restore
    ///
    /// # Returns
    /// * `Ok(true)` - File was restored
    /// * `Ok(false)` - File wasn't found in deleted state
    pub async fn restore(&self, id: &str) -> Result<bool> {
        let restored = self.index.restore(id).await?;

        if restored {
            info!("Restored file {}", id);
        } else {
            debug!("File {} not found or not in deleted state", id);
        }

        Ok(restored)
    }

    /// List all soft-deleted files
    ///
    /// Returns files that have been soft-deleted but not yet purged.
    /// Files are sorted by deletion time (most recent first).
    ///
    /// # Returns
    /// List of deleted file entries with deletion metadata
    pub async fn list_deleted(&self) -> Result<Vec<FileIndexEntry>> {
        let entries = self.index.list_deleted().await?;
        debug!("Found {} soft-deleted files", entries.len());
        Ok(entries)
    }

    /// Permanently delete a specific file (bypass retention period)
    ///
    /// WARNING: This immediately and permanently removes the file.
    /// Unlike soft delete, there is no way to recover a hard-deleted file.
    ///
    /// # Arguments
    /// * `id` - File ID to permanently delete
    ///
    /// # Returns
    /// The deleted entry (for logging/audit purposes), or None if not found
    pub async fn hard_delete(&self, id: &str) -> Result<Option<FileIndexEntry>> {
        // First check if the file exists (maybe already deleted)
        let entry = self.index.get(id, true).await?;

        match entry {
            Some(e) => {
                // Delete the physical file
                if let Err(e) = self.storage.delete_data(&e.path).await {
                    warn!("Failed to delete physical file {}: {}", id, e);
                    // Continue anyway - we'll remove from index
                }

                // Remove from index entirely
                self.index.hard_delete(id).await?;
                info!(
                    "Hard deleted file {} (was deleted_at={:?})",
                    id, e.metadata.last_accessed_at
                );

                Ok(Some(e))
            }
            None => {
                debug!("File {} not found for hard delete", id);
                Ok(None)
            }
        }
    }

    /// Purge all soft-deleted files that have exceeded the retention period
    ///
    /// This is the cleanup task for soft deletes. It finds all soft-deleted files
    /// where `deleted_at` is older than `retention_days` and permanently removes them.
    ///
    /// # Arguments
    /// * `retention_days` - Files deleted more than this many days ago will be purged
    ///
    /// # Returns
    /// Number of files permanently deleted
    pub async fn purge_expired(&self, retention_days: u32) -> Result<usize> {
        let expired = self.index.get_expired_deletions(retention_days).await?;

        let mut purged = 0;
        for entry in expired {
            // Delete the physical file
            if let Err(e) = self.storage.delete_data(&entry.path).await {
                warn!("Failed to delete expired file {}: {}", entry.id, e);
                continue;
            }

            // Remove from index
            self.index.hard_delete(&entry.id).await?;
            purged += 1;

            info!(
                "Purged expired file {} (deleted at {:?})",
                entry.id, entry.last_accessed_at
            );
        }

        if purged > 0 {
            info!(
                "Purge completed: {} expired files permanently deleted",
                purged
            );
        }

        Ok(purged)
    }

    /// Check if a file is soft-deleted
    ///
    /// # Arguments
    /// * `id` - File ID to check
    ///
    /// # Returns
    /// `true` if the file is soft-deleted, `false` otherwise
    pub async fn is_deleted(&self, id: &str) -> Result<bool> {
        // Get with include_deleted=true to see if it exists
        let entry = self.index.get(id, true).await?;

        match entry {
            Some(_) => {
                // Now check if it has a deleted_at by looking at the raw entry
                // We need to query specifically for deleted status
                let deleted_entries = self.index.list_deleted().await?;
                Ok(deleted_entries.iter().any(|e| e.id == id))
            }
            None => Ok(false),
        }
    }

    /// Read file data by handle
    ///
    /// # Hook Integration
    /// - Calls `ReadHook::before_read` hooks before reading (e.g., permission check)
    /// - Calls `ReadHook::after_read` hooks after reading (e.g., decryption)
    pub async fn read(&self, handle: &FileHandle) -> Result<Vec<u8>> {
        // ========================================
        // STEP 1: Run before_read hooks
        // ========================================
        // Hooks can reject the read (e.g., permission denied)
        let should_read = self.hooks.run_before_read(&handle.id, None).await?;

        if !should_read {
            return Err(FileError::NotFound(format!(
                "File {} read was blocked by hook",
                handle.id
            )));
        }

        // ========================================
        // STEP 2: Read file data from storage
        // ========================================
        let data = self.storage.read_data(&handle.path).await?;

        // ========================================
        // STEP 3: Run after_read hooks
        // ========================================
        // Hooks can transform the data (e.g., decrypt)
        let processed_data = self.hooks.run_after_read(&data).await?;

        Ok(processed_data)
    }

    /// Read file as string (for text files)
    pub async fn read_string(&self, handle: &FileHandle) -> Result<String> {
        let data = self.read(handle).await?;
        String::from_utf8(data).map_err(|e| FileError::Storage(format!("Invalid UTF-8: {}", e)))
    }

    /// Get the full path for a handle
    pub fn full_path(&self, handle: &FileHandle) -> PathBuf {
        handle.full_path(&self.config.data_dir())
    }

    /// Check if a file exists (non-deleted)
    pub async fn exists(&self, id: &str) -> bool {
        self.index.get(id, false).await.ok().flatten().is_some()
    }

    /// Get file metadata
    ///
    /// Returns metadata for non-deleted files only.
    pub async fn metadata(&self, id: &str) -> Result<FileMetadata> {
        self.index
            .get(id, false)
            .await?
            .map(|e| e.metadata)
            .ok_or_else(|| FileError::NotFound(format!("File {} not found", id)))
    }

    /// Run cleanup - delete files with ref_count <= threshold
    ///
    /// Returns the number of files deleted.
    ///
    /// # Hook Integration
    /// - Calls `CleanupHook::should_cleanup` for each candidate
    /// - Calls `CleanupHook::after_cleanup` after each file is deleted
    pub async fn cleanup(&self) -> Result<usize> {
        let threshold = self.config.cleanup.min_ref_count;
        let max_age_days = self.config.cleanup.max_age_days;

        let candidates = self
            .index
            .get_candidates_for_cleanup(threshold, max_age_days)
            .await?;

        let mut deleted = 0;

        // Get list of soft-deleted file IDs to exclude from regular cleanup
        // Soft-deleted files are handled by purge_expired(), not regular cleanup
        let deleted_entries = self.index.list_deleted().await?;
        let deleted_ids: std::collections::HashSet<_> =
            deleted_entries.iter().map(|e| e.id.clone()).collect();

        for entry in candidates {
            // Skip soft-deleted files - they're handled by purge_expired()
            if deleted_ids.contains(&entry.id) {
                debug!("Skipping soft-deleted file {} in regular cleanup", entry.id);
                continue;
            }

            // ========================================
            // STEP 1: Run should_cleanup hooks
            // ========================================
            // Hooks can veto the deletion (e.g., file is protected)
            let can_delete = match self.hooks.run_should_cleanup(&entry).await {
                Ok(should) => should,
                Err(e) => {
                    warn!("Cleanup hook error for {}: {}", entry.id, e);
                    continue;
                }
            };

            if !can_delete {
                debug!("Cleanup of {} was blocked by hook", entry.id);
                continue;
            }

            // ========================================
            // STEP 2: Delete file data
            // ========================================
            if let Err(e) = self.storage.delete_data(&entry.path).await {
                warn!("Failed to delete file {}: {}", entry.id, e);
                continue;
            }

            // ========================================
            // STEP 3: Remove from index
            // ========================================
            if let Err(e) = self.index.remove(&entry.id).await {
                warn!("Failed to remove {} from index: {}", entry.id, e);
                // Continue anyway - file data is already deleted
            }

            // ========================================
            // STEP 4: Run after_cleanup hooks
            // ========================================
            // Fire-and-forget: after_cleanup hooks shouldn't block the deletion
            let _ = self.hooks.run_after_cleanup(&entry).await;

            deleted += 1;
            info!("Cleaned up file {}", entry.id);
        }

        if deleted > 0 {
            info!("Cleanup completed: {} files deleted", deleted);
        }

        Ok(deleted)
    }

    /// Cleanup a single file by ID
    ///
    /// Used by immediate cleanup strategy when ref_count reaches 0.
    async fn cleanup_single(&self, id: &str) -> Result<()> {
        // Get the entry (include deleted=false since we only clean up non-deleted files)
        if let Some(entry) = self.index.get(id, false).await? {
            if entry.ref_count == 0 {
                self.storage.delete_data(&entry.path).await?;
                self.index.remove(id).await?;
                info!("Immediately cleaned up file {}", id);
            }
        } else {
            return Err(FileError::NotFound(format!("File {} not found", id)));
        }

        Ok(())
    }

    /// Get storage statistics
    pub async fn stats(&self) -> Result<StorageStats> {
        let mut stats = self.storage.stats().await?;

        // Get stats from the index
        let index_stats = self.index.stats().await?;
        stats.total_refs = index_stats.total_refs;

        Ok(stats)
    }

    /// Get a reference to the config
    pub fn config(&self) -> &FileConfig {
        &self.config
    }

    /// Get a reference to the hooks registry
    ///
    /// Useful for inspecting registered hooks or adding hooks dynamically.
    pub fn hooks(&self) -> &HookRegistry {
        &self.hooks
    }

    /// Get mutable reference to the hooks registry
    ///
    /// Allows adding new hooks at runtime.
    pub fn hooks_mut(&mut self) -> &mut HookRegistry {
        &mut self.hooks
    }

    /// Start background cleanup task
    ///
    /// This spawns a task that periodically runs cleanup
    pub fn start_cleanup_task(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let interval_secs = self.config.cleanup.interval_secs;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                match self.cleanup().await {
                    Ok(count) => {
                        if count > 0 {
                            info!("Background cleanup removed {} files", count);
                        }
                    }
                    Err(e) => {
                        warn!("Background cleanup failed: {}", e);
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_manager() -> (FileManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = FileConfig::with_path(temp_dir.path().to_path_buf());
        let manager = FileManager::new(config).await.unwrap();
        (manager, temp_dir)
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
    async fn test_store_and_get() {
        let (manager, _temp) = create_test_manager().await;

        let data = b"hello world";
        let metadata = create_test_metadata("test.txt");

        let handle = manager.store(data, metadata.clone()).await.unwrap();
        assert!(handle.id.starts_with("sha256:"));
        assert_eq!(handle.metadata.name, "test.txt");

        // Get the file back
        let handle2 = manager.get(&handle.id).await.unwrap();
        assert_eq!(handle2.id, handle.id);

        // Read data
        let read_data = manager.read(&handle2).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let (manager, _temp) = create_test_manager().await;

        let data = b"test content for dedup";
        let metadata1 = create_test_metadata("file1.txt");
        let metadata2 = create_test_metadata("file2.txt");

        let handle1 = manager.store(data, metadata1).await.unwrap();
        let handle2 = manager.store(data, metadata2).await.unwrap();

        // Should have same ID (same content)
        assert_eq!(handle1.id, handle2.id);

        // Check ref count in index
        let stats = manager.stats().await.unwrap();
        assert_eq!(stats.total_files, 1);
        assert_eq!(stats.total_refs, 2);
    }

    #[tokio::test]
    async fn test_reference_counting() {
        let (manager, _temp) = create_test_manager().await;

        let data = b"ref counting test";
        let metadata = create_test_metadata("test.txt");

        let handle = manager.store(data, metadata).await.unwrap();

        // Clone reference
        let cloned = manager.clone_ref(&handle).await.unwrap();
        assert_eq!(cloned.ref_count(), 2);

        // Release reference
        manager.release(&cloned).await.unwrap();

        // Check index was updated
        let stats = manager.stats().await.unwrap();
        assert_eq!(stats.total_refs, 1);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = FileConfig::with_path(temp_dir.path().to_path_buf());
        config.cleanup.max_age_days = 0; // Allow immediate cleanup

        let manager = FileManager::new(config).await.unwrap();

        let data = b"cleanup test";
        let metadata = create_test_metadata("cleanup.txt");

        let handle = manager.store(data, metadata).await.unwrap();

        // Release the reference
        manager.release(&handle).await.unwrap();

        // Cleanup should remove the file (ref_count = 0)
        let deleted = manager.cleanup().await.unwrap();
        assert_eq!(deleted, 1);

        // File should no longer exist
        assert!(!manager.exists(&handle.id).await);
    }

    #[tokio::test]
    async fn test_exists() {
        let (manager, _temp) = create_test_manager().await;

        let data = b"exists test";
        let metadata = create_test_metadata("exists.txt");

        let handle = manager.store(data, metadata).await.unwrap();

        assert!(manager.exists(&handle.id).await);
        assert!(!manager.exists("sha256:nonexistent").await);
    }

    #[tokio::test]
    async fn test_file_too_large() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = FileConfig::with_path(temp_dir.path().to_path_buf());
        config.max_file_size = 10; // 10 bytes limit

        let manager = FileManager::new(config).await.unwrap();

        let data = b"this is too large";
        let metadata = create_test_metadata("large.txt");

        let result = manager.store(data, metadata).await;
        assert!(matches!(result, Err(FileError::TooLarge(_, _))));
    }
}
