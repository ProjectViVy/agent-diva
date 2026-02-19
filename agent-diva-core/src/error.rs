//! Error types for agent-diva

use thiserror::Error;

/// The main error type for agent-diva operations
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Session management errors
    #[error("Session error: {0}")]
    Session(String),

    /// Channel communication errors
    #[error("Channel error: {0}")]
    Channel(String),

    /// Provider (LLM) errors
    #[error("Provider error: {0}")]
    Provider(String),

    /// Tool execution errors
    #[error("Tool error: {0}")]
    Tool(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Not found errors
    #[error("Not found: {0}")]
    NotFound(String),

    /// Unauthorized access
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Internal errors
    #[error("Internal error: {0}")]
    Internal(String),
}

/// A specialized Result type for agent-diva operations
pub type Result<T> = std::result::Result<T, Error>;

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Serialization(e.to_string())
    }
}

impl From<config::ConfigError> for Error {
    fn from(e: config::ConfigError) -> Self {
        Error::Config(e.to_string())
    }
}
