//! Security policy - unified security management
//!
//! This module provides the main SecurityPolicy struct that integrates
//! configuration, path validation, and rate limiting into a cohesive
//! security framework.

use crate::security::config::{SecurityConfig, SecurityLevel};
use crate::security::error::SecurityError;
use crate::security::path::PathValidator;
use crate::security::rate_limit::ActionTracker;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Unified security policy that coordinates all security checks
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Security configuration
    config: SecurityConfig,
    /// Action tracker for rate limiting
    tracker: ActionTracker,
    /// Workspace directory (base path for relative paths)
    workspace_dir: PathBuf,
}

impl SecurityPolicy {
    /// Create a new security policy with default configuration
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            config: SecurityConfig::default(),
            tracker: ActionTracker::new(),
            workspace_dir,
        }
    }

    /// Create a new security policy with custom configuration
    pub fn with_config(workspace_dir: PathBuf, config: SecurityConfig) -> Self {
        Self {
            config,
            tracker: ActionTracker::new(),
            workspace_dir,
        }
    }

    /// Create a new security policy from a security level preset
    pub fn from_level(workspace_dir: PathBuf, level: SecurityLevel) -> Self {
        Self {
            config: SecurityConfig::from_level(level),
            tracker: ActionTracker::new(),
            workspace_dir,
        }
    }

    /// Get the security configuration
    pub fn config(&self) -> &SecurityConfig {
        &self.config
    }

    /// Get the workspace directory
    pub fn workspace_dir(&self) -> &Path {
        &self.workspace_dir
    }

    /// Check if read-only mode is enabled
    pub fn is_read_only(&self) -> bool {
        self.config.is_read_only()
    }

    /// Check if shell access is allowed (always false in standard mode)
    pub fn has_shell_access(&self) -> bool {
        // Shell access is determined by security level
        matches!(self.config.level, SecurityLevel::Permissive)
    }

    // ==================== Path Validation ====================

    /// Layer 1-5: Basic path validation (before resolution)
    ///
    /// Checks for:
    /// - Null bytes
    /// - Path traversal (../)
    /// - URL-encoded traversal
    /// - Tilde expansion
    /// - Forbidden prefixes
    pub fn is_path_allowed(&self, path: &str) -> Result<(), SecurityError> {
        // Layer 1: Null-byte detection
        if PathValidator::contains_null_bytes(path) {
            return Err(SecurityError::InvalidPathFormat {
                reason: "Path contains null bytes".to_string(),
            });
        }

        // Layer 2: Path traversal detection
        if PathValidator::contains_path_traversal(path) {
            return Err(SecurityError::ForbiddenComponent {
                component: "parent directory (..)".to_string(),
            });
        }

        // Layer 3: URL-encoded traversal detection
        if PathValidator::contains_url_encoded_traversal(path) {
            return Err(SecurityError::InvalidPathFormat {
                reason: "URL-encoded path traversal detected".to_string(),
            });
        }

        // Layer 4: Tilde expansion check
        if PathValidator::starts_with_tilde(path) {
            return Err(SecurityError::InvalidPathFormat {
                reason: "Tilde expansion is not allowed".to_string(),
            });
        }

        // Layer 5: Absolute path check (when workspace_only is enabled)
        if self.config.workspace_only && PathValidator::is_absolute(path) {
            return Err(SecurityError::InvalidPathFormat {
                reason: "Absolute paths are not allowed in workspace-only mode".to_string(),
            });
        }

        // Layer 6: Forbidden prefix check
        if let Some(prefix) =
            PathValidator::matches_forbidden_prefix(path, &self.config.forbidden_paths)
        {
            return Err(SecurityError::PathNotAllowed {
                path: format!("matches forbidden prefix: {}", prefix),
            });
        }

        // Layer 7: Forbidden extension check
        if let Some(ext) = PathValidator::get_extension(path) {
            if PathValidator::is_extension_forbidden(&ext, &self.config.forbidden_extensions) {
                return Err(SecurityError::ForbiddenExtension { ext });
            }
        }

        Ok(())
    }

    /// Resolve a path relative to the workspace
    pub fn resolve_path(&self, path: &str) -> PathBuf {
        if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.workspace_dir.join(path)
        }
    }

    /// Layer 8: Check if a resolved (canonicalized) path is within allowed roots
    pub fn is_resolved_path_allowed(&self, resolved: &Path) -> bool {
        // Try to canonicalize for comparison
        let resolved_canonical = if let Ok(c) = resolved.canonicalize() {
            c
        } else {
            resolved.to_path_buf()
        };

        // Check workspace directory
        let workspace_canonical = if let Ok(c) = self.workspace_dir.canonicalize() {
            c
        } else {
            self.workspace_dir.clone()
        };

        if resolved_canonical.starts_with(&workspace_canonical) {
            return true;
        }

        // Check additional allowed roots
        PathValidator::is_within_allowed_roots(resolved, &self.config.allowed_roots)
    }

    /// Full path validation: from input to resolved path
    pub async fn validate_path(&self, path: &str) -> Result<PathBuf, SecurityError> {
        // Basic validation
        self.is_path_allowed(path)?;

        // Resolve the path
        let full_path = self.resolve_path(path);

        // Canonicalize and check if it exists
        let resolved = match tokio::fs::canonicalize(&full_path).await {
            Ok(p) => p,
            Err(_) => full_path, // File may not exist yet (for write operations)
        };

        // Check if resolved path is within allowed workspace
        if self.config.workspace_only && !self.is_resolved_path_allowed(&resolved) {
            return Err(SecurityError::PathEscapesWorkspace { resolved });
        }

        // Check symlink restrictions
        if !self.config.allow_symlinks {
            if let Ok(meta) = tokio::fs::symlink_metadata(&resolved).await {
                if meta.file_type().is_symlink() {
                    return Err(SecurityError::SymlinkNotAllowed { path: resolved });
                }
            }
        }

        Ok(resolved)
    }

    /// Validate parent directory for write operations (TOCTOU-safe)
    pub async fn validate_parent_directory(&self, path: &Path) -> Result<PathBuf, SecurityError> {
        let Some(parent) = path.parent() else {
            return Err(SecurityError::InvalidPathFormat {
                reason: "Path has no parent directory".to_string(),
            });
        };

        // Create parent directories if needed
        if let Err(e) = tokio::fs::create_dir_all(parent).await {
            return Err(SecurityError::InvalidPathFormat {
                reason: format!("Failed to create parent directories: {}", e),
            });
        }

        // Canonicalize parent after creation
        let resolved_parent = tokio::fs::canonicalize(parent).await.map_err(|e| {
            SecurityError::InvalidPathFormat {
                reason: format!("Failed to resolve parent directory: {}", e),
            }
        })?;

        // Validate resolved parent is within allowed workspace
        if self.config.workspace_only && !self.is_resolved_path_allowed(&resolved_parent) {
            return Err(SecurityError::PathEscapesWorkspace {
                resolved: resolved_parent,
            });
        }

        Ok(resolved_parent)
    }

    // ==================== Rate Limiting ====================

    /// Check if rate limit is exceeded (without recording)
    pub fn is_rate_limited(&self) -> bool {
        self.tracker
            .is_rate_limited(self.config.max_actions_per_hour)
    }

    /// Record an action and return current count
    pub fn record_action(&self) -> usize {
        self.tracker.record()
    }

    /// Try to record an action, returning false if rate limited
    ///
    /// This is the main method for checking and recording in one step
    pub fn try_record_action(&self) -> Result<(), SecurityError> {
        if !self.tracker.try_record(self.config.max_actions_per_hour) {
            let count = self.tracker.count();
            return Err(SecurityError::RateLimitExceeded {
                count,
                max: self.config.max_actions_per_hour,
            });
        }
        Ok(())
    }

    /// Get current action count in the window
    pub fn action_count(&self) -> usize {
        self.tracker.count()
    }

    /// Check if can perform an action (rate limit + read-only check)
    pub fn can_act(&self) -> Result<(), SecurityError> {
        if self.is_read_only() {
            return Err(SecurityError::ReadOnlyMode);
        }
        self.try_record_action()
    }

    // ==================== File Size ====================

    /// Check if file size is within limits
    pub fn check_file_size(&self, size: u64) -> Result<(), SecurityError> {
        if self.config.max_file_size > 0 && size > self.config.max_file_size {
            Err(SecurityError::FileTooLarge {
                size,
                max_size: self.config.max_file_size,
            })
        } else {
            Ok(())
        }
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

/// Shared security policy (Arc wrapper for thread-safe sharing)
pub type SharedSecurityPolicy = Arc<SecurityPolicy>;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_policy() -> (SecurityPolicy, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let policy = SecurityPolicy::new(temp_dir.path().to_path_buf());
        (policy, temp_dir)
    }

    #[test]
    fn test_path_validation_null_bytes() {
        let (policy, _temp) = create_test_policy();
        assert!(policy.is_path_allowed("/path\0/file").is_err());
    }

    #[test]
    fn test_path_validation_traversal() {
        let (policy, _temp) = create_test_policy();
        assert!(policy.is_path_allowed("../etc/passwd").is_err());
        assert!(policy.is_path_allowed("/path/../file").is_err());
    }

    #[test]
    fn test_rate_limiting() {
        let (policy, _temp) = create_test_policy();

        // Should be able to record actions up to limit
        for _ in 0..policy.config.max_actions_per_hour {
            assert!(policy.try_record_action().is_ok());
        }

        // Next action should fail
        assert!(policy.try_record_action().is_err());
    }

    #[test]
    fn test_read_only_mode() {
        let temp_dir = TempDir::new().unwrap();
        let policy =
            SecurityPolicy::from_level(temp_dir.path().to_path_buf(), SecurityLevel::Paranoid);

        assert!(policy.is_read_only());
        assert!(policy.can_act().is_err());
    }

    #[tokio::test]
    async fn test_validate_path() {
        let temp_dir = TempDir::new().unwrap();
        let policy = SecurityPolicy::new(temp_dir.path().to_path_buf());

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        tokio::fs::write(&test_file, "test").await.unwrap();

        // Should validate successfully
        let result = policy.validate_path("test.txt").await;
        assert!(result.is_ok());

        // Should reject path traversal
        let result = policy.validate_path("../test.txt").await;
        assert!(result.is_err());
    }
}
