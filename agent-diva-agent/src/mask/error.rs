//! Error types for the mask system.

use thiserror::Error;

/// Errors that can occur when loading or parsing mask files.
#[derive(Debug, Error)]
pub enum MaskError {
    /// The requested mask file was not found on disk.
    #[error("mask file not found: {path}")]
    NotFound { path: String },

    /// No mask with the given name exists in the registry.
    #[error("mask not found: \"{name}\"")]
    MaskNotFound { name: String },

    /// The file exists but cannot be read or is not valid UTF-8.
    #[error("invalid mask file: {path} — {reason}")]
    InvalidFile { path: String, reason: String },

    /// The YAML frontmatter is malformed or missing required fields.
    #[error("invalid frontmatter in {path}: {reason}")]
    InvalidFrontmatter { path: String, reason: String },
}
