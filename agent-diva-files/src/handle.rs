//! File handle with reference counting
//!
//! FileHandle provides a safe reference to a stored file.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// A handle to a stored file
///
/// FileHandle tracks the file ID, path, and metadata.
/// Note: Reference counting is managed by FileIndex, not FileHandle.
/// FileHandle's ref_count is for informational purposes only.
pub struct FileHandle {
    /// Unique file ID (SHA256 hash)
    pub id: String,

    /// Storage path relative to the data directory
    pub path: PathBuf,

    /// File metadata
    pub metadata: FileMetadata,

    /// Local reference count view (for debugging/information)
    pub ref_count: Arc<AtomicUsize>,
}

impl Clone for FileHandle {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            path: self.path.clone(),
            metadata: self.metadata.clone(),
            ref_count: Arc::new(AtomicUsize::new(1)),
        }
    }
}

impl std::fmt::Debug for FileHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileHandle")
            .field("id", &self.id)
            .field("path", &self.path)
            .field("metadata", &self.metadata)
            .field("ref_count", &self.ref_count.load(Ordering::SeqCst))
            .finish()
    }
}

impl FileHandle {
    /// Create a new file handle
    pub fn new(id: String, path: PathBuf, metadata: FileMetadata) -> Self {
        Self {
            id,
            path,
            metadata,
            ref_count: Arc::new(AtomicUsize::new(1)),
        }
    }

    /// Create from index entry with specific ref count
    pub fn with_ref_count(id: String, path: PathBuf, metadata: FileMetadata, count: usize) -> Self {
        Self {
            id,
            path,
            metadata,
            ref_count: Arc::new(AtomicUsize::new(count)),
        }
    }

    /// Get the current reference count
    pub fn ref_count(&self) -> usize {
        self.ref_count.load(Ordering::SeqCst)
    }

    /// Increment reference count (returns new count)
    pub fn increment_ref(&self) -> usize {
        self.ref_count.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrement reference count (returns new count)
    pub fn decrement_ref(&self) -> usize {
        let prev = self.ref_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 0 {
            self.ref_count.store(0, Ordering::SeqCst);
            0
        } else {
            prev - 1
        }
    }

    /// Check if this is the last reference
    pub fn is_last_ref(&self) -> bool {
        self.ref_count() <= 1
    }

    /// Get the full path given a base directory
    pub fn full_path(&self, base_dir: &Path) -> PathBuf {
        base_dir.join(&self.path)
    }

    /// Update last accessed timestamp
    pub fn touch(&mut self) {
        self.metadata.last_accessed_at = Some(chrono::Utc::now());
    }

    /// Get file extension (if any)
    pub fn extension(&self) -> Option<&str> {
        std::path::Path::new(&self.metadata.name)
            .extension()
            .and_then(|e| e.to_str())
    }

    /// Check if file is an image
    pub fn is_image(&self) -> bool {
        matches!(
            self.metadata.mime_type.as_deref(),
            Some("image/jpeg")
                | Some("image/png")
                | Some("image/gif")
                | Some("image/webp")
                | Some("image/svg+xml")
        )
    }

    /// Check if file is a text file
    pub fn is_text(&self) -> bool {
        matches!(
            self.metadata.mime_type.as_deref(),
            Some("text/plain")
                | Some("text/markdown")
                | Some("text/html")
                | Some("application/json")
                | Some("application/xml")
                | Some("text/csv")
        ) || self
            .extension()
            .map(|e| {
                matches!(
                    e.to_lowercase().as_str(),
                    "txt" | "md" | "markdown" | "json" | "xml" | "csv" | "rs" | "py" | "js" | "ts"
                )
            })
            .unwrap_or(false)
    }
}

/// File metadata stored with the handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Original filename
    pub name: String,

    /// File size in bytes
    pub size: u64,

    /// MIME type (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Source channel (telegram, discord, ui, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last access timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_accessed_at: Option<chrono::DateTime<chrono::Utc>>,

    /// Optional preview/content for small files
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

/// Index entry for persistent storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileIndexEntry {
    pub id: String,
    pub path: PathBuf,
    pub size: u64,
    pub ref_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_accessed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: FileMetadata,
}

impl FileIndexEntry {
    /// Convert to FileHandle (with index's ref_count)
    pub fn to_handle(&self) -> FileHandle {
        FileHandle::with_ref_count(
            self.id.clone(),
            self.path.clone(),
            self.metadata.clone(),
            self.ref_count,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metadata() -> FileMetadata {
        FileMetadata {
            name: "test.txt".to_string(),
            size: 100,
            mime_type: Some("text/plain".to_string()),
            source: Some("test".to_string()),
            created_at: chrono::Utc::now(),
            last_accessed_at: None,
            preview: None,
        }
    }

    #[test]
    fn test_ref_counting() {
        let handle = FileHandle::new(
            "abc123".to_string(),
            PathBuf::from("data/ab/c123"),
            create_test_metadata(),
        );

        assert_eq!(handle.ref_count(), 1);

        // Clone creates independent handle with its own ref_count
        let cloned = handle.clone();
        assert_eq!(handle.ref_count(), 1); // Original unchanged
        assert_eq!(cloned.ref_count(), 1); // Clone starts at 1

        // Increment cloned
        cloned.increment_ref();
        assert_eq!(cloned.ref_count(), 2);
        assert_eq!(handle.ref_count(), 1); // Original still unchanged
    }

    #[test]
    fn test_is_last_ref() {
        let handle = FileHandle::new(
            "abc123".to_string(),
            PathBuf::from("data/ab/c123"),
            create_test_metadata(),
        );

        assert!(handle.is_last_ref());

        handle.increment_ref();
        assert!(!handle.is_last_ref());

        handle.decrement_ref();
        assert!(handle.is_last_ref());
    }

    #[test]
    fn test_is_text() {
        let metadata = FileMetadata {
            name: "test.txt".to_string(),
            size: 100,
            mime_type: Some("text/plain".to_string()),
            source: None,
            created_at: chrono::Utc::now(),
            last_accessed_at: None,
            preview: None,
        };

        let handle = FileHandle::new("id".to_string(), PathBuf::from("path"), metadata);
        assert!(handle.is_text());
    }

    #[test]
    fn test_is_image() {
        let metadata = FileMetadata {
            name: "test.png".to_string(),
            size: 100,
            mime_type: Some("image/png".to_string()),
            source: None,
            created_at: chrono::Utc::now(),
            last_accessed_at: None,
            preview: None,
        };

        let handle = FileHandle::new("id".to_string(), PathBuf::from("path"), metadata);
        assert!(handle.is_image());
    }
}
