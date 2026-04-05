//! Path validation utilities for security policy

use std::path::{Component, Path, PathBuf};

/// Validates paths against security threats
pub struct PathValidator;

impl PathValidator {
    /// Layer 1: Check for null bytes
    pub fn contains_null_bytes(path: &str) -> bool {
        path.contains('\0')
    }

    /// Layer 2: Check for path traversal components (../)
    pub fn contains_path_traversal(path: &str) -> bool {
        Path::new(path)
            .components()
            .any(|c| matches!(c, Component::ParentDir))
    }

    /// Layer 3: Check for URL-encoded traversal
    pub fn contains_url_encoded_traversal(path: &str) -> bool {
        let lower = path.to_lowercase();
        lower.contains("..%2f")
            || lower.contains("%2f..")
            || lower.contains("..%5c")
            || lower.contains("%5c..")
    }

    /// Layer 4: Check for tilde expansion (~user)
    pub fn starts_with_tilde(path: &str) -> bool {
        path.starts_with('~')
    }

    /// Layer 5: Check if path is absolute
    pub fn is_absolute(path: &str) -> bool {
        Path::new(path).is_absolute()
    }

    /// Layer 6: Check against forbidden path prefixes
    pub fn matches_forbidden_prefix(path: &str, forbidden: &[String]) -> Option<String> {
        let normalized = path.to_lowercase().replace('\\', "/");
        for prefix in forbidden {
            let norm_prefix = prefix.to_lowercase().replace('\\', "/");
            if normalized.starts_with(&norm_prefix)
                || normalized.contains(&format!("/{}", norm_prefix))
            {
                return Some(prefix.clone());
            }
        }
        None
    }

    /// Normalize a path for comparison
    pub fn normalize_path(path: &str) -> String {
        path.replace('\\', "/")
            .to_lowercase()
            .trim_start_matches('/')
            .to_string()
    }

    /// Check if a resolved path is within allowed roots
    pub fn is_within_allowed_roots(resolved: &Path, allowed_roots: &[PathBuf]) -> bool {
        // Try to canonicalize for comparison
        let resolved_canonical = if let Ok(c) = resolved.canonicalize() {
            c
        } else {
            resolved.to_path_buf()
        };

        for root in allowed_roots {
            let root_canonical = if let Ok(c) = root.canonicalize() {
                c
            } else {
                root.clone()
            };

            if resolved_canonical.starts_with(&root_canonical) {
                return true;
            }
        }

        false
    }

    /// Validate that a path doesn't escape the workspace via symlinks
    pub async fn validate_no_symlink_escape(path: &Path, workspace: &Path) -> Result<(), String> {
        // Check if the path itself is a symlink
        if let Ok(meta) = tokio::fs::symlink_metadata(path).await {
            if meta.file_type().is_symlink() {
                return Err(format!("Path is a symbolic link: {}", path.display()));
            }
        }

        // Check all parent directories
        let mut current = path.parent();
        while let Some(parent) = current {
            if parent.as_os_str().is_empty() || parent == Path::new("/") {
                break;
            }

            if let Ok(meta) = tokio::fs::symlink_metadata(parent).await {
                if meta.file_type().is_symlink() {
                    // Resolve the symlink and check if it's within workspace
                    let resolved = tokio::fs::canonicalize(parent).await.map_err(|e| {
                        format!("Failed to resolve symlink {}: {}", parent.display(), e)
                    })?;

                    if !resolved.starts_with(workspace) {
                        return Err(format!(
                            "Symlink {} escapes workspace (resolves to {})",
                            parent.display(),
                            resolved.display()
                        ));
                    }
                }
            }

            current = parent.parent();
        }

        Ok(())
    }

    /// Get the file extension from a path
    pub fn get_extension(path: &str) -> Option<String> {
        Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
    }

    /// Check if an extension is in the forbidden list
    pub fn is_extension_forbidden(ext: &str, forbidden: &[String]) -> bool {
        let ext_lower = ext.to_lowercase().trim_start_matches('.').to_string();
        forbidden
            .iter()
            .any(|f| f.to_lowercase().trim_start_matches('.') == ext_lower)
    }

    /// Sanitize a path component for safe use
    pub fn sanitize_component(component: &str) -> String {
        component
            .replace(['/', '\\'], "_")
            .replace('\0', "")
            .replace("..", "_")
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_bytes() {
        assert!(PathValidator::contains_null_bytes("/path\0to/file"));
        assert!(!PathValidator::contains_null_bytes("/path/to/file"));
    }

    #[test]
    fn test_path_traversal() {
        assert!(PathValidator::contains_path_traversal("../etc/passwd"));
        assert!(PathValidator::contains_path_traversal("/path/../file"));
        assert!(!PathValidator::contains_path_traversal("/path/to/file"));
    }

    #[test]
    fn test_url_encoded_traversal() {
        assert!(PathValidator::contains_url_encoded_traversal(
            "..%2fetc/passwd"
        ));
        assert!(PathValidator::contains_url_encoded_traversal(
            "%2f..%5cwindows"
        ));
        assert!(!PathValidator::contains_url_encoded_traversal(
            "/path/to/file"
        ));
    }

    #[test]
    fn test_tilde_expansion() {
        assert!(PathValidator::starts_with_tilde("~/.ssh/id_rsa"));
        assert!(PathValidator::starts_with_tilde("~user/file"));
        assert!(!PathValidator::starts_with_tilde("/home/user/file"));
    }

    #[test]
    fn test_forbidden_prefix() {
        let forbidden = vec!["/etc".to_string(), "/root".to_string()];
        assert!(PathValidator::matches_forbidden_prefix("/etc/passwd", &forbidden).is_some());
        assert!(PathValidator::matches_forbidden_prefix("/root/.bashrc", &forbidden).is_some());
        assert!(PathValidator::matches_forbidden_prefix("/home/user/file", &forbidden).is_none());
    }

    #[test]
    fn test_extension_validation() {
        let forbidden = vec![".exe".to_string(), ".dll".to_string()];
        assert!(PathValidator::is_extension_forbidden("exe", &forbidden));
        assert!(PathValidator::is_extension_forbidden(".exe", &forbidden));
        assert!(!PathValidator::is_extension_forbidden("txt", &forbidden));

        assert_eq!(
            PathValidator::get_extension("/path/to/file.EXE"),
            Some("exe".to_string())
        );
    }
}
