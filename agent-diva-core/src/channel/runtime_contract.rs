//! Runtime contracts shared by the Fabric, Manager and channel adapters.
//!
//! These types are internal Rust contracts rather than an alternative wire
//! schema. `ChannelEnvelopeV1` and the Neuro-Link JSON Schema remain the only
//! transport authority.

use super::{
    ChannelAddress, ChannelContractError, ChannelEnvelopeV1, ChannelHealthStatus, ContentPart,
    Correlation, ReactionOperation, TypingState,
};
use chrono::{DateTime, Utc};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Validated adapter identity used by the Registry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChannelId(String);

impl ChannelId {
    pub fn new(value: impl Into<String>) -> Result<Self, ChannelContractError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ChannelContractError::EmptyField("channel_id"));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ChannelId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Capability atoms used by the static declaration/runtime probe matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChannelCapability {
    IngressText,
    IngressMarkdown,
    IngressThread,
    IngressGroup,
    IngressDirect,
    IngressTypedAttachments,
    IngressDedupId,
    EgressText,
    EgressMarkdown,
    EgressChunking,
    EgressReply,
    EgressImage,
    EgressAudio,
    EgressVideo,
    EgressFile,
    EgressCard,
    InteractionTyping,
    InteractionListening,
    InteractionEdit,
    InteractionDelete,
    InteractionReaction,
    InteractionStreamFinalize,
    ReliabilityHealth,
    ReliabilityHeartbeat,
    ReliabilityResume,
    ReliabilityTokenRefresh,
    ReliabilityPacing,
    ReliabilitySupervisedRestart,
}

/// Adapter limits that participate in command planning and capability display.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChannelLimits {
    pub max_text_chars: Option<usize>,
    pub max_attachment_bytes: Option<u64>,
    pub supported_mime_types: BTreeSet<String>,
    pub rate_limit_hint_ms: Option<u64>,
}

/// Partial runtime overrides for adapter limits.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChannelLimitsProbe {
    pub max_text_chars: Option<usize>,
    pub max_attachment_bytes: Option<u64>,
    pub supported_mime_types: Option<BTreeSet<String>>,
    pub rate_limit_hint_ms: Option<u64>,
}

/// Static or effective adapter capability snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChannelCapabilities {
    pub supported: BTreeSet<ChannelCapability>,
    pub limits: ChannelLimits,
}

impl ChannelCapabilities {
    pub fn new(supported: impl IntoIterator<Item = ChannelCapability>) -> Self {
        Self {
            supported: supported.into_iter().collect(),
            limits: ChannelLimits::default(),
        }
    }

    pub fn supports(&self, capability: ChannelCapability) -> bool {
        self.supported.contains(&capability)
    }

    /// Merge a partial live probe over the code-declared capability snapshot.
    pub fn merge_probe(&self, probe: &ChannelCapabilityProbe) -> Self {
        let mut merged = self.clone();
        for (capability, supported) in &probe.overrides {
            if *supported {
                merged.supported.insert(*capability);
            } else {
                merged.supported.remove(capability);
            }
        }
        if let Some(value) = probe.limits.max_text_chars {
            merged.limits.max_text_chars = Some(value);
        }
        if let Some(value) = probe.limits.max_attachment_bytes {
            merged.limits.max_attachment_bytes = Some(value);
        }
        if let Some(value) = &probe.limits.supported_mime_types {
            merged.limits.supported_mime_types = value.clone();
        }
        if let Some(value) = probe.limits.rate_limit_hint_ms {
            merged.limits.rate_limit_hint_ms = Some(value);
        }
        merged
    }
}

/// Partial live capability probe; every explicit value overrides static code.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChannelCapabilityProbe {
    pub overrides: BTreeMap<ChannelCapability, bool>,
    pub limits: ChannelLimitsProbe,
}

/// Current adapter health projected by its supervisor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelHealth {
    pub status: ChannelHealthStatus,
    pub diagnosis: Option<String>,
    pub consecutive_failures: u32,
    pub checked_at: DateTime<Utc>,
}

impl ChannelHealth {
    pub fn new(status: ChannelHealthStatus) -> Self {
        Self {
            status,
            diagnosis: None,
            consecutive_failures: 0,
            checked_at: Utc::now(),
        }
    }
}

/// Closed command union routed to one adapter.
#[derive(Debug, Clone, PartialEq)]
pub enum ChannelCommand {
    Send {
        envelope: ChannelEnvelopeV1,
        idempotency_key: Option<String>,
    },
    Typing {
        address: ChannelAddress,
        correlation: Correlation,
        state: TypingState,
        idempotency_key: Option<String>,
    },
    Edit {
        address: ChannelAddress,
        correlation: Correlation,
        target_message_id: String,
        parts: Vec<ContentPart>,
        idempotency_key: Option<String>,
    },
    Delete {
        address: ChannelAddress,
        correlation: Correlation,
        target_message_id: String,
        idempotency_key: Option<String>,
    },
    React {
        address: ChannelAddress,
        correlation: Correlation,
        target_message_id: String,
        operation: ReactionOperation,
        emoji: String,
        idempotency_key: Option<String>,
    },
    FinalizeStream {
        address: ChannelAddress,
        correlation: Correlation,
        parts: Vec<ContentPart>,
        idempotency_key: Option<String>,
    },
    ProbeHealth {
        channel: ChannelId,
    },
}

impl ChannelCommand {
    pub fn target_channel(&self) -> Result<ChannelId, ChannelContractError> {
        match self {
            Self::Send { envelope, .. } => ChannelId::new(envelope.address.channel.clone()),
            Self::Typing { address, .. }
            | Self::Edit { address, .. }
            | Self::Delete { address, .. }
            | Self::React { address, .. }
            | Self::FinalizeStream { address, .. } => ChannelId::new(address.channel.clone()),
            Self::ProbeHealth { channel } => Ok(channel.clone()),
        }
    }

    pub fn idempotency_key(&self) -> Option<&str> {
        match self {
            Self::Send {
                idempotency_key, ..
            }
            | Self::Typing {
                idempotency_key, ..
            }
            | Self::Edit {
                idempotency_key, ..
            }
            | Self::Delete {
                idempotency_key, ..
            }
            | Self::React {
                idempotency_key, ..
            }
            | Self::FinalizeStream {
                idempotency_key, ..
            } => idempotency_key.as_deref(),
            Self::ProbeHealth { .. } => None,
        }
    }

    pub fn is_retry_safe(&self) -> bool {
        matches!(self, Self::ProbeHealth { .. }) || self.idempotency_key().is_some()
    }

    pub fn required_capabilities(&self) -> BTreeSet<ChannelCapability> {
        let mut required = BTreeSet::new();
        match self {
            Self::Send { envelope, .. } => {
                if let super::ChannelPayloadV1::Message { parts, .. }
                | super::ChannelPayloadV1::Stream { parts, .. } = &envelope.payload
                {
                    for part in parts {
                        required.insert(match part {
                            ContentPart::Text { .. }
                            | ContentPart::Location { .. }
                            | ContentPart::Reference { .. } => ChannelCapability::EgressText,
                            ContentPart::Markdown { .. } => ChannelCapability::EgressMarkdown,
                            ContentPart::Image { .. } => ChannelCapability::EgressImage,
                            ContentPart::Audio { .. } => ChannelCapability::EgressAudio,
                            ContentPart::Video { .. } => ChannelCapability::EgressVideo,
                            ContentPart::File { .. } => ChannelCapability::EgressFile,
                            ContentPart::Card { .. } => ChannelCapability::EgressCard,
                        });
                    }
                }
            }
            Self::Typing { .. } => {
                required.insert(ChannelCapability::InteractionTyping);
            }
            Self::Edit { .. } => {
                required.insert(ChannelCapability::InteractionEdit);
            }
            Self::Delete { .. } => {
                required.insert(ChannelCapability::InteractionDelete);
            }
            Self::React { .. } => {
                required.insert(ChannelCapability::InteractionReaction);
            }
            Self::FinalizeStream { .. } => {
                required.insert(ChannelCapability::InteractionStreamFinalize);
            }
            Self::ProbeHealth { .. } => {
                required.insert(ChannelCapability::ReliabilityHealth);
            }
        }
        required
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_probe_overrides_static_capability_values() {
        let declared = ChannelCapabilities::new([
            ChannelCapability::EgressText,
            ChannelCapability::EgressMarkdown,
        ]);
        let probe = ChannelCapabilityProbe {
            overrides: BTreeMap::from([
                (ChannelCapability::EgressMarkdown, false),
                (ChannelCapability::EgressImage, true),
            ]),
            ..ChannelCapabilityProbe::default()
        };
        let effective = declared.merge_probe(&probe);
        assert!(effective.supports(ChannelCapability::EgressText));
        assert!(!effective.supports(ChannelCapability::EgressMarkdown));
        assert!(effective.supports(ChannelCapability::EgressImage));
    }
}
