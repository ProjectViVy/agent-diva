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

/// S3-compatible storage backend.
///
/// This backend is exposed as an explicit unsupported stub until S3 support is
/// fully implemented. It must not panic when selected by configuration.
pub struct S3StorageBackend {
    bucket: String,
    prefix: String,
}

impl S3StorageBackend {
    /// Create a new S3 backend stub.
    pub fn new(bucket: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            prefix: prefix.into(),
        }
    }

    fn unsupported_error(&self, operation: &str) -> FileError {
        FileError::UnsupportedBackend(format!(
            "S3 backend is not implemented; cannot {} bucket '{}' with prefix '{}'",
            operation, self.bucket, self.prefix
        ))
    }
}

#[async_trait]
impl StorageBackend for S3StorageBackend {
    async fn initialize(&self) -> Result<()> {
        Err(self.unsupported_error("initialize"))
    }

    async fn write(&self, _key: &str, _data: &[u8]) -> Result<PathBuf> {
        Err(self.unsupported_error("write to"))
    }

    async fn read(&self, _path: &Path) -> Result<Vec<u8>> {
        Err(self.unsupported_error("read from"))
    }

    async fn delete(&self, _path: &Path) -> Result<()> {
        Err(self.unsupported_error("delete from"))
    }

    async fn exists(&self, _key: &str) -> bool {
        tracing::warn!(
            "S3 backend is not implemented; treating object existence as false for bucket '{}'",
            self.bucket
        );
        false
    }

    fn full_path(&self, relative_path: &Path) -> PathBuf {
        let key = if self.prefix.is_empty() {
            relative_path.display().to_string()
        } else {
            format!("{}/{}", self.prefix, relative_path.display())
        };
        PathBuf::from(format!("s3://{}/{}", self.bucket, key))
    }

    async fn stats(&self) -> Result<BackendStats> {
        Err(self.unsupported_error("inspect stats for"))
    }
}

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

    #[tokio::test]
    async fn test_s3_backend_returns_unsupported_errors() {
        let backend = S3StorageBackend::new("bucket", "prefix");

        let err = backend
            .initialize()
            .await
            .expect_err("S3 initialize should be unsupported");
        assert!(matches!(err, FileError::UnsupportedBackend(_)));

        let err = backend
            .write("abc123", b"data")
            .await
            .expect_err("S3 write should be unsupported");
        assert!(matches!(err, FileError::UnsupportedBackend(_)));

        let err = backend
            .read(Path::new("prefix/abc123"))
            .await
            .expect_err("S3 read should be unsupported");
        assert!(matches!(err, FileError::UnsupportedBackend(_)));

        let err = backend
            .delete(Path::new("prefix/abc123"))
            .await
            .expect_err("S3 delete should be unsupported");
        assert!(matches!(err, FileError::UnsupportedBackend(_)));

        assert!(!backend.exists("abc123").await);

        let err = backend
            .stats()
            .await
            .expect_err("S3 stats should be unsupported");
        assert!(matches!(err, FileError::UnsupportedBackend(_)));
    }

    #[test]
    fn test_s3_backend_full_path_includes_prefix() {
        let backend = S3StorageBackend::new("bucket", "prefix");
        assert_eq!(
            backend.full_path(Path::new("ab/c123")),
            PathBuf::from("s3://bucket/prefix/ab/c123")
        );
    }
}
