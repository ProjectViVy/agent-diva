use agent_diva_core::attachment::FileAttachment;
use agent_diva_files::handle::FileMetadata;
use agent_diva_files::FileManager;
use std::sync::Arc;

/// File service for handling file uploads and downloads
///
/// Uses a shared Arc<FileManager> to ensure data consistency
/// across all components. Do NOT create multiple FileManager instances.
pub struct FileService {
    manager: Arc<FileManager>,
}

impl FileService {
    /// Create a new file service with the given file manager
    ///
    /// # Arguments
    /// * `manager` - Shared FileManager instance
    ///
    /// # Example
    /// ```ignore
    /// use agent_diva_files::{FileManager, FileConfig};
    /// use agent_diva_manager::file_service::FileService;
    /// use std::sync::Arc;
    ///
    /// let config = FileConfig::default();
    /// let manager = Arc::new(FileManager::new(config).await?);
    /// let file_service = FileService::new(manager);
    /// ```
    pub fn new(manager: Arc<FileManager>) -> Self {
        Self { manager }
    }

    /// Upload a file and return a FileAttachment
    ///
    /// # Arguments
    /// * `file_name` - Original file name
    /// * `bytes` - File content
    /// * `channel` - Source channel (e.g., "ui", "telegram")
    /// * `message_id` - Optional message ID for association
    pub async fn upload_file(
        &self,
        file_name: &str,
        bytes: Vec<u8>,
        channel: &str,
        message_id: Option<&str>,
    ) -> anyhow::Result<FileAttachment> {
        let mime_type = mime_guess::from_path(file_name)
            .first()
            .map(|m| m.to_string());

        let metadata = FileMetadata {
            name: file_name.to_string(),
            size: bytes.len() as u64,
            mime_type,
            source: Some(channel.to_string()),
            created_at: chrono::Utc::now(),
            last_accessed_at: None,
            preview: None,
        };

        let handle = self.manager.store(&bytes, metadata).await?;

        let attachment = FileAttachment::from_handle(handle, channel, message_id);

        Ok(attachment)
    }

    /// Read file content by ID
    ///
    /// # Arguments
    /// * `file_id` - File ID (SHA256 hash)
    pub async fn read_file(&self, file_id: &str) -> anyhow::Result<Vec<u8>> {
        let handle = self.manager.get(file_id).await?;
        let content = self.manager.read(&handle).await?;

        Ok(content)
    }

    /// Get the underlying FileManager
    pub fn manager(&self) -> &FileManager {
        &self.manager
    }

    /// Get a clone of the Arc<FileManager>
    pub fn manager_arc(&self) -> Arc<FileManager> {
        self.manager.clone()
    }
}

impl Clone for FileService {
    fn clone(&self) -> Self {
        Self {
            manager: self.manager.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_files::FileConfig;
    use tempfile::TempDir;

    async fn create_test_service() -> (FileService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = FileConfig::with_path(temp_dir.path().to_path_buf());
        let manager = Arc::new(FileManager::new(config).await.unwrap());
        let service = FileService::new(manager);
        (service, temp_dir)
    }

    #[tokio::test]
    async fn test_upload_and_read() {
        let (service, _temp) = create_test_service().await;

        let content = b"hello world";
        let attachment = service
            .upload_file("test.txt", content.to_vec(), "test", None)
            .await
            .unwrap();

        assert!(attachment.file_id.starts_with("sha256:"));
        assert_eq!(attachment.file_name, "test.txt");

        // Read back
        let read_content = service.read_file(&attachment.file_id).await.unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_clone() {
        let (service, _temp) = create_test_service().await;
        let cloned = service.clone();

        // Both should share the same manager
        let content = b"test";
        let attachment = service
            .upload_file("test.txt", content.to_vec(), "test", None)
            .await
            .unwrap();

        // Clone should be able to read
        let read_content = cloned.read_file(&attachment.file_id).await.unwrap();
        assert_eq!(read_content, content);
    }
}
