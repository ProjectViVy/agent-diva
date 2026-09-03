use agent_diva_channels::adapter::{
    validate_attachment_reference, AttachmentStoreError, ChannelAttachmentStore, IngressAttachment,
    StoredAttachment,
};
use agent_diva_core::channel::AttachmentRef;
use agent_diva_files::{handle::FileMetadata, FileError, FileManager};
use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

pub struct FileManagerAttachmentStore {
    files: Arc<FileManager>,
}

impl FileManagerAttachmentStore {
    pub fn new(files: Arc<FileManager>) -> Self {
        Self { files }
    }
}

#[async_trait]
impl ChannelAttachmentStore for FileManagerAttachmentStore {
    async fn put(&self, input: IngressAttachment) -> Result<AttachmentRef, AttachmentStoreError> {
        input.validate()?;
        let media_type = input
            .declared_mime
            .clone()
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let metadata = FileMetadata {
            name: input
                .file_name
                .clone()
                .unwrap_or_else(|| "channel-attachment".to_string()),
            size: input.bytes.len() as u64,
            mime_type: Some(media_type.clone()),
            source: Some(input.source_channel),
            created_at: Utc::now(),
            last_accessed_at: None,
            preview: None,
        };
        let handle = self
            .files
            .store(&input.bytes, metadata)
            .await
            .map_err(map_file_error)?;
        let sha256 = handle.id.strip_prefix("sha256:").ok_or_else(|| {
            AttachmentStoreError::InvalidReference {
                diagnosis: format!("file authority returned non-sha256 id: {}", handle.id),
            }
        })?;
        let reference = AttachmentRef {
            uri: handle.id.clone(),
            media_type,
            size_bytes: input.bytes.len() as u64,
            sha256: sha256.to_string(),
            file_name: input.file_name,
        };
        validate_attachment_reference(&reference, &input.bytes)?;
        Ok(reference)
    }

    async fn get(
        &self,
        reference: &AttachmentRef,
    ) -> Result<StoredAttachment, AttachmentStoreError> {
        let handle = self
            .files
            .get(&reference.uri)
            .await
            .map_err(map_file_error)?;
        let bytes = self.files.read(&handle).await.map_err(map_file_error)?;
        validate_attachment_reference(reference, &bytes)?;
        Ok(StoredAttachment {
            reference: reference.clone(),
            bytes,
        })
    }
}

fn map_file_error(error: FileError) -> AttachmentStoreError {
    match error {
        FileError::TooLarge(size_bytes, max_bytes) => AttachmentStoreError::TooLarge {
            size_bytes,
            max_bytes,
        },
        FileError::NotFound(uri) => AttachmentStoreError::NotFound { uri },
        FileError::HashMismatch(_, _) => AttachmentStoreError::DigestMismatch {
            uri: "sha256:unknown".to_string(),
        },
        other => AttachmentStoreError::Backend {
            diagnosis: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_channels::adapter::IngressAttachment;
    use agent_diva_files::FileConfig;
    use tempfile::TempDir;

    #[tokio::test]
    async fn file_manager_is_the_content_addressed_channel_authority() {
        let root = TempDir::new().unwrap();
        let files = Arc::new(
            FileManager::new(FileConfig::with_path(root.path().to_path_buf()))
                .await
                .unwrap(),
        );
        let store = FileManagerAttachmentStore::new(files);
        let reference = store
            .put(IngressAttachment {
                source_channel: "telegram".into(),
                platform_message_id: Some("m-1".into()),
                sender_id: Some("u-1".into()),
                file_name: Some("hello.txt".into()),
                declared_mime: Some("text/plain".into()),
                bytes: b"hello".to_vec(),
            })
            .await
            .unwrap();

        assert!(reference.uri.starts_with("sha256:"));
        assert_eq!(reference.uri, format!("sha256:{}", reference.sha256));
        let stored = store.get(&reference).await.unwrap();
        assert_eq!(stored.bytes, b"hello");
        stored.validate().unwrap();
    }
}
