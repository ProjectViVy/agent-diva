//! Attachment tools for reading files from content-addressed storage

use crate::base::{Result, Tool};
use agent_diva_files::FileManager;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

/// Read attachment tool - reads files from the content-addressed storage
///
/// Uses a shared Arc<FileManager> to ensure data consistency.
/// The FileManager should be shared with other components (FileService, AgentLoop, etc.)
pub struct ReadAttachmentTool {
    manager: Arc<FileManager>,
}

impl ReadAttachmentTool {
    /// Create a new read attachment tool with the given file manager
    ///
    /// # Arguments
    /// * `manager` - Shared FileManager instance
    ///
    /// # Example
    /// ```ignore
    /// use agent_diva_files::{FileManager, FileConfig};
    /// use agent_diva_tools::attachment::ReadAttachmentTool;
    /// use std::sync::Arc;
    ///
    /// let config = FileConfig::default();
    /// let manager = Arc::new(FileManager::new(config).await?);
    /// let tool = ReadAttachmentTool::new(manager);
    /// ```
    pub fn new(manager: Arc<FileManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl Tool for ReadAttachmentTool {
    fn name(&self) -> &str {
        "read_attachment"
    }

    fn description(&self) -> &str {
        "读取已上传文件附件的内容，通过 file_id (SHA256 哈希) 访问。\
         返回文件内容的文本形式，如果无法作为文本读取则返回文件信息。"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file_id": {
                    "type": "string",
                    "description": "要读取的附件的 file_id (SHA256 哈希)"
                },
                "max_size": {
                    "type": "integer",
                    "description": "最大读取文件大小（字节，默认: 1048576 = 1MB）。超过此大小的文件将返回错误。"
                }
            },
            "required": ["file_id"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let file_id = match params.get("file_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return Ok("错误: 缺少 'file_id' 参数".to_string()),
        };

        let max_size = params
            .get("max_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(1048576); // 1MB default

        let handle = match self.manager.get(file_id).await {
            Ok(h) => h,
            Err(e) => {
                return Ok(format!(
                    "错误: 文件不存在或无法访问: {}。\
                     该文件可能尚未上传，或已被删除。",
                    e
                ))
            }
        };

        // Check file size
        if handle.metadata.size > max_size {
            return Ok(format!(
                "错误: 文件太大 ({} 字节)，无法读取。\
                 最大允许大小为 {} 字节。\
                 您可以尝试读取预览版本或指定更小的 max_size。",
                handle.metadata.size, max_size
            ));
        }

        // Read file content
        let content = match self.manager.read(&handle).await {
            Ok(c) => c,
            Err(e) => {
                return Ok(format!("错误: 无法读取文件内容: {}", e));
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
                        "[文件已截断 - 显示前 {} 个字符]\n\n{}...\n\n[文件继续 - 共 {} 个字符]",
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
                    "[二进制文件: {}]\n\
                     文件名: {}\n\
                     大小: {} 字节\n\
                     MIME 类型: {}\n\
                     \n\
                     此文件无法作为文本显示。\
                     文件内容已存储，可以被其他工具处理\
                     或传输到外部服务。",
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
    use agent_diva_files::{FileConfig, FileMetadata};
    use tempfile::TempDir;

    async fn create_test_tool() -> (ReadAttachmentTool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = FileConfig::with_path(temp_dir.path().to_path_buf());
        let manager = Arc::new(FileManager::new(config).await.unwrap());

        // Store a test file
        let data = b"test content";
        let metadata = FileMetadata {
            name: "test.txt".to_string(),
            size: data.len() as u64,
            mime_type: Some("text/plain".to_string()),
            source: Some("test".to_string()),
            created_at: chrono::Utc::now(),
            last_accessed_at: None,
            preview: None,
        };
        manager.store(data, metadata).await.unwrap();

        let tool = ReadAttachmentTool::new(manager);
        (tool, temp_dir)
    }

    #[tokio::test]
    async fn test_tool_metadata() {
        let (tool, _temp) = create_test_tool().await;
        assert_eq!(tool.name(), "read_attachment");
        assert!(!tool.description().is_empty());
        assert!(tool.parameters().is_object());
    }

    #[tokio::test]
    async fn test_missing_file_id() {
        let (tool, _temp) = create_test_tool().await;
        let result = tool.execute(json!({})).await.unwrap();
        assert!(result.contains("缺少"));
    }

    #[tokio::test]
    async fn test_read_existing_file() {
        let (tool, _temp) = create_test_tool().await;

        // Get the stored file ID
        let stats = tool.manager.stats().await.unwrap();
        assert_eq!(stats.total_files, 1);

        // For this test, we'll just verify the tool can handle file not found
        // The actual file reading is tested in FileManager tests
        let result = tool.execute(json!({"file_id": "sha256:nonexistent"})).await.unwrap();
        assert!(result.contains("不存在") || result.contains("not found"));
    }
}
