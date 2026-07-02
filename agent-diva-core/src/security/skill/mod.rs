//! Skill security validation module.
//!
//! Enforces size limits, content requirements, context budgets, and
//! provenance-based trust decisions for the skill system. Skills that
//! fail validation are rejected before they can influence the agent
//! context.
//!
//! # Key constraints
//!
//! | Constraint          | Default  | Purpose                              |
//! |---------------------|----------|--------------------------------------|
//! | `MAX_SKILL_ZIP_SIZE`| 5 MB     | Prevent oversized skill archives     |
//! | `MIN_SKILL_MD_CHARS`| 100      | Ensure SKILL.md has useful content   |
//! | `MAX_ALWAYS_CHARS`  | 8 000    | Cap total always-inject character    |
//! | `MAX_ALWAYS_COUNT`  | 3        | Cap number of always-inject skills   |

pub mod trust;

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum allowed size for a skill ZIP archive (5 MB).
pub const MAX_SKILL_ZIP_SIZE: u64 = 5 * 1024 * 1024;

/// Minimum character count for a valid SKILL.md file.
pub const MIN_SKILL_MD_CHARS: usize = 100;

/// Maximum total character budget for always-inject skill content.
pub const MAX_ALWAYS_CHARS: usize = 8_000;

/// Maximum number of skills that may be marked `always` inject.
pub const MAX_ALWAYS_COUNT: usize = 3;

// ---------------------------------------------------------------------------
// Data models
// ---------------------------------------------------------------------------

/// Provenance marker attached to a skill archive or manifest.
///
/// The provenance determines the trust tier and whether auto-injection
/// is permitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    /// Signed or verified-by-checksum skill from a trusted registry.
    Trusted,
    /// Skill found in the local workspace but without a trusted marker.
    Review,
    /// No provenance information available or verification failed.
    Untrusted,
}

/// Errors that can occur during skill validation.
#[derive(Debug, Clone, thiserror::Error)]
pub enum SkillError {
    /// Skill ZIP archive exceeds the maximum allowed size.
    #[error("skill zip too large: {size} bytes (max {max} bytes)")]
    ZipTooLarge { size: u64, max: u64 },

    /// SKILL.md content is missing or too short.
    #[error("invalid SKILL.md: {reason}")]
    InvalidSkillMd { reason: String },

    /// Skill requires manual review/approval before loading.
    #[error("skill requires review: {name}")]
    ReviewRequired { name: String },

    /// The combined context budget for always-inject skills is exceeded.
    #[error("context budget exceeded: {current} (max {max}) — {detail}")]
    ContextBudgetExceeded {
        current: usize,
        max: usize,
        detail: String,
    },
}

// ---------------------------------------------------------------------------
// Validation functions
// ---------------------------------------------------------------------------

/// Validate that a skill ZIP archive does not exceed the size limit.
pub fn validate_skill_zip_size(size: u64) -> Result<(), SkillError> {
    if size > MAX_SKILL_ZIP_SIZE {
        crate::audit::emit(crate::audit::AuditEvent::SkillRejected {
            skill_name: "unknown".to_string(),
            reason: format!("zip too large: {} bytes (max {})", size, MAX_SKILL_ZIP_SIZE),
        });
        return Err(SkillError::ZipTooLarge {
            size,
            max: MAX_SKILL_ZIP_SIZE,
        });
    }
    Ok(())
}

/// Validate that SKILL.md content meets the minimum character requirement.
pub fn validate_skill_md(content: &str) -> Result<(), SkillError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        crate::audit::emit(crate::audit::AuditEvent::SkillRejected {
            skill_name: "unknown".to_string(),
            reason: "SKILL.md is empty".to_string(),
        });
        return Err(SkillError::InvalidSkillMd {
            reason: "SKILL.md is empty".to_string(),
        });
    }
    if trimmed.len() < MIN_SKILL_MD_CHARS {
        crate::audit::emit(crate::audit::AuditEvent::SkillRejected {
            skill_name: "unknown".to_string(),
            reason: format!("SKILL.md too short: {} chars (min {})", trimmed.len(), MIN_SKILL_MD_CHARS),
        });
        return Err(SkillError::InvalidSkillMd {
            reason: format!(
                "SKILL.md too short: {} chars (min {})",
                trimmed.len(),
                MIN_SKILL_MD_CHARS
            ),
        });
    }
    crate::audit::emit(crate::audit::AuditEvent::SkillLoaded {
        skill_name: "unknown".to_string(),
        trust_tier: "trusted".to_string(),
        provenance: "workspace".to_string(),
    });
    Ok(())
}

/// Check whether the always-inject context budget is still within limits.
///
/// Callers should supply the **total** character count across all skills
/// marked `always` and the **count** of such skills.
pub fn check_context_budget(chars: usize, count: usize) -> Result<(), SkillError> {
    if count > MAX_ALWAYS_COUNT {
        return Err(SkillError::ContextBudgetExceeded {
            current: count,
            max: MAX_ALWAYS_COUNT,
            detail: format!("{} always-inject skills (max {})", count, MAX_ALWAYS_COUNT),
        });
    }
    if chars > MAX_ALWAYS_CHARS {
        return Err(SkillError::ContextBudgetExceeded {
            current: chars,
            max: MAX_ALWAYS_CHARS,
            detail: format!(
                "{} total always-inject chars (max {})",
                chars, MAX_ALWAYS_CHARS
            ),
        });
    }
    Ok(())
}

/// Determine whether a skill with the given provenance may be auto-injected.
///
/// Only [`Provenance::Trusted`] skills are auto-injected; `Review` and
/// `Untrusted` skills require explicit approval.
pub fn should_inject(provenance: &Provenance) -> bool {
    matches!(provenance, Provenance::Trusted)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- zip size -----------------------------------------------------------

    #[test]
    fn test_zip_size_ok() {
        assert!(validate_skill_zip_size(1024 * 1024).is_ok()); // 1 MB
    }

    #[test]
    fn test_zip_size_too_large() {
        let result = validate_skill_zip_size(6 * 1024 * 1024); // 6 MB
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, SkillError::ZipTooLarge { .. }));
    }

    // -- SKILL.md -----------------------------------------------------------

    #[test]
    fn test_skill_md_empty() {
        let result = validate_skill_md("");
        assert!(result.is_err());
        assert!(matches!(result, Err(SkillError::InvalidSkillMd { .. })));
    }

    #[test]
    fn test_skill_md_too_short() {
        let result = validate_skill_md("hi");
        assert!(result.is_err());
    }

    #[test]
    fn test_skill_md_valid() {
        let content = "x".repeat(150);
        assert!(validate_skill_md(&content).is_ok());
    }

    // -- provenance ---------------------------------------------------------

    #[test]
    fn test_provenance_no_marker_is_untrusted() {
        // No marker → Untrusted
        let p = Provenance::Untrusted;
        assert!(!should_inject(&p));
    }

    #[test]
    fn test_provenance_workspace_is_review() {
        let p = Provenance::Review;
        assert!(!should_inject(&p));
    }

    #[test]
    fn test_provenance_trusted_injects() {
        let p = Provenance::Trusted;
        assert!(should_inject(&p));
    }

    // -- context budget -----------------------------------------------------

    #[test]
    fn test_context_budget_exceeds_count() {
        // 4 always-skills → exceeds max of 3
        let result = check_context_budget(500, 4);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(SkillError::ContextBudgetExceeded { .. })
        ));
    }

    #[test]
    fn test_context_budget_exceeds_chars() {
        // 9000 chars → exceeds max of 8000
        let result = check_context_budget(9000, 2);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(SkillError::ContextBudgetExceeded { .. })
        ));
    }

    #[test]
    fn test_context_budget_ok() {
        assert!(check_context_budget(3000, 2).is_ok());
    }

    // -- should_inject ------------------------------------------------------

    #[test]
    fn test_should_inject_trusted() {
        assert!(should_inject(&Provenance::Trusted));
    }

    #[test]
    fn test_should_inject_review() {
        assert!(!should_inject(&Provenance::Review));
    }

    #[test]
    fn test_should_inject_untrusted() {
        assert!(!should_inject(&Provenance::Untrusted));
    }
}
