//! Filesystem tools with SecurityPolicy integration
//!
//! This module provides file operations with comprehensive security checks
//! including path validation, rate limiting, and workspace restrictions.

use crate::base::{Result, Tool};
use crate::sanitize::{sanitize_for_json, truncate_file_content, MAX_FILE_CONTENT_CHARS};
use agent_diva_core::security::{SecurityError, SecurityPolicy, SharedSecurityPolicy};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// Read file tool with security policy
pub struct ReadFileTool {
    security: SharedSecurityPolicy,
}

impl ReadFileTool {
    /// Create a new read file tool with a security policy
    pub fn new(security: SharedSecurityPolicy) -> Self {
        Self { security }
    }

    /// Create a new read file tool with default policy for a workspace
    pub fn for_workspace(workspace: PathBuf) -> Self {
        let policy = Arc::new(SecurityPolicy::new(workspace));
        Self::new(policy)
    }
}

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::for_workspace(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "读取指定路径文件的内容。支持通过偏移量和行数限制读取大文件。"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要读取的文件路径（相对于工作区）"
                },
                "offset": {
                    "type": "integer",
                    "description": "开始读取的行号（从1开始）"
                },
                "limit": {
                    "type": "integer",
                    "description": "最多读取的行数"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return Ok("Error: Missing 'path' parameter".to_string()),
        };

        let offset = params.get("offset").and_then(|v| v.as_u64()).map(|v| v as usize);
        let limit = params.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

        // Security checks
        if let Err(e) = self.security.try_record_action() {
            return Ok(format!("Error: {}", e.user_message()));
        }

        // Validate path and resolve
        let resolved_path = match self.security.validate_path(path).await {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {}", e.user_message())),
        };

        // Check file existence and type
        let metadata = match tokio::fs::metadata(&resolved_path).await {
            Ok(m) => m,
            Err(e) => return Ok(format!("Error: File not found: {} ({})", path, e)),
        };

        if !metadata.is_file() {
            return Ok(format!("Error: Not a file: {}", path));
        }

        // Check file size
        if let Err(e) = self.security.check_file_size(metadata.len()) {
            return Ok(format!("Error: {}", e.user_message()));
        }

        // Read file content with memory-efficient approach
        let processed = if offset.is_some() || limit.is_some() {
            // Use streaming read for large files when offset/limit is specified
            match read_file_with_offset_limit(&resolved_path, offset, limit).await {
                Ok(content) => content,
                Err(e) => return Ok(format!("Error reading file: {}", e)),
            }
        } else {
            // Read entire file for small files or when no offset/limit specified
            let content = match tokio::fs::read_to_string(&resolved_path).await {
                Ok(c) => c,
                Err(e) => {
                    // Try binary fallback for non-UTF8 files
                    match tokio::fs::read(&resolved_path).await {
                        Ok(bytes) => {
                            let lossy = String::from_utf8_lossy(&bytes);
                            lossy.into_owned()
                        }
                        Err(_) => return Ok(format!("Error reading file: {}", e)),
                    }
                }
            };
            content
        };

        // Sanitize and truncate if needed
        let char_count = processed.chars().count();
        let result = if char_count > MAX_FILE_CONTENT_CHARS {
            truncate_file_content(&sanitize_for_json(&processed))
        } else {
            sanitize_for_json(&processed)
        };

        Ok(result)
    }
}

/// Apply offset and limit to file content
fn apply_offset_limit(content: &str, offset: Option<usize>, limit: Option<usize>) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let total = lines.len();

    let start = offset
        .map(|o| o.saturating_sub(1).min(total))
        .unwrap_or(0);

    let end = match limit {
        Some(l) => (start + l).min(total),
        None => total,
    };

    if start >= end {
        return format!("[No lines in range, file has {} lines]", total);
    }

    let numbered: String = lines[start..end]
        .iter()
        .enumerate()
        .map(|(i, line)| format!("{}: {}", start + i + 1, line))
        .collect::<Vec<_>>()
        .join("\n");

    let summary = if start > 0 || end < total {
        format!("\n[Lines {}-{} of {}]", start + 1, end, total)
    } else {
        format!("\n[{} lines total]", total)
    };

    format!("{}{}", numbered, summary)
}

/// Write file tool with security policy
pub struct WriteFileTool {
    security: SharedSecurityPolicy,
}

impl WriteFileTool {
    /// Create a new write file tool with a security policy
    pub fn new(security: SharedSecurityPolicy) -> Self {
        Self { security }
    }

    /// Create a new write file tool with default policy for a workspace
    pub fn for_workspace(workspace: PathBuf) -> Self {
        let policy = Arc::new(SecurityPolicy::new(workspace));
        Self::new(policy)
    }
}

impl Default for WriteFileTool {
    fn default() -> Self {
        Self::for_workspace(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "将内容写入指定路径的文件。如需要会自动创建父目录。"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要写入的文件路径（相对于工作区）"
                },
                "content": {
                    "type": "string",
                    "description": "要写入的内容"
                }
            },
            "required": ["path", "content"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return Ok("Error: Missing 'path' parameter".to_string()),
        };

        let content = match params.get("content").and_then(|v| v.as_str()) {
            Some(c) => c,
            None => return Ok("Error: Missing 'content' parameter".to_string()),
        };

        // Security checks
        if let Err(e) = self.security.can_act() {
            return Ok(format!("Error: {}", e.user_message()));
        }

        // Basic path validation
        if let Err(e) = self.security.is_path_allowed(path) {
            return Ok(format!("Error: {}", e.user_message()));
        }

        // Resolve the target path
        let full_path = self.security.resolve_path(path);

        // Validate parent directory (TOCTOU-safe)
        let resolved_parent = match self.security.validate_parent_directory(&full_path).await {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {}", e.user_message())),
        };

        // Get file name and construct final resolved path
        let file_name = match full_path.file_name() {
            Some(name) => name,
            None => return Ok("Error: Invalid path - no file name".to_string()),
        };

        let resolved_target = resolved_parent.join(file_name);

        // Check for symlink if target exists
        if !self.security.config().allow_symlinks {
            if let Ok(meta) = tokio::fs::symlink_metadata(&resolved_target).await {
                if meta.file_type().is_symlink() {
                    return Ok(format!(
                        "Error: {}",
                        SecurityError::SymlinkNotAllowed {
                            path: resolved_target.clone()
                        }
                        .user_message()
                    ));
                }
            }
        }

        // Write file
        match tokio::fs::write(&resolved_target, content).await {
            Ok(_) => Ok(format!(
                "Successfully wrote {} bytes to {}",
                content.len(),
                path
            )),
            Err(e) => Ok(format!("Error writing file: {}", e)),
        }
    }
}

/// Edit file tool with security policy
pub struct EditFileTool {
    security: SharedSecurityPolicy,
}

impl EditFileTool {
    /// Create a new edit file tool with a security policy
    pub fn new(security: SharedSecurityPolicy) -> Self {
        Self { security }
    }

    /// Create a new edit file tool with default policy for a workspace
    pub fn for_workspace(workspace: PathBuf) -> Self {
        let policy = Arc::new(SecurityPolicy::new(workspace));
        Self::new(policy)
    }
}

impl Default for EditFileTool {
    fn default() -> Self {
        Self::for_workspace(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "编辑文件，将 old_text 替换为 new_text。old_text 必须完全匹配文件中存在的文本。"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要编辑的文件路径（相对于工作区）"
                },
                "old_text": {
                    "type": "string",
                    "description": "要查找并替换的精确文本"
                },
                "new_text": {
                    "type": "string",
                    "description": "用于替换的新文本"
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return Ok("Error: Missing 'path' parameter".to_string()),
        };

        let old_text = match params.get("old_text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return Ok("Error: Missing 'old_text' parameter".to_string()),
        };

        let new_text = match params.get("new_text").and_then(|v| v.as_str()) {
            Some(t) => t,
            None => return Ok("Error: Missing 'new_text' parameter".to_string()),
        };

        // Security checks
        if let Err(e) = self.security.can_act() {
            return Ok(format!("Error: {}", e.user_message()));
        }

        // Validate path and resolve
        let resolved_path = match self.security.validate_path(path).await {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {}", e.user_message())),
        };

        // Check if file exists
        let metadata = match tokio::fs::metadata(&resolved_path).await {
            Ok(m) => m,
            Err(e) => return Ok(format!("Error: File not found: {} ({})", path, e)),
        };

        if !metadata.is_file() {
            return Ok(format!("Error: Not a file: {}", path));
        }

        // Read file content
        let content = match tokio::fs::read_to_string(&resolved_path).await {
            Ok(c) => c,
            Err(e) => return Ok(format!("Error reading file: {}", e)),
        };

        // Validate old_text exists
        if !content.contains(old_text) {
            return Ok("Error: old_text not found in file. Make sure it matches exactly.".to_string());
        }

        // Count occurrences
        let count = content.matches(old_text).count();
        if count > 1 {
            return Ok(format!(
                "Warning: old_text appears {} times. Please provide more context to make it unique.",
                count
            ));
        }

        // Perform replacement
        let new_content = content.replacen(old_text, new_text, 1);

        // Write back
        match tokio::fs::write(&resolved_path, new_content).await {
            Ok(_) => Ok(format!("Successfully edited {}", path)),
            Err(e) => Ok(format!("Error writing file: {}", e)),
        }
    }
}

/// List directory tool with security policy
pub struct ListDirTool {
    security: SharedSecurityPolicy,
}

impl ListDirTool {
    /// Create a new list dir tool with a security policy
    pub fn new(security: SharedSecurityPolicy) -> Self {
        Self { security }
    }

    /// Create a new list dir tool with default policy for a workspace
    pub fn for_workspace(workspace: PathBuf) -> Self {
        let policy = Arc::new(SecurityPolicy::new(workspace));
        Self::new(policy)
    }
}

impl Default for ListDirTool {
    fn default() -> Self {
        Self::for_workspace(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "列出目录中的内容。"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "要列出的目录路径（相对于工作区）"
                }
            },
            "required": ["path"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let path = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => p,
            None => return Ok("Error: Missing 'path' parameter".to_string()),
        };

        // Security checks
        if let Err(e) = self.security.try_record_action() {
            return Ok(format!("Error: {}", e.user_message()));
        }

        // Validate path and resolve
        let resolved_path = match self.security.validate_path(path).await {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {}", e.user_message())),
        };

        // Check if directory exists
        let metadata = match tokio::fs::metadata(&resolved_path).await {
            Ok(m) => m,
            Err(e) => return Ok(format!("Error: Directory not found: {} ({})", path, e)),
        };

        if !metadata.is_dir() {
            return Ok(format!("Error: Not a directory: {}", path));
        }

        // Read directory entries
        let mut entries = match tokio::fs::read_dir(&resolved_path).await {
            Ok(e) => e,
            Err(e) => return Ok(format!("Error reading directory: {}", e)),
        };

        let mut items = Vec::new();

        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            let prefix = if path.is_dir() { "📁 " } else { "📄 " };
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("<invalid>")
                .to_string();
            items.push(format!("{}{}", prefix, name));
        }

        if items.is_empty() {
            return Ok(format!("Directory {} is empty", path));
        }

        items.sort();
        Ok(items.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::security::SecurityLevel;
    use tempfile::TempDir;

    fn create_test_security() -> (SharedSecurityPolicy, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let policy = Arc::new(SecurityPolicy::new(temp_dir.path().to_path_buf()));
        (policy, temp_dir)
    }

    #[tokio::test]
    async fn test_read_file() {
        let (security, temp_dir) = create_test_security();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Hello, World!").await.unwrap();

        let tool = ReadFileTool::new(security);
        let params = json!({ "path": "test.txt" });
        let result = tool.execute(params).await.unwrap();

        assert_eq!(result, "Hello, World!");
    }

    #[tokio::test]
    async fn test_read_file_with_offset_limit() {
        let (security, temp_dir) = create_test_security();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line1\nline2\nline3\nline4\nline5").await.unwrap();

        let tool = ReadFileTool::new(security);
        let params = json!({ "path": "test.txt", "offset": 2, "limit": 2 });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("2: line2"));
        assert!(result.contains("3: line3"));
        assert!(result.contains("[Lines 2-3 of 5]"));
    }

    #[tokio::test]
    async fn test_write_file() {
        let (security, temp_dir) = create_test_security();

        let tool = WriteFileTool::new(security);
        let params = json!({
            "path": "test.txt",
            "content": "Test content"
        });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("Successfully wrote"));

        let file_path = temp_dir.path().join("test.txt");
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Test content");
    }

    #[tokio::test]
    async fn test_edit_file() {
        let (security, temp_dir) = create_test_security();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "Hello, World!").await.unwrap();

        let tool = EditFileTool::new(security);
        let params = json!({
            "path": "test.txt",
            "old_text": "World",
            "new_text": "Rust"
        });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("Successfully edited"));

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello, Rust!");
    }

    #[tokio::test]
    async fn test_list_dir() {
        let (security, temp_dir) = create_test_security();
        tokio::fs::write(temp_dir.path().join("file1.txt"), "").await.unwrap();
        tokio::fs::write(temp_dir.path().join("file2.txt"), "").await.unwrap();
        tokio::fs::create_dir(temp_dir.path().join("subdir")).await.unwrap();

        let tool = ListDirTool::new(security);
        let params = json!({ "path": "." });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("📄 file1.txt"));
        assert!(result.contains("📄 file2.txt"));
        assert!(result.contains("📁 subdir"));
    }

    #[tokio::test]
    async fn test_path_traversal_blocked() {
        let (security, temp_dir) = create_test_security();

        // Create a file outside temp_dir
        let outside_file = std::env::temp_dir().join("agent_diva_test_outside.txt");
        tokio::fs::write(&outside_file, "Outside content").await.unwrap();

        let tool = ReadFileTool::new(security);
        let params = json!({ "path": "../agent_diva_test_outside.txt" });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("Error:"));

        // Cleanup
        tokio::fs::remove_file(&outside_file).await.ok();
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let temp_dir = TempDir::new().unwrap();
        let config = agent_diva_core::security::SecurityConfig {
            max_actions_per_hour: 2,
            ..Default::default()
        };
        let policy = Arc::new(SecurityPolicy::with_config(
            temp_dir.path().to_path_buf(),
            config,
        ));

        // Create test files
        tokio::fs::write(temp_dir.path().join("file1.txt"), "content").await.unwrap();
        tokio::fs::write(temp_dir.path().join("file2.txt"), "content").await.unwrap();
        tokio::fs::write(temp_dir.path().join("file3.txt"), "content").await.unwrap();

        let tool = ReadFileTool::new(policy);

        // First two should succeed
        let result1 = tool.execute(json!({ "path": "file1.txt" })).await.unwrap();
        assert!(!result1.contains("Rate limit"));

        let result2 = tool.execute(json!({ "path": "file2.txt" })).await.unwrap();
        assert!(!result2.contains("Rate limit"));

        // Third should fail due to rate limit
        let result3 = tool.execute(json!({ "path": "file3.txt" })).await.unwrap();
        assert!(result3.contains("Rate limit") || result3.contains("Too many"));
    }

    #[tokio::test]
    async fn test_read_only_mode() {
        let temp_dir = TempDir::new().unwrap();
        let policy = Arc::new(SecurityPolicy::from_level(
            temp_dir.path().to_path_buf(),
            SecurityLevel::Paranoid,
        ));

        let tool = WriteFileTool::new(policy);
        let params = json!({
            "path": "test.txt",
            "content": "Test"
        });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("read-only") || result.contains("Read-only"));
    }
}

/// Read file with offset and limit using streaming for memory efficiency
async fn read_file_with_offset_limit(
    path: &std::path::Path,
    offset: Option<usize>,
    limit: Option<usize>,
) -> std::io::Result<String> {
    use tokio::io::AsyncBufReadExt;
    
    let file = tokio::fs::File::open(path).await?;
    let reader = tokio::io::BufReader::new(file);
    let mut lines = reader.lines();
    
    let start = offset.map(|o| o.saturating_sub(1)).unwrap_or(0);
    let max_lines = limit.unwrap_or(usize::MAX);
    let mut result = Vec::new();
    let mut line_num = 0;
    
    while let Some(line) = lines.next_line().await? {
        line_num += 1;
        if line_num <= start {
            continue;
        }
        if result.len() >= max_lines {
            break;
        }
        result.push(format!("{}: {}", line_num, line));
    }
    
    let total = line_num;
    let end = (start + result.len()).min(total);
    
    let content = result.join("\n");
    let summary = if start > 0 || end < total {
        format!("\n[Lines {}-{} of {}]", start + 1, end, total)
    } else {
        format!("\n[{} lines total]", total)
    };
    
    Ok(format!("{}{}", content, summary))
}
