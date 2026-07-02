//! Thinking mode and reasoning configuration types.
//!
//! Provides a user-facing thinking mode toggle and per-model reasoning
//! configuration compatible with Cherry Studio's ReasoningConfig schema.

use serde::{Deserialize, Serialize};

/// User-facing thinking mode toggle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingMode {
    /// Auto: enable thinking for reasoning-capable models
    #[default]
    Auto,
    /// Force thinking on for all models
    On,
    /// Disable thinking entirely
    Off,
}

/// Per-model reasoning configuration (compatible with Cherry Studio ReasoningConfig)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    /// Reasoning format type (e.g. "openai-chat", "anthropic", "gemini")
    pub reasoning_type: String,
    /// Token limits for thinking budget
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_token_limits: Option<ThinkingTokenLimits>,
    /// Supported effort levels (e.g. ["low", "medium", "high"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_efforts: Option<Vec<String>>,
    /// Default effort level for this model
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_effort: Option<String>,
}

/// Token budget constraints for thinking/reasoning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingTokenLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<u32>,
}
