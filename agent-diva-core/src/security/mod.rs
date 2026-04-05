//! Security module for agent-diva
//!
//! Provides comprehensive security features including:
//! - Path validation and sanitization
//! - Rate limiting for file operations
//! - Security policy configuration
//! - Security error types
//!
//! # Example
//!
//! ```rust
//! use agent_diva_core::security::{SecurityPolicy, SecurityLevel};
//! use std::path::PathBuf;
//!
//! // Create a security policy with standard settings
//! let policy = SecurityPolicy::from_level(
//!     PathBuf::from("/workspace"),
//!     SecurityLevel::Standard,
//! );
//!
//! // Validate a path
//! assert!(policy.is_path_allowed("src/main.rs").is_ok());
//! assert!(policy.is_path_allowed("../etc/passwd").is_err());
//! ```

pub mod config;
pub mod error;
pub mod path;
pub mod policy;
pub mod rate_limit;

// Re-export commonly used types
pub use config::{SecurityConfig, SecurityLevel};
pub use error::SecurityError;
pub use path::PathValidator;
pub use policy::{SecurityPolicy, SharedSecurityPolicy};
pub use rate_limit::ActionTracker;
