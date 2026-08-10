use std::fmt;

use super::PromptSection;

/// Declared reason for rebuilding a session-stable prompt section.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CacheBreakReason {
    MaskChanged,
    L1HotRefresh,
    AgentRulesReload,
    SkillsReload,
    SessionReset,
    SessionEnded,
}

impl CacheBreakReason {
    /// Stable wire value consumed by logs and cache observers.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MaskChanged => "mask_changed",
            Self::L1HotRefresh => "l1_hot_refresh",
            Self::AgentRulesReload => "agent_rules_reload",
            Self::SkillsReload => "skills_reload",
            Self::SessionReset => "session_reset",
            Self::SessionEnded => "session_ended",
        }
    }
}

impl fmt::Display for CacheBreakReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Immutable view of one session's current stable-prefix cache version.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StablePrefixSnapshot {
    pub sections: Vec<PromptSection>,
    pub rendered: String,
    pub prefix_version: u64,
}

impl StablePrefixSnapshot {
    /// Break reasons declared by the sections rebuilt for this version.
    pub fn cache_break_reasons(&self) -> Vec<CacheBreakReason> {
        let mut reasons = self
            .sections
            .iter()
            .filter_map(|section| section.cache_break_reason)
            .collect::<Vec<_>>();
        reasons.sort_unstable();
        reasons.dedup();
        reasons
    }
}
