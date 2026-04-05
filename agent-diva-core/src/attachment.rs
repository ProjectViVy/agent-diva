//! File attachment types for agent-diva
//!
//! This module provides the `FileAttachment` struct that represents
//! a file managed by the agent-diva-files content-addressed storage system.
//!
//! ## Overview
//!
//! When a file is uploaded through any channel (Telegram, Discord, etc.),
//! it gets stored in the content-addressed storage. The `FileAttachment`
//! struct provides a unified view of such stored files across all channels.
//!
//! ## Usage
//!
//! ```ignore
//! use agent_diva_core::attachment::FileAttachment;
//! use agent_diva_files::{FileManager, FileConfig};
//! use agent_diva_files::handle::FileMetadata;
//! use std::path::PathBuf;
//!
//! // Create a FileHandle first (see agent-diva-files crate)
//! let config = FileConfig::with_path(PathBuf::from("./data"));
//! let manager = FileManager::new(config).await?;
//! let metadata = FileMetadata {
//!     name: "document.pdf".to_string(),
//!     size: 1024,
//!     mime_type: Some("application/pdf".to_string()),
//!     source: Some("telegram".to_string()),
//!     created_at: chrono::Utc::now(),
//!     last_accessed_at: None,
//!     preview: None,
//! };
//! let handle = manager.store(b"dummy content", metadata).await?;
//! let attachment = FileAttachment::from_handle(handle, "telegram", Some("msg_123"));
//! ```

use agent_diva_files::handle::FileMetadata;
use agent_diva_files::FileHandle;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Unified file attachment representation
///
/// This struct wraps a `FileHandle` from agent-diva-files and adds
/// channel-specific metadata for tracking which channel and message
/// the file was associated with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAttachment {
    /// Content-addressed file ID (SHA256 hash)
    pub file_id: String,

    /// Original filename as uploaded
    pub filename: String,

    /// File size in bytes
    pub size: u64,

    /// MIME type if known
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// Source channel (e.g., "telegram", "discord", "slack")
    pub channel: String,

    /// Associated message ID from the channel
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    /// User who uploaded the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uploaded_by: Option<String>,

    /// When the file was stored
    pub stored_at: DateTime<Utc>,

    /// Reference count (number of channel associations)
    pub ref_count: usize,
}

impl FileAttachment {
    /// Create a `FileAttachment` from a `FileHandle` and channel info
    ///
    /// # Arguments
    /// * `handle` - The FileHandle from agent-diva-files
    /// * `channel` - The source channel identifier
    /// * `message_id` - Optional message ID from the channel
    ///
    /// # Example
    /// ```ignore
    /// use agent_diva_core::attachment::FileAttachment;
    /// use agent_diva_files::{FileManager, FileConfig};
    /// use agent_diva_files::handle::FileMetadata;
    /// use std::path::PathBuf;
    ///
    /// let config = FileConfig::with_path(PathBuf::from("./data"));
    /// let manager = FileManager::new(config).await?;
    /// let metadata = FileMetadata {
    ///     name: "document.pdf".to_string(),
    ///     size: 1024,
    ///     mime_type: Some("application/pdf".to_string()),
    ///     source: Some("telegram".to_string()),
    ///     created_at: chrono::Utc::now(),
    ///     last_accessed_at: None,
    ///     preview: None,
    /// };
    /// let handle = manager.store(b"dummy content", metadata).await?;
    /// let attachment = FileAttachment::from_handle(handle, "telegram", Some("123456"));
    /// ```
    pub fn from_handle(
        handle: FileHandle,
        channel: &str,
        message_id: Option<&str>,
    ) -> Self {
        // Get values before consuming the handle
        let ref_count = handle.ref_count();
        let file_id = handle.id;
        let metadata = &handle.metadata;

        Self {
            file_id,
            filename: metadata.name.clone(),
            size: metadata.size,
            mime_type: metadata.mime_type.clone(),
            channel: channel.to_string(),
            message_id: message_id.map(String::from),
            uploaded_by: metadata.source.clone(),
            stored_at: metadata.created_at,
            ref_count,
        }
    }

    /// Create a `FileAttachment` from stored metadata
    ///
    /// This is useful when reconstructing an attachment from the database
    /// without needing the full FileHandle.
    pub fn from_metadata(
        file_id: &str,
        metadata: &FileMetadata,
        channel: &str,
        message_id: Option<&str>,
        ref_count: usize,
    ) -> Self {
        Self {
            file_id: file_id.to_string(),
            filename: metadata.name.clone(),
            size: metadata.size,
            mime_type: metadata.mime_type.clone(),
            channel: channel.to_string(),
            message_id: message_id.map(String::from),
            uploaded_by: metadata.source.clone(),
            stored_at: metadata.created_at,
            ref_count,
        }
    }

    /// Get a display string for the attachment
    ///
    /// # Example
    /// ```ignore
    /// use agent_diva_core::attachment::FileAttachment;
    /// use agent_diva_files::{FileManager, FileConfig};
    /// use agent_diva_files::handle::FileMetadata;
    /// use std::path::PathBuf;
    ///
    /// let config = FileConfig::with_path(PathBuf::from("./data"));
    /// let manager = FileManager::new(config).await?;
    /// let metadata = FileMetadata {
    ///     name: "document.pdf".to_string(),
    ///     size: 1024 * 1024, // 1 MB
    ///     mime_type: Some("application/pdf".to_string()),
    ///     source: Some("telegram".to_string()),
    ///     created_at: chrono::Utc::now(),
    ///     last_accessed_at: None,
    ///     preview: None,
    /// };
    /// let handle = manager.store(b"dummy content", metadata).await?;
    /// let attachment = FileAttachment::from_handle(handle, "telegram", Some("msg_123"));
    /// let display = attachment.display();
    /// // "document.pdf (1.0 MB) from telegram"
    /// ```
    pub fn display(&self) -> String {
        let size_str = Self::format_size(self.size);
        format!("{} ({}) from {}", self.filename, size_str, self.channel)
    }

    /// Format file size in human-readable format
    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.1} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.1} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} B", bytes)
        }
    }

    /// Check if this attachment is an image
    pub fn is_image(&self) -> bool {
        self.mime_type
            .as_ref()
            .map(|m| m.starts_with("image/"))
            .unwrap_or(false)
    }

    /// Check if this attachment is a video
    pub fn is_video(&self) -> bool {
        self.mime_type
            .as_ref()
            .map(|m| m.starts_with("video/"))
            .unwrap_or(false)
    }

    /// Check if this attachment is audio
    pub fn is_audio(&self) -> bool {
        self.mime_type
            .as_ref()
            .map(|m| m.starts_with("audio/"))
            .unwrap_or(false)
    }

    /// Check if this attachment is a document
    pub fn is_document(&self) -> bool {
        self.mime_type
            .as_ref()
            .map(|m| {
                m.starts_with("application/pdf")
                    || m.starts_with("application/")
                    || m.starts_with("text/")
            })
            .unwrap_or(false)
    }
}

impl std::fmt::Display for FileAttachment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_metadata() -> FileMetadata {
        FileMetadata {
            name: "test_document.pdf".to_string(),
            size: 1024 * 1024, // 1 MB
            mime_type: Some("application/pdf".to_string()),
            source: Some("telegram".to_string()),
            created_at: Utc::now(),
            last_accessed_at: None,
            preview: None,
        }
    }

    fn create_test_handle() -> FileHandle {
        let metadata = create_test_metadata();
        FileHandle::new(
            "sha256:abc123def456".to_string(),
            PathBuf::from("ab/c123def456"),
            metadata,
        )
    }

    #[test]
    fn test_from_handle() {
        let handle = create_test_handle();
        let attachment = FileAttachment::from_handle(handle, "telegram", Some("msg_789"));

        assert_eq!(attachment.file_id, "sha256:abc123def456");
        assert_eq!(attachment.filename, "test_document.pdf");
        assert_eq!(attachment.size, 1024 * 1024);
        assert_eq!(attachment.mime_type, Some("application/pdf".to_string()));
        assert_eq!(attachment.channel, "telegram");
        assert_eq!(attachment.message_id, Some("msg_789".to_string()));
        assert_eq!(attachment.uploaded_by, Some("telegram".to_string()));
    }

    #[test]
    fn test_display() {
        let handle = create_test_handle();
        let attachment = FileAttachment::from_handle(handle, "discord", None);

        let display = attachment.display();
        assert!(display.contains("test_document.pdf"));
        assert!(display.contains("discord"));
        assert!(display.contains("MB")); // 1 MB formatted
    }

    #[test]
    fn test_is_image() {
        let mut handle = create_test_handle();
        handle.metadata.mime_type = Some("image/png".to_string());
        let attachment = FileAttachment::from_handle(handle, "telegram", None);

        assert!(attachment.is_image());
        assert!(!attachment.is_video());
        assert!(!attachment.is_audio());
        assert!(!attachment.is_document());
    }

    #[test]
    fn test_is_video() {
        let mut handle = create_test_handle();
        handle.metadata.mime_type = Some("video/mp4".to_string());
        let attachment = FileAttachment::from_handle(handle, "discord", None);

        assert!(!attachment.is_image());
        assert!(attachment.is_video());
    }

    #[test]
    fn test_format_size() {
        assert_eq!(FileAttachment::format_size(500), "500 B");
        assert_eq!(FileAttachment::format_size(1024), "1.0 KB");
        assert_eq!(FileAttachment::format_size(1024 * 512), "512.0 KB");
        assert_eq!(FileAttachment::format_size(1024 * 1024), "1.0 MB");
        assert_eq!(FileAttachment::format_size(1024 * 1024 * 50), "50.0 MB");
        assert_eq!(
            FileAttachment::format_size(1024 * 1024 * 1024),
            "1.0 GB"
        );
    }

    #[test]
    fn test_serialize() {
        let handle = create_test_handle();
        let attachment = FileAttachment::from_handle(handle, "slack", Some("ts_123"));

        let json = serde_json::to_string(&attachment).unwrap();
        assert!(json.contains("test_document.pdf"));
        assert!(json.contains("slack"));
        assert!(json.contains("sha256:abc123def456"));
    }

    #[test]
    fn test_deserialize() {
        let json = r#"{
            "file_id": "sha256:test123",
            "filename": "doc.pdf",
            "size": 2048,
            "mime_type": "application/pdf",
            "channel": "telegram",
            "message_id": "msg_456",
            "uploaded_by": "user123",
            "stored_at": "2024-01-15T10:30:00Z",
            "ref_count": 3
        }"#;

        let attachment: FileAttachment = serde_json::from_str(json).unwrap();
        assert_eq!(attachment.file_id, "sha256:test123");
        assert_eq!(attachment.filename, "doc.pdf");
        assert_eq!(attachment.size, 2048);
        assert_eq!(attachment.channel, "telegram");
    }
}
