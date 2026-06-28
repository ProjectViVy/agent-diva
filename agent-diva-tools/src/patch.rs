//! Patch tool with multi-strategy matching and security checks.

#[path = "patch_apply.rs"]
mod patch_apply;
#[path = "patch_compare.rs"]
mod patch_compare;
#[path = "patch_diff.rs"]
mod patch_diff;
#[cfg(test)]
#[path = "patch_tests.rs"]
mod patch_tests;
#[path = "patch_types.rs"]
mod patch_types;

use agent_diva_core::security::{SecurityPolicy, SharedSecurityPolicy};
use agent_diva_tooling::{Result, Tool};
use async_trait::async_trait;
use patch_apply::apply_patch;
use patch_diff::format_success;
use patch_types::PatchRequest;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

/// Patch file tool with workspace-aware security checks.
pub struct PatchTool {
    security: SharedSecurityPolicy,
}

impl PatchTool {
    /// Create a new patch tool with a security policy.
    pub fn new(security: SharedSecurityPolicy) -> Self {
        Self { security }
    }

    /// Create a new patch tool with default policy for a workspace.
    pub fn for_workspace(workspace: PathBuf) -> Self {
        let policy = Arc::new(SecurityPolicy::new(workspace));
        Self::new(policy)
    }
}

impl Default for PatchTool {
    fn default() -> Self {
        Self::for_workspace(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[async_trait]
impl Tool for PatchTool {
    fn name(&self) -> &str {
        "patch"
    }

    fn description(&self) -> &str {
        "Apply a targeted patch to a text file using one of nine matching strategies and return a diff."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File path relative to the workspace."
                },
                "old_text": {
                    "type": "string",
                    "description": "The original text or pattern to match."
                },
                "new_text": {
                    "type": "string",
                    "description": "The replacement text."
                },
                "match_strategy": {
                    "type": "string",
                    "enum": [
                        "Exact",
                        "TrimWhitespace",
                        "NormalizeWhitespace",
                        "CaseInsensitive",
                        "IndentTolerance",
                        "Fuzzy",
                        "LineBased",
                        "PartialMatch",
                        "Regex"
                    ],
                    "description": "Match strategy. Defaults to Exact."
                }
            },
            "required": ["path", "old_text", "new_text"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String> {
        let request = match serde_json::from_value::<PatchRequest>(params) {
            Ok(request) => request,
            Err(error) => return Ok(format!("Error: Invalid patch arguments: {}", error)),
        };

        if request.old_text.is_empty() {
            return Ok("Error: old_text must not be empty".to_string());
        }

        if request.old_text == request.new_text {
            return Ok("Error: old_text and new_text must differ".to_string());
        }

        if let Err(error) = self.security.can_act() {
            return Ok(format!("Error: {}", error.user_message()));
        }

        let resolved_path = match self.security.validate_path(&request.path).await {
            Ok(path) => path,
            Err(error) => return Ok(format!("Error: {}", error.user_message())),
        };

        let metadata = match tokio::fs::metadata(&resolved_path).await {
            Ok(metadata) => metadata,
            Err(error) => {
                return Ok(format!(
                    "Error: File not found: {} ({})",
                    request.path, error
                ))
            }
        };

        if !metadata.is_file() {
            return Ok(format!("Error: Not a file: {}", request.path));
        }

        if let Err(error) = self.security.check_file_size(metadata.len()) {
            return Ok(format!("Error: {}", error.user_message()));
        }

        let original_content = match tokio::fs::read_to_string(&resolved_path).await {
            Ok(content) => content,
            Err(error) => return Ok(format!("Error reading file: {}", error)),
        };

        let success = match apply_patch(&original_content, &request) {
            Ok(success) => success,
            Err(message) => return Ok(format!("Error: {}", message)),
        };

        match tokio::fs::write(&resolved_path, &success.new_content).await {
            Ok(_) => Ok(format_success(&request.path, &success)),
            Err(error) => Ok(format!("Error writing file: {}", error)),
        }
    }
}
