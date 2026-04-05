//! Storage backend for file management
//!
//! Provides content-addressed storage using SHA256 hashes.
//! Files are stored in a directory structure based on hash prefixes.
//!
//! This module is now backed by the `StorageBackend` trait from `backend.rs`.
//! The default implementation uses `LocalStorageBackend`.

use crate::backend::{LocalStorageBackend, StorageBackend};
use crate::config::FileConfig;
use crate::Result;
use std::path::{Path, PathBuf};

/// File storage using a pluggable backend
///
/// This struct wraps a `StorageBackend` implementation and provides
/// higher-level operations like index management.
pub struct FileStorage {
    backend: Box<dyn StorageBackend>,
    #[allow(dead_code)]
    config: FileConfig,
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub total_files: usize,
    pub total_size: u64,
    pub total_refs: usize,
}

impl FileStorage {
    /// Create a new file storage with the default local backend
    pub fn new(config: FileConfig) -> Self {
        let data_dir = config.data_dir();
        let backend = Box::new(LocalStorageBackend::new(data_dir));
        Self { backend, config }
    }

    /// Create a file storage with a custom backend
    ///
    /// This allows using different storage backends like S3, Azure Blob, etc.
    ///
    /// # Example
    /// ```rust
    /// use agent_diva_files::{FileStorage, FileConfig};
    /// use agent_diva_files::backend::LocalStorageBackend;
    /// use std::path::PathBuf;
    ///
    /// # fn example() {
    /// let config = FileConfig::default();
    /// let backend = LocalStorageBackend::new(PathBuf::from("/custom/path"));
    /// let storage = FileStorage::with_backend(config, Box::new(backend));
    /// # }
    /// ```
    pub fn with_backend(config: FileConfig, backend: Box<dyn StorageBackend>) -> Self {
        Self { backend, config }
    }

    /// Initialize the storage backend
    pub async fn initialize(&self) -> Result<()> {
        self.backend.initialize().await
    }

    /// Store file data and return the relative path
    ///
    /// The hash is used as the key for content-addressed storage.
    pub async fn store_data(&self, hash: &str, data: &[u8]) -> Result<PathBuf> {
        self.backend.write(hash, data).await
    }

    /// Read file data by relative path
    pub async fn read_data(&self, relative_path: &Path) -> Result<Vec<u8>> {
        self.backend.read(relative_path).await
    }

    /// Delete file data by relative path
    pub async fn delete_data(&self, relative_path: &Path) -> Result<()> {
        self.backend.delete(relative_path).await
    }

    /// Check if a file exists by hash
    pub async fn exists(&self, hash: &str) -> bool {
        self.backend.exists(hash).await
    }

    /// Get the full path for a relative path
    pub fn full_path(&self, relative_path: &Path) -> PathBuf {
        self.backend.full_path(relative_path)
    }

    /// Get storage statistics
    pub async fn stats(&self) -> Result<StorageStats> {
        let backend_stats = self.backend.stats().await?;
        Ok(StorageStats {
            total_files: backend_stats.total_objects,
            total_size: backend_stats.total_size,
            total_refs: 0, // This is populated from the index, not the backend
        })
    }

    /// Get a reference to the underlying backend
    pub fn backend(&self) -> &dyn StorageBackend {
        self.backend.as_ref()
    }
}

/// Convert hash to storage path
///
/// Example: "abcdef123..." -> "data/ab/cdef123..."
pub fn hash_to_path(hash: &str) -> PathBuf {
    if hash.len() < 4 {
        return PathBuf::from(hash);
    }

    let prefix = &hash[0..2];
    let rest = &hash[2..];
    PathBuf::from(prefix).join(rest)
}

/// Compute SHA256 hash of data
pub fn compute_hash(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hash_to_path() {
        assert_eq!(hash_to_path("abcdef123456"), PathBuf::from("ab/cdef123456"));
        assert_eq!(hash_to_path("ab"), PathBuf::from("ab"));
        assert_eq!(hash_to_path("a"), PathBuf::from("a"));
    }

    #[test]
    fn test_compute_hash() {
        let data = b"hello world";
        let hash = compute_hash(data);
        assert_eq!(hash.len(), 64); // SHA256 is 64 hex chars
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_file_storage_with_backend() {
        let temp_dir = TempDir::new().unwrap();
        let config = FileConfig::with_path(temp_dir.path().to_path_buf());
        let storage = FileStorage::new(config);

        storage.initialize().await.unwrap();

        let data = b"test content";
        let hash = compute_hash(data);
        let path = storage.store_data(&hash, data).await.unwrap();

        assert!(storage.exists(&hash).await);

        let read_data = storage.read_data(&path).await.unwrap();
        assert_eq!(read_data, data);
    }

    #[tokio::test]
    async fn test_file_storage_with_custom_backend() {
        let temp_dir = TempDir::new().unwrap();
        let config = FileConfig::with_path(temp_dir.path().to_path_buf());

        // Create custom backend
        let custom_backend = LocalStorageBackend::new(temp_dir.path().join("custom"));
        let storage = FileStorage::with_backend(config, Box::new(custom_backend));

        storage.initialize().await.unwrap();

        let data = b"custom backend test";
        let hash = compute_hash(data);
        let path = storage.store_data(&hash, data).await.unwrap();

        assert!(storage.exists(&hash).await);

        let read_data = storage.read_data(&path).await.unwrap();
        assert_eq!(read_data, data);
    }
}
