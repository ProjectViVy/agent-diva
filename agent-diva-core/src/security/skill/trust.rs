//! Skill trust tier classification.
//!
//! Defines the trust hierarchy for skill provenance and controls
//! whether a skill may be auto-injected into the agent context or
//! requires explicit user approval.

use serde::{Deserialize, Serialize};

use super::Provenance;

/// Trust tier derived from a skill's provenance marker.
///
/// | Tier        | Auto-inject | Requires approval |
/// |-------------|-------------|-------------------|
/// | Trusted     | yes         | no                |
/// | Review      | no          | yes               |
/// | Untrusted   | no          | yes               |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    /// Skill carries a verified trusted provenance marker.
    Trusted,
    /// Skill originates from the local workspace but lacks a trusted marker.
    Review,
    /// Skill has no provenance information or failed verification.
    Untrusted,
}

impl TrustTier {
    /// Returns `true` if this tier requires explicit user approval before
    /// the skill can be loaded or injected.
    pub fn requires_approval(&self) -> bool {
        matches!(self, Self::Review | Self::Untrusted)
    }

    /// Returns `true` if skills at this tier may be auto-injected into the
    /// agent context without prompting the user.
    pub fn auto_inject(&self) -> bool {
        matches!(self, Self::Trusted)
    }

    /// Derive a trust tier from a [`Provenance`] value.
    pub fn from_provenance(p: &Provenance) -> Self {
        match p {
            Provenance::Trusted => Self::Trusted,
            Provenance::Review => Self::Review,
            Provenance::Untrusted => Self::Untrusted,
        }
    }
}

impl std::fmt::Display for TrustTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Trusted => write!(f, "trusted"),
            Self::Review => write!(f, "review"),
            Self::Untrusted => write!(f, "untrusted"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trusted_does_not_require_approval() {
        assert!(!TrustTier::Trusted.requires_approval());
        assert!(TrustTier::Trusted.auto_inject());
    }

    #[test]
    fn test_review_requires_approval() {
        assert!(TrustTier::Review.requires_approval());
        assert!(!TrustTier::Review.auto_inject());
    }

    #[test]
    fn test_untrusted_requires_approval() {
        assert!(TrustTier::Untrusted.requires_approval());
        assert!(!TrustTier::Untrusted.auto_inject());
    }

    #[test]
    fn test_from_provenance() {
        assert_eq!(
            TrustTier::from_provenance(&Provenance::Trusted),
            TrustTier::Trusted
        );
        assert_eq!(
            TrustTier::from_provenance(&Provenance::Review),
            TrustTier::Review
        );
        assert_eq!(
            TrustTier::from_provenance(&Provenance::Untrusted),
            TrustTier::Untrusted
        );
    }

    #[test]
    fn test_display() {
        assert_eq!(TrustTier::Trusted.to_string(), "trusted");
        assert_eq!(TrustTier::Review.to_string(), "review");
        assert_eq!(TrustTier::Untrusted.to_string(), "untrusted");
    }
}
