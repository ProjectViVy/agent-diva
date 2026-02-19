//! Filesystem tools

use crate::base::{Result, Tool};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Resolve path and optionally enforce directory restriction
fn resolve_path(path: &str, allowed_dir: Option<&PathBuf>) -> std::result::Result<PathBuf, String> {
    let resolved = PathBuf::from(path)
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;

    if let Some(allowed) = allowed_dir {
        let allowed_canonical = allowed
            .canonicalize()
            .map_err(|e| format!("Failed to resolve allowed directory: {}", e))?;
        if !resolved.starts_with(&allowed_canonical) {
            return Err(format!(
                "Path {:?} is outside allowed directory {:?}",
                path, allowed
            ));
        }
    }

    Ok(resolved)
}

/// Read file tool
pub struct ReadFileTool {
    allowed_dir: Option<PathBuf>,
}

impl ReadFileTool {
    /// Create a new read file tool
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file at the given path."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to read"
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

        match resolve_path(path, self.allowed_dir.as_ref()) {
            Ok(file_path) => {
                if !file_path.exists() {
                    return Ok(format!("Error: File not found: {}", path));
                }
                if !file_path.is_file() {
                    return Ok(format!("Error: Not a file: {}", path));
                }

                match std::fs::read_to_string(&file_path) {
                    Ok(content) => Ok(content),
                    Err(e) => Ok(format!("Error reading file: {}", e)),
                }
            }
            Err(e) => Ok(format!("Error: {}", e)),
        }
    }
}

/// Write file tool
pub struct WriteFileTool {
    allowed_dir: Option<PathBuf>,
}

impl WriteFileTool {
    /// Create a new write file tool
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

impl Default for WriteFileTool {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file at the given path. Creates parent directories if needed."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to write to"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write"
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

        // For write operations, we need to resolve the parent directory first
        let file_path = PathBuf::from(path);
        let parent = file_path.parent().unwrap_or(Path::new("."));

        // Check parent directory permission
        if let Some(allowed) = &self.allowed_dir {
            let parent_canonical = match parent.canonicalize() {
                Ok(p) => p,
                Err(_) => {
                    // Parent doesn't exist yet, check if it would be allowed
                    let allowed_canonical = match allowed.canonicalize() {
                        Ok(a) => a,
                        Err(e) => return Ok(format!("Error resolving allowed directory: {}", e)),
                    };
                    let parent_abs = match std::env::current_dir() {
                        Ok(cwd) => cwd.join(parent),
                        Err(e) => return Ok(format!("Error getting current directory: {}", e)),
                    };
                    if !parent_abs.starts_with(&allowed_canonical) {
                        return Ok(format!(
                            "Error: Path {:?} is outside allowed directory {:?}",
                            path, allowed
                        ));
                    }
                    parent_abs
                }
            };

            let allowed_canonical = match allowed.canonicalize() {
                Ok(a) => a,
                Err(e) => return Ok(format!("Error resolving allowed directory: {}", e)),
            };

            if !parent_canonical.starts_with(&allowed_canonical) {
                return Ok(format!(
                    "Error: Path {:?} is outside allowed directory {:?}",
                    path, allowed
                ));
            }
        }

        // Create parent directories
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Ok(format!("Error creating parent directories: {}", e));
        }

        // Write file
        match std::fs::write(&file_path, content) {
            Ok(_) => Ok(format!(
                "Successfully wrote {} bytes to {}",
                content.len(),
                path
            )),
            Err(e) => Ok(format!("Error writing file: {}", e)),
        }
    }
}

/// Edit file tool
pub struct EditFileTool {
    allowed_dir: Option<PathBuf>,
}

impl EditFileTool {
    /// Create a new edit file tool
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

impl Default for EditFileTool {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> &str {
        "edit_file"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing old_text with new_text. The old_text must exist exactly in the file."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to edit"
                },
                "old_text": {
                    "type": "string",
                    "description": "The exact text to find and replace"
                },
                "new_text": {
                    "type": "string",
                    "description": "The text to replace with"
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

        match resolve_path(path, self.allowed_dir.as_ref()) {
            Ok(file_path) => {
                if !file_path.exists() {
                    return Ok(format!("Error: File not found: {}", path));
                }

                let content = match std::fs::read_to_string(&file_path) {
                    Ok(c) => c,
                    Err(e) => return Ok(format!("Error reading file: {}", e)),
                };

                if !content.contains(old_text) {
                    return Ok(
                        "Error: old_text not found in file. Make sure it matches exactly."
                            .to_string(),
                    );
                }

                // Count occurrences
                let count = content.matches(old_text).count();
                if count > 1 {
                    return Ok(format!(
                        "Warning: old_text appears {} times. Please provide more context to make it unique.",
                        count
                    ));
                }

                let new_content = content.replacen(old_text, new_text, 1);

                match std::fs::write(&file_path, new_content) {
                    Ok(_) => Ok(format!("Successfully edited {}", path)),
                    Err(e) => Ok(format!("Error writing file: {}", e)),
                }
            }
            Err(e) => Ok(format!("Error: {}", e)),
        }
    }
}

/// List directory tool
pub struct ListDirTool {
    allowed_dir: Option<PathBuf>,
}

impl ListDirTool {
    /// Create a new list dir tool
    pub fn new(allowed_dir: Option<PathBuf>) -> Self {
        Self { allowed_dir }
    }
}

impl Default for ListDirTool {
    fn default() -> Self {
        Self::new(None)
    }
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List the contents of a directory."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The directory path to list"
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

        match resolve_path(path, self.allowed_dir.as_ref()) {
            Ok(dir_path) => {
                if !dir_path.exists() {
                    return Ok(format!("Error: Directory not found: {}", path));
                }
                if !dir_path.is_dir() {
                    return Ok(format!("Error: Not a directory: {}", path));
                }

                match std::fs::read_dir(&dir_path) {
                    Ok(entries) => {
                        let mut items: Vec<_> = entries
                            .filter_map(|e| e.ok())
                            .map(|e| {
                                let path = e.path();
                                let prefix = if path.is_dir() { "📁 " } else { "📄 " };
                                format!(
                                    "{}{}",
                                    prefix,
                                    path.file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("<invalid>")
                                )
                            })
                            .collect();

                        if items.is_empty() {
                            return Ok(format!("Directory {} is empty", path));
                        }

                        items.sort();
                        Ok(items.join("\n"))
                    }
                    Err(e) => Ok(format!("Error reading directory: {}", e)),
                }
            }
            Err(e) => Ok(format!("Error: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!").unwrap();

        let tool = ReadFileTool::new(Some(temp_dir.path().to_path_buf()));
        let params = json!({ "path": file_path.to_str().unwrap() });
        let result = tool.execute(params).await.unwrap();

        assert_eq!(result, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let tool = WriteFileTool::new(Some(temp_dir.path().to_path_buf()));
        let params = json!({
            "path": file_path.to_str().unwrap(),
            "content": "Test content"
        });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("Successfully wrote"));
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "Test content");
    }

    #[tokio::test]
    async fn test_edit_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "Hello, World!").unwrap();

        let tool = EditFileTool::new(Some(temp_dir.path().to_path_buf()));
        let params = json!({
            "path": file_path.to_str().unwrap(),
            "old_text": "World",
            "new_text": "Rust"
        });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("Successfully edited"));
        assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "Hello, Rust!");
    }

    #[tokio::test]
    async fn test_list_dir() {
        let temp_dir = TempDir::new().unwrap();
        std::fs::write(temp_dir.path().join("file1.txt"), "").unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), "").unwrap();
        std::fs::create_dir(temp_dir.path().join("subdir")).unwrap();

        let tool = ListDirTool::new(Some(temp_dir.path().to_path_buf()));
        let params = json!({ "path": temp_dir.path().to_str().unwrap() });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("📄 file1.txt"));
        assert!(result.contains("📄 file2.txt"));
        assert!(result.contains("📁 subdir"));
    }

    #[tokio::test]
    async fn test_path_restriction() {
        let temp_dir = TempDir::new().unwrap();
        let outside_file = std::env::temp_dir().join("outside.txt");
        std::fs::write(&outside_file, "Outside content").unwrap();

        let tool = ReadFileTool::new(Some(temp_dir.path().to_path_buf()));
        let params = json!({ "path": outside_file.to_str().unwrap() });
        let result = tool.execute(params).await.unwrap();

        assert!(result.contains("outside allowed directory"));

        // Cleanup
        std::fs::remove_file(&outside_file).ok();
    }
}
