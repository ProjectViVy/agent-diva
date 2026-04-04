//! Attachment tools for reading files from content-addressed storage

use crate::base::{Result, Tool};
use agent_diva_files::{FileConfig, FileManager};
use anyhow::anyhow;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::sync::OnceCell;

static FILE_MANAGER: OnceCell<FileManager> = OnceCell::const_new();

fn data_dir() -> anyhow::Result<PathBuf> {
    let base = dirs::data_local_dir()
        .ok_or_else(|| anyhow!("failed to find local data directory"))?;
    Ok(base.join("agent-diva").join("files"))
}

async fn get_file_manager() -> anyhow::Result<&'static FileManager> {
    FILE_MANAGER
        .get_or_try_init(|| async {
            let data_dir = data_dir()?;
            let config = FileConfig::with_path(data_dir);
            FileManager::new(config)
                .await
                .map_err(|e| anyhow!("{:?}", e))
        })
        .await
        .map_err(|e| anyhow!("{:?}", e))
}

/// Read attachment tool - reads files from the content-addressed storage
pub struct ReadAttachmentTool;

impl ReadAttachmentTool {
    /// Create a new read attachment tool
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReadAttachmentTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ReadAttachmentTool {
    fn name(&self) -> &str {
        "read_attachment"
    }

    fn description(&self) -> &str {
        "Read the content of an uploaded file attachment by its file_id (SHA256 hash). \
         Returns the file content as text, or information about the file if it cannot be read as text."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_id": {
                    "type": "string",
                    "description": "The file_id (SHA256 hash) of the attachment to read"
                },
                "max_size": {
                    "type": "integer",
                    "description": "Maximum file size in bytes to read (default: 1048576 = 1MB). Files larger than this will return an error."
                }
            },
            "required": ["file_id"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let file_id = match params.get("file_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok("Error: Missing 'file_id' parameter".to_string()),
        };

        let max_size = params
            .get("max_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(1048576); // 1MB default

        let manager = match get_file_manager().await {
            Ok(m) => m,
            Err(e) => return Ok(format!("Error: Failed to access file storage: {}", e)),
        };

        let handle = match manager.get(file_id).await {
            Ok(h) => h,
            Err(e) => {
                return Ok(format!(
                    "Error: File not found or inaccessible: {}. \
                     The file may not have been uploaded yet, or may have been deleted.",
                    e
                ))
            }
        };

        // Check file size
        if handle.metadata.size > max_size {
            return Ok(format!(
                "Error: File is too large ({} bytes) to read. \
                 Maximum allowed size is {} bytes. \
                 You can try reading a preview or specify a smaller max_size.",
                handle.metadata.size, max_size
            ));
        }

        // Read file content
        let content = match manager.read(&handle).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(format!("Error: Failed to read file content: {}", e));
            }
        };

        // Try to convert to text for display
        match String::from_utf8(content.clone()) {
            Ok(text) => {
                // Truncate if too large for display
                const MAX_DISPLAY_CHARS: usize = 50000;
                if text.chars().count() > MAX_DISPLAY_CHARS {
                    let preview: String = text.chars().take(MAX_DISPLAY_CHARS).collect();
                    Ok(format!(
                        "[File truncated - showing first {} characters]\n\n{}...\n\n[File continues - {} total characters]",
                        MAX_DISPLAY_CHARS,
                        preview,
                        text.chars().count()
                    ))
                } else {
                    Ok(text)
                }
            }
            Err(_) => {
                // Binary file - return info instead of raw bytes
                let mime_type = handle
                    .metadata
                    .mime_type
                    .as_deref()
                    .unwrap_or("application/octet-stream");
                Ok(format!(
                    "[Binary file: {}]\n\
                     Filename: {}\n\
                     Size: {} bytes\n\
                     MIME type: {}\n\
                     \n\
                     This file cannot be displayed as text. \
                     The file content is stored and can be processed by other tools \
                     or transferred to external services.",
                    file_id,
                    handle.metadata.name,
                    handle.metadata.size,
                    mime_type
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_metadata() {
        let tool = ReadAttachmentTool::new();
        assert_eq!(tool.name(), "read_attachment");
        assert!(!tool.description().is_empty());
        assert!(tool.parameters().is_object());
    }

    #[tokio::test]
    async fn test_missing_file_id() {
        let tool = ReadAttachmentTool::new();
        let result = tool.execute(json!({})).await.unwrap();
        assert!(result.contains("Missing 'file_id'"));
    }
}
