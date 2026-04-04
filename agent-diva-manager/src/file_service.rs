use agent_diva_core::attachment::FileAttachment;
use agent_diva_files::handle::FileMetadata;
use agent_diva_files::{FileConfig, FileManager};
use anyhow::anyhow;
use std::path::PathBuf;
use tokio::sync::OnceCell;

static FILE_MANAGER: OnceCell<FileManager> = OnceCell::const_new();

pub struct FileService;

impl FileService {
    pub fn new() -> Self {
        Self
    }

    pub async fn get_manager() -> anyhow::Result<&'static FileManager> {
        FILE_MANAGER
            .get_or_try_init(|| async {
                let data_dir = Self::data_dir()?;
                let config = FileConfig::with_path(data_dir);
                FileManager::new(config)
                    .await
                    .map_err(|e| anyhow!("{:?}", e))
            })
            .await
            .map_err(|e| anyhow!("{:?}", e))
    }

    fn data_dir() -> anyhow::Result<PathBuf> {
        let base = dirs::data_local_dir()
            .ok_or_else(|| anyhow!("failed to find local data directory"))?;
        Ok(base.join("agent-diva").join("files"))
    }

    pub async fn upload_file(
        &self,
        file_name: &str,
        bytes: Vec<u8>,
        channel: &str,
        message_id: Option<&str>,
    ) -> anyhow::Result<FileAttachment> {
        let manager = Self::get_manager().await?;

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

        let handle = manager.store(&bytes, metadata).await?;

        let attachment = FileAttachment::from_handle(handle, channel, message_id);

        Ok(attachment)
    }

    pub async fn read_file(&self, file_id: &str) -> anyhow::Result<Vec<u8>> {
        let manager = Self::get_manager().await?;

        let handle = manager.get(file_id).await?;
        let content = manager.read(&handle).await?;

        Ok(content)
    }
}

impl Default for FileService {
    fn default() -> Self {
        Self::new()
    }
}
