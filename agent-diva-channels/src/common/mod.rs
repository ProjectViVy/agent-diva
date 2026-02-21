use crate::base::{ChannelError, Result};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

/// Create a standard HTTP client with timeout
pub fn create_http_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ChannelError::Error(format!("Failed to build HTTP client: {}", e)))
}

/// Download a file from a URL to the media directory
pub async fn download_file(
    client: &reqwest::Client,
    url: &str,
    filename: &str,
    prefix: &str,
) -> Result<String> {
    let media_dir = dirs::home_dir()
        .map(|h: PathBuf| h.join(".nanobot").join("media"))
        .unwrap_or_else(|| PathBuf::from(".nanobot/media"));

    tokio::fs::create_dir_all(&media_dir)
        .await
        .map_err(|e| ChannelError::Error(format!("Failed to create media dir: {}", e)))?;

    let safe_filename = filename.replace('/', "_");
    let file_path = media_dir.join(format!("{}_{}", prefix, safe_filename));

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| ChannelError::ApiError(format!("Download failed: {}", e)))?;

    let bytes = response
        .bytes()
        .await
        .map_err(|e| ChannelError::ApiError(format!("Read failed: {}", e)))?;

    let mut file = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| ChannelError::Error(format!("File creation failed: {}", e)))?;

    file.write_all(&bytes)
        .await
        .map_err(|e| ChannelError::Error(format!("Write failed: {}", e)))?;

    Ok(file_path.to_string_lossy().to_string())
}
