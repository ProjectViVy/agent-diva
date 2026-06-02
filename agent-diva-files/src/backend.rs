//! Storage backend trait and implementations
//!
//! This module provides the `StorageBackend` trait for pluggable storage backends.
//! The default implementation is `LocalStorageBackend` which uses the local filesystem.
//!
//! # Example
//! ```rust,no_run
//! use agent_diva_files::backend::{StorageBackend, LocalStorageBackend};
//! use std::path::PathBuf;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let backend = LocalStorageBackend::new(PathBuf::from("./data"));
//! backend.initialize().await?;
//!
//! let data = b"hello world";
//! let path = backend.write("abc123", data).await?;
//! let read_data = backend.read(&path).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use std::path::{Path, PathBuf};

use crate::{FileError, Result};

/// Trait for storage backends
///
/// Implement this trait to add support for different storage backends
/// such as S3, Azure Blob, Google Cloud Storage, etc.
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Initialize the storage backend
    ///
    /// This should create necessary directories, buckets, or connections.
    async fn initialize(&self) -> Result<()>;

    /// Write data to storage and return a relative path/identifier
    ///
    /// # Arguments
    /// * `key` - A unique identifier for the data (typically a hash)
    /// * `data` - The data to store
    ///
    /// # Returns
    /// A relative path or identifier that can be used to read the data later
    async fn write(&self, key: &str, data: &[u8]) -> Result<PathBuf>;

    /// Read data from storage
    ///
    /// # Arguments
    /// * `path` - The relative path returned by `write`
    ///
    /// # Returns
    /// The stored data as a byte vector
    async fn read(&self, path: &Path) -> Result<Vec<u8>>;

    /// Delete data from storage
    ///
    /// # Arguments
    /// * `path` - The relative path returned by `write`
    async fn delete(&self, path: &Path) -> Result<()>;

    /// Check if data exists
    ///
    /// # Arguments
    /// * `key` - The unique identifier for the data
    async fn exists(&self, key: &str) -> bool;

    /// Get the full path/URI for a relative path
    ///
    /// This is used to construct absolute paths for local files
    /// or URIs for remote storage.
    fn full_path(&self, relative_path: &Path) -> PathBuf;

    /// Get storage statistics
    ///
    /// Returns information about the storage usage.
    async fn stats(&self) -> Result<BackendStats>;
}

/// Statistics for a storage backend
#[derive(Debug, Clone, Default)]
pub struct BackendStats {
    /// Total number of stored objects
    pub total_objects: usize,
    /// Total size in bytes
    pub total_size: u64,
    /// Available space (if applicable)
    pub available_space: Option<u64>,
}

/// Local filesystem storage backend
///
/// This is the default storage backend that stores files on the local filesystem
/// using a content-addressed structure based on hash prefixes.
pub struct LocalStorageBackend {
    /// Base directory for storage
    data_dir: PathBuf,
}

impl LocalStorageBackend {
    /// Create a new local storage backend
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    /// Get the data directory
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Convert a hash to a storage path
    ///
    /// Example: "abcdef123..." -> "ab/cdef123..."
    fn hash_to_path(&self, hash: &str) -> PathBuf {
        if hash.len() < 4 {
            return PathBuf::from(hash);
        }
        let prefix = &hash[0..2];
        let rest = &hash[2..];
        PathBuf::from(prefix).join(rest)
    }
}

#[async_trait]
impl StorageBackend for LocalStorageBackend {
    async fn initialize(&self) -> Result<()> {
        use tokio::fs;

        // Create data directory
        fs::create_dir_all(&self.data_dir).await?;

        // Create hash-based subdirectories (aa, ab, ac, ..., ff)
        // This prevents having too many files in a single directory
        for first in 'a'..='f' {
            for second in '0'..='9' {
                let subdir = self.data_dir.join(format!("{}{}", first, second));
                fs::create_dir_all(&subdir).await?;
            }
            for second in 'a'..='f' {
                let subdir = self.data_dir.join(format!("{}{}", first, second));
                fs::create_dir_all(&subdir).await?;
            }
        }

        tracing::info!("Local storage backend initialized at {:?}", self.data_dir);
        Ok(())
    }

    async fn write(&self, key: &str, data: &[u8]) -> Result<PathBuf> {
        use tokio::fs;

        let relative_path = self.hash_to_path(key);
        let full_path = self.data_dir.join(&relative_path);

        // Check if file already exists (deduplication)
        if full_path.exists() {
            tracing::debug!("File with key {} already exists, skipping write", key);
            return Ok(relative_path);
        }

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Write file atomically using temp file + rename
        let temp_path = full_path.with_extension("tmp");
        fs::write(&temp_path, data).await?;

        // Atomic rename
        fs::rename(&temp_path, &full_path).await?;

        tracing::debug!("Stored file with key {} at {:?}", key, full_path);
        Ok(relative_path)
    }

    async fn read(&self, relative_path: &Path) -> Result<Vec<u8>> {
        use tokio::fs;

        let full_path = self.data_dir.join(relative_path);

        if !full_path.exists() {
            return Err(FileError::NotFound(format!(
                "File not found at {:?}",
                full_path
            )));
        }

        Ok(fs::read(&full_path).await?)
    }

    async fn delete(&self, relative_path: &Path) -> Result<()> {
        use tokio::fs;

        let full_path = self.data_dir.join(relative_path);

        if full_path.exists() {
            fs::remove_file(&full_path).await?;
            tracing::debug!("Deleted file at {:?}", full_path);
        }

        Ok(())
    }

    async fn exists(&self, key: &str) -> bool {
        let path = self.data_dir.join(self.hash_to_path(key));
        path.exists()
    }

    fn full_path(&self, relative_path: &Path) -> PathBuf {
        self.data_dir.join(relative_path)
    }

    async fn stats(&self) -> Result<BackendStats> {
        let mut stats = BackendStats::default();

        // Walk the directory and count files
        let mut entries = tokio::fs::read_dir(&self.data_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                // Count files in subdirectories
                let mut sub_entries = tokio::fs::read_dir(&path).await?;
                while let Some(sub_entry) = sub_entries.next_entry().await? {
                    let sub_path = sub_entry.path();
                    if sub_path.is_file() {
                        stats.total_objects += 1;
                        if let Ok(metadata) = sub_entry.metadata().await {
                            stats.total_size += metadata.len();
                        }
                    }
                }
            } else if path.is_file() {
                stats.total_objects += 1;
                if let Ok(metadata) = entry.metadata().await {
                    stats.total_size += metadata.len();
                }
            }
        }

        Ok(stats)
    }
}

/// S3-compatible storage backend (placeholder for future implementation)
///
/// This is a stub that shows how to implement a remote storage backend.
/// Uncomment and implement when needed.
/*
pub struct S3StorageBackend {
    bucket: String,
    prefix: String,
    client: aws_sdk_s3::Client,
}

#[async_trait]
impl StorageBackend for S3StorageBackend {
    async fn initialize(&self) -> Result<()> {
        // Ensure bucket exists or create it
        todo!("Implement S3 initialization")
    }

    async fn write(&self, key: &str, data: &[u8]) -> Result<PathBuf> {
        // Upload to S3
        let path = PathBuf::from(format!("{}/{}", self.prefix, key));
        todo!("Implement S3 upload")
    }

    async fn read(&self, path: &Path) -> Result<Vec<u8>> {
        // Download from S3
        todo!("Implement S3 download")
    }

    async fn delete(&self, path: &Path) -> Result<()> {
        // Delete from S3
        todo!("Implement S3 delete")
    }

    async fn exists(&self, key: &str) -> bool {
        // Check S3 head object
        todo!("Implement S3 exists check")
    }

    fn full_path(&self, relative_path: &Path) -> PathBuf {
        // Return S3 URI
        PathBuf::from(format!("s3://{}/{}", self.bucket, relative_path.display()))
    }

    async fn stats(&self) -> Result<BackendStats> {
        // Get S3 bucket stats
        todo!("Implement S3 stats")
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_local_storage_backend() {
        let temp_dir = TempDir::new().unwrap();
        let backend = LocalStorageBackend::new(temp_dir.path().to_path_buf());

        // Initialize
        backend.initialize().await.unwrap();

        // Write
        let data = b"test content";
        let path = backend.write("abc123", data).await.unwrap();
        assert_eq!(path, PathBuf::from("ab/c123"));

        // Exists
        assert!(backend.exists("abc123").await);
        assert!(!backend.exists("xyz789").await);

        // Read
        let read_data = backend.read(&path).await.unwrap();
        assert_eq!(read_data, data);

        // Full path
        let full = backend.full_path(&path);
        assert!(full.to_string_lossy().contains("ab"));
        assert!(full.to_string_lossy().contains("c123"));

        // Delete
        backend.delete(&path).await.unwrap();
        assert!(!backend.exists("abc123").await);
    }

    #[tokio::test]
    async fn test_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let backend = LocalStorageBackend::new(temp_dir.path().to_path_buf());
        backend.initialize().await.unwrap();

        let data = b"duplicate content";

        // Write twice with same key
        let path1 = backend.write("same_key", data).await.unwrap();
        let path2 = backend.write("same_key", data).await.unwrap();

        // Should return same path
        assert_eq!(path1, path2);

        // Should only have one file
        let stats = backend.stats().await.unwrap();
        assert_eq!(stats.total_objects, 1);
    }

    #[test]
    fn test_hash_to_path() {
        let backend = LocalStorageBackend::new(PathBuf::from("/tmp"));

        assert_eq!(
            backend.hash_to_path("abcdef123456"),
            PathBuf::from("ab/cdef123456")
        );
        assert_eq!(backend.hash_to_path("ab"), PathBuf::from("ab"));
        assert_eq!(backend.hash_to_path("a"), PathBuf::from("a"));
        assert_eq!(backend.hash_to_path(""), PathBuf::from(""));
    }
}
