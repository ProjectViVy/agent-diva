//! Provider-neutral context assembly contracts.
//!
//! C1-0 deliberately does not route production prompt assembly through these
//! types yet. It freezes the logical section vocabulary and ordering that C1
//! will adopt while characterization tests protect the existing wire shape.

/// Stability class used when deciding whether a section may participate in
/// the prompt-cache prefix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SectionStability {
    /// Content is invariant across sessions unless the binary changes.
    Stable,
    /// Content is captured for a session and changes only after invalidation.
    SessionStable,
    /// Content may change for every provider call.
    TurnVolatile,
}

/// Logical sections in the fixed provider-neutral assembly order.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ContextSection {
    MaskAndIdentity = 0,
    FrozenCore = 1,
    AgentRulesAndSkills = 2,
    MemoryPolicyAndIndex = 3,
    Compaction = 4,
    History = 5,
    WorkingMemory = 6,
    PrefetchRecall = 7,
    VolatileMeta = 8,
    PlanGuard = 9,
    CurrentUser = 10,
}

impl ContextSection {
    /// Stability required by the C1 target contract.
    pub const fn stability(self) -> SectionStability {
        match self {
            Self::MaskAndIdentity
            | Self::FrozenCore
            | Self::AgentRulesAndSkills
            | Self::MemoryPolicyAndIndex => SectionStability::SessionStable,
            Self::Compaction
            | Self::History
            | Self::WorkingMemory
            | Self::PrefetchRecall
            | Self::VolatileMeta
            | Self::PlanGuard
            | Self::CurrentUser => SectionStability::TurnVolatile,
        }
    }

    /// Whether this section belongs to the stable prompt-cache prefix.
    pub const fn is_prefix(self) -> bool {
        matches!(
            self,
            Self::MaskAndIdentity
                | Self::FrozenCore
                | Self::AgentRulesAndSkills
                | Self::MemoryPolicyAndIndex
        )
    }
}

/// Fixed logical order that C1 serialization must preserve.
pub const CONTEXT_SECTION_ORDER: [ContextSection; 11] = [
    ContextSection::MaskAndIdentity,
    ContextSection::FrozenCore,
    ContextSection::AgentRulesAndSkills,
    ContextSection::MemoryPolicyAndIndex,
    ContextSection::Compaction,
    ContextSection::History,
    ContextSection::WorkingMemory,
    ContextSection::PrefetchRecall,
    ContextSection::VolatileMeta,
    ContextSection::PlanGuard,
    ContextSection::CurrentUser,
];

/// Minimal typed section carried into C1 without changing today's serializer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptSection {
    pub section: ContextSection,
    pub stability: SectionStability,
    pub body: String,
    pub cache_break_reason: Option<String>,
}

impl PromptSection {
    /// Create a section using the stability declared by its logical class.
    pub fn new(section: ContextSection, body: impl Into<String>) -> Self {
        Self {
            section,
            stability: section.stability(),
            body: body.into(),
            cache_break_reason: None,
        }
    }

    /// Record why a session-stable prefix section changed.
    pub fn with_cache_break_reason(mut self, reason: impl Into<String>) -> Self {
        self.cache_break_reason = Some(reason.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_order_places_every_prefix_section_before_volatile_sections() {
        let first_non_prefix = CONTEXT_SECTION_ORDER
            .iter()
            .position(|section| !section.is_prefix())
            .expect("contract includes post-prefix sections");

        assert!(CONTEXT_SECTION_ORDER[..first_non_prefix]
            .iter()
            .all(|section| section.is_prefix()));
        assert!(CONTEXT_SECTION_ORDER[first_non_prefix..]
            .iter()
            .all(|section| !section.is_prefix()));
    }

    #[test]
    fn volatile_context_cannot_be_classified_as_cache_prefix() {
        for section in [
            ContextSection::WorkingMemory,
            ContextSection::PrefetchRecall,
            ContextSection::VolatileMeta,
            ContextSection::PlanGuard,
            ContextSection::CurrentUser,
        ] {
            assert_eq!(section.stability(), SectionStability::TurnVolatile);
            assert!(!section.is_prefix());
        }
    }

    #[test]
    fn prefix_change_reason_is_explicit_data() {
        let section = PromptSection::new(ContextSection::MemoryPolicyAndIndex, "L1")
            .with_cache_break_reason("l1_hot_refresh");

        assert_eq!(section.stability, SectionStability::SessionStable);
        assert_eq!(
            section.cache_break_reason.as_deref(),
            Some("l1_hot_refresh")
        );
    }
}
