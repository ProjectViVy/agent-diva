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

pub mod check;
pub mod config;
pub mod decision;
pub mod error;
pub mod injection;
pub mod instruction_hierarchy;
pub mod path;
pub mod pii;
pub mod policy;
pub mod rate_limit;
pub mod skill;
pub mod tool_result_filter;

// Re-export commonly used types
pub use check::check_security;
pub use config::{SecurityConfig, SecurityLevel};
pub use decision::{SecurityContext, SecurityDecision, SecurityFinding, SecurityKind};
pub use error::SecurityError;
pub use injection::{
    detect_injection, detect_tool_output_injection, InjectionAction, InjectionContext,
    InjectionDetection, InjectionKind,
};
pub use instruction_hierarchy::{
    assign_tier, resolve_tier_conflict, tag_message, ConflictResolution, MessageTier, TierConflict,
    TieredMessage,
};
pub use path::PathValidator;
pub use pii::{redact_pii, PiiConfig, PiiKind, PiiMatch, RedactionResult};
// PiiSeverity is re-exported from audit module
pub use crate::audit::PiiSeverity;
pub use policy::{SecurityPolicy, SharedSecurityPolicy};
pub use rate_limit::ActionTracker;
pub use skill::trust::TrustTier;
pub use skill::{
    check_context_budget, should_inject, validate_skill_md, validate_skill_zip_size, Provenance,
    SkillError, MAX_ALWAYS_CHARS, MAX_ALWAYS_COUNT, MAX_SKILL_ZIP_SIZE, MIN_SKILL_MD_CHARS,
};
pub use tool_result_filter::{
    sanitize_tool_output, truncate_tool_result, wrap_untrusted, MAX_TOOL_RESULT_CHARS,
    UNTRUSTED_CLOSE, UNTRUSTED_OPEN,
};
