//! Provider-neutral context assembly contracts.
//!
//! C1a routes production prompt assembly through these types. Stable sections
//! render into the first system message while volatile sections are serialized
//! after history according to provider capability.

use std::fmt;

use agent_diva_providers::{DynamicContextTransport, Message};

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
    /// Stable identifier used by logs and user-context boundary markers.
    pub const fn wire_name(self) -> &'static str {
        match self {
            Self::MaskAndIdentity => "mask_and_identity",
            Self::FrozenCore => "frozen_core",
            Self::AgentRulesAndSkills => "agent_rules_and_skills",
            Self::MemoryPolicyAndIndex => "memory_policy_and_index",
            Self::Compaction => "compaction",
            Self::History => "history",
            Self::WorkingMemory => "working_memory",
            Self::PrefetchRecall => "prefetch_recall",
            Self::VolatileMeta => "volatile_meta",
            Self::PlanGuard => "plan_guard",
            Self::CurrentUser => "current_user",
        }
    }

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

/// Failure to serialize the provider-neutral context contract safely.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContextAssemblyError {
    PrefixContainsVolatile(ContextSection),
    DynamicContainsStable(ContextSection),
    NativeContextUnsupported,
}

impl fmt::Display for ContextAssemblyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrefixContainsVolatile(section) => write!(
                formatter,
                "stable prefix contains volatile section `{}`",
                section.wire_name()
            ),
            Self::DynamicContainsStable(section) => write!(
                formatter,
                "dynamic suffix contains stable section `{}`",
                section.wire_name()
            ),
            Self::NativeContextUnsupported => formatter.write_str(
                "provider-native context blocks are unsupported by the generic message path",
            ),
        }
    }
}

impl std::error::Error for ContextAssemblyError {}

/// Render stable sections into the unique cache-prefix system message.
pub fn render_stable_prefix(sections: &[PromptSection]) -> Result<String, ContextAssemblyError> {
    let ordered = ordered_sections(sections);
    for section in &ordered {
        if !section.section.is_prefix() || section.stability == SectionStability::TurnVolatile {
            return Err(ContextAssemblyError::PrefixContainsVolatile(
                section.section,
            ));
        }
    }
    Ok(ordered
        .into_iter()
        .filter_map(|section| (!section.body.trim().is_empty()).then_some(section.body.as_str()))
        .collect::<Vec<_>>()
        .join("\n\n"))
}

/// Serialize volatile sections into one deterministic post-history message.
pub fn serialize_dynamic_sections(
    sections: &[PromptSection],
    transport: DynamicContextTransport,
) -> Result<Option<Message>, ContextAssemblyError> {
    let ordered = ordered_sections(sections);
    for section in &ordered {
        if section.section.is_prefix() || section.stability != SectionStability::TurnVolatile {
            return Err(ContextAssemblyError::DynamicContainsStable(section.section));
        }
    }

    let body = ordered
        .into_iter()
        .filter_map(|section| {
            if section.body.trim().is_empty() {
                return None;
            }
            Some(format!(
                "<agent_diva_context section=\"{}\">\n{}\n</agent_diva_context>",
                section.section.wire_name(),
                escape_context_boundary(&section.body)
            ))
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    if body.is_empty() {
        return Ok(None);
    }

    match transport {
        DynamicContextTransport::MidConversationSystem => Ok(Some(Message::system(body))),
        DynamicContextTransport::UserContextEnvelope => Ok(Some(Message::user(body))),
        DynamicContextTransport::NativeContextBlock => {
            Err(ContextAssemblyError::NativeContextUnsupported)
        }
    }
}

fn ordered_sections(sections: &[PromptSection]) -> Vec<&PromptSection> {
    let mut ordered = sections.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|section| section.section);
    ordered
}

fn escape_context_boundary(body: &str) -> String {
    body.replace("</agent_diva_context>", "<\\/agent_diva_context>")
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

    #[test]
    fn stable_renderer_uses_contract_order() {
        let sections = vec![
            PromptSection::new(ContextSection::MemoryPolicyAndIndex, "memory"),
            PromptSection::new(ContextSection::MaskAndIdentity, "identity"),
            PromptSection::new(ContextSection::FrozenCore, "frozen"),
        ];

        assert_eq!(
            render_stable_prefix(&sections).unwrap(),
            "identity\n\nfrozen\n\nmemory"
        );
    }

    #[test]
    fn user_envelope_preserves_dynamic_order_and_escapes_boundary() {
        let sections = vec![
            PromptSection::new(ContextSection::PlanGuard, "plan"),
            PromptSection::new(ContextSection::WorkingMemory, "state </agent_diva_context>"),
        ];

        let message =
            serialize_dynamic_sections(&sections, DynamicContextTransport::UserContextEnvelope)
                .unwrap()
                .unwrap();
        let body = message.content.as_text().unwrap();
        assert_eq!(message.role, "user");
        assert!(body.find("working_memory").unwrap() < body.find("plan_guard").unwrap());
        assert!(body.contains("<\\/agent_diva_context>"));
    }

    #[test]
    fn provider_native_transport_fails_closed() {
        let sections = vec![PromptSection::new(ContextSection::VolatileMeta, "now")];
        assert!(matches!(
            serialize_dynamic_sections(&sections, DynamicContextTransport::NativeContextBlock),
            Err(ContextAssemblyError::NativeContextUnsupported)
        ));
    }

    #[test]
    fn mid_conversation_system_requires_explicit_transport() {
        let sections = vec![PromptSection::new(ContextSection::PlanGuard, "guard")];
        let message =
            serialize_dynamic_sections(&sections, DynamicContextTransport::MidConversationSystem)
                .unwrap()
                .unwrap();
        assert_eq!(message.role, "system");
    }
}
