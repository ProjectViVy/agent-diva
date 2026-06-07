//! Context budget monitoring.
//!
//! Tracks estimated token usage against a configured budget and decides
//! when compaction is needed to avoid exceeding the provider's context window.
//!
//! # Budget allocation
//!
//! ```text
//! total_budget = max_tokens
//! system_budget = total_budget × system_budget_ratio    (reserved for system prompt)
//! history_budget = total_budget - system_budget           (available for messages)
//! compact_threshold = history_budget × compact_threshold_ratio
//! ```
//!
//! Compaction is triggered when `history_estimated > compact_threshold`.
//!
//! # References
//!
//! - ADR-0010: Context Compaction architecture

use agent_diva_core::session::ChatMessage;

use super::token_estimate;

/// Configuration for context budget management.
///
/// # Default values
///
/// | Parameter                | Default    | Rationale                                      |
/// |--------------------------|------------|------------------------------------------------|
/// | `max_tokens`             | 180_000    | DeepSeek V3 context = 128K, with headroom      |
/// | `system_budget_ratio`    | 0.15       | 15% reserved for system prompt + skills        |
/// | `compact_threshold_ratio`| 0.80       | Compact when history budget reaches 80%        |
/// | `keep_recent_count`      | 10         | Always keep 10 most recent messages            |
#[derive(Debug, Clone)]
pub struct BudgetConfig {
    /// Maximum tokens allowed in the full assembled context.
    pub max_tokens: usize,

    /// Fraction of `max_tokens` reserved for system prompt, skills, and memory.
    /// Must be in range [0.0, 1.0).
    pub system_budget_ratio: f64,

    /// Fraction of history budget that triggers compaction.
    /// Must be in range (0.0, 1.0].
    pub compact_threshold_ratio: f64,

    /// Number of recent messages to always keep (never compacted).
    pub keep_recent_count: usize,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_tokens: 180_000,
            system_budget_ratio: 0.15,
            compact_threshold_ratio: 0.80,
            keep_recent_count: 10,
        }
    }
}

impl From<agent_diva_core::config::CompactionBudgetConfig> for BudgetConfig {
    fn from(cfg: agent_diva_core::config::CompactionBudgetConfig) -> Self {
        Self {
            max_tokens: cfg.max_tokens,
            system_budget_ratio: cfg.system_budget_ratio,
            compact_threshold_ratio: cfg.compact_threshold_ratio,
            keep_recent_count: cfg.keep_recent_count,
        }
    }
}
impl BudgetConfig {
    /// Build a [`BudgetConfig`] using the hardcoded context window for the given model.
    ///
    /// Looks up the model in the hardcoded table via
    /// [`agent_diva_providers::model_capabilities_for_model`].
    /// Falls back to 128_000 for unknown models (conservative default).
    ///
    /// Other config fields (`system_budget_ratio`, `compact_threshold_ratio`,
    /// `keep_recent_count`) use their default values.
    pub fn for_model(model: &str) -> Self {
        let max_tokens = agent_diva_providers::model_capabilities_for_model(model)
            .context_window
            .unwrap_or(128_000);
        Self {
            max_tokens,
            ..Self::default()
        }
    }
    /// Build a [`BudgetConfig`] with the full priority chain:
    ///
    /// 1. Environment variable `AGENT_DIVA_MAX_CONTEXT_TOKENS` (highest priority)
    /// 2. Hardcoded model table via [`for_model`](Self::for_model)
    /// 3. Conservative default of 128_000 tokens
    ///
    /// This is the recommended constructor for runtime use — it respects
    /// operator overrides while still being model-aware.
    pub fn from_env_or_model(model: &str) -> Self {
        if let Ok(val) = std::env::var("AGENT_DIVA_MAX_CONTEXT_TOKENS") {
            if let Ok(tokens) = val.parse::<usize>() {
                if tokens > 0 {
                    return Self {
                        max_tokens: tokens,
                        ..Self::default()
                    };
                }
            }
        }
        Self::for_model(model)
    }
}

/// Report produced by [`check_budget`].
///
/// Contains token estimates and the compaction decision.
#[derive(Debug, Clone)]
pub struct BudgetReport {
    /// Total estimated tokens (history + system allocation).
    pub total_estimated: usize,

    /// System budget allocation (reserved headroom, not measured from messages).
    pub system_estimated: usize,

    /// Estimated tokens consumed by the message history.
    pub history_estimated: usize,

    /// Pressure ratio: `history_estimated / history_budget`.
    ///
    /// - `0.0` = empty history
    /// - `< 0.80` = safe zone
    /// - `≥ 0.80` = compaction threshold approaching
    /// - `> 1.0` = history exceeds its allocated budget
    pub pressure_ratio: f64,

    /// Whether compaction should be triggered.
    ///
    /// True when `history_estimated > compact_threshold` and history is non-empty.
    pub should_compact: bool,
}

/// Check whether the message history is approaching the context budget limit.
///
/// # Arguments
///
/// * `history` - Slice of chat messages representing the history to be included
///   in the context window.
/// * `config` - Budget configuration specifying limits and thresholds.
///
/// # Returns
///
/// A [`BudgetReport`] with token estimates, pressure ratio, and compaction decision.
///
/// # Algorithm
///
/// 1. Estimate total tokens for `history` using [`token_estimate::estimate_total_tokens`].
/// 2. Compute `history_budget = max_tokens × (1 - system_budget_ratio)`.
/// 3. Compute `compact_threshold = history_budget × compact_threshold_ratio`.
/// 4. Set `should_compact = history_estimated > compact_threshold && history_estimated > 0`.
pub fn check_budget(history: &[ChatMessage], config: &BudgetConfig) -> BudgetReport {
    let history_estimated = token_estimate::estimate_total_tokens(history);

    let system_budget = (config.max_tokens as f64 * config.system_budget_ratio) as usize;
    let history_budget = config.max_tokens.saturating_sub(system_budget);
    let compact_threshold = (history_budget as f64 * config.compact_threshold_ratio) as usize;

    let system_estimated = system_budget;
    let total_estimated = history_estimated.saturating_add(system_estimated);

    let pressure_ratio = if history_budget > 0 {
        history_estimated as f64 / history_budget as f64
    } else {
        // Degenerate case: no history budget → always at pressure
        if history_estimated > 0 {
            f64::INFINITY
        } else {
            0.0
        }
    };

    let should_compact = history_estimated > compact_threshold && history_estimated > 0;

    BudgetReport {
        total_estimated,
        system_estimated,
        history_estimated,
        pressure_ratio,
        should_compact,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_msg(content: &str) -> ChatMessage {
        ChatMessage {
            role: "user".to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            reasoning_content: None,
            thinking_blocks: None,
        }
    }

    // ── Basic behaviour ──────────────────────────────────────────

    #[test]
    fn empty_history_no_compact() {
        let config = BudgetConfig::default();
        let report = check_budget(&[], &config);
        assert!(!report.should_compact);
        assert_eq!(report.history_estimated, 0);
        assert_eq!(report.pressure_ratio, 0.0);
        // system_estimated should be the reserved budget
        assert_eq!(report.system_estimated, (180_000.0 * 0.15) as usize);
    }

    #[test]
    fn small_history_no_compact() {
        let config = BudgetConfig::default();
        let msgs: Vec<_> = (0..5)
            .map(|i| make_msg(&format!("message number {}", i)))
            .collect();
        let report = check_budget(&msgs, &config);
        assert!(!report.should_compact);
        assert!(report.pressure_ratio < 0.01);
    }

    // ── Threshold triggering ─────────────────────────────────────

    #[test]
    fn above_threshold_triggers_compact() {
        let config = BudgetConfig {
            max_tokens: 1000,
            system_budget_ratio: 0.0,
            compact_threshold_ratio: 0.80,
            keep_recent_count: 10,
        };
        // Each message: 402 chars → ceil(402/3) = 134 tokens
        // 7 messages → 938 tokens
        // history_budget = 1000, compact_threshold = 800
        // 938 > 800 → should_compact = true
        let msgs: Vec<_> = (0..7).map(|_| make_msg(&"x".repeat(402))).collect();
        let report = check_budget(&msgs, &config);
        assert!(report.should_compact);
        assert!(report.pressure_ratio > 0.8);
    }

    #[test]
    fn below_threshold_no_compact() {
        let config = BudgetConfig {
            max_tokens: 1000,
            system_budget_ratio: 0.0,
            compact_threshold_ratio: 0.80,
            keep_recent_count: 10,
        };
        // 1 message = 134 tokens < 800 → no compact
        let msgs = vec![make_msg(&"x".repeat(402))];
        let report = check_budget(&msgs, &config);
        assert!(!report.should_compact);
        assert!(report.pressure_ratio < 0.8);
    }

    #[test]
    fn zero_history_with_nonzero_budget_no_compact() {
        let config = BudgetConfig {
            max_tokens: 100,
            system_budget_ratio: 0.0,
            compact_threshold_ratio: 0.80,
            keep_recent_count: 10,
        };
        let report = check_budget(&[], &config);
        assert!(!report.should_compact);
    }

    // ── System budget ratio ──────────────────────────────────────

    #[test]
    fn system_budget_ratio_reduces_history_budget() {
        let config = BudgetConfig {
            max_tokens: 1000,
            system_budget_ratio: 0.5,
            compact_threshold_ratio: 0.80,
            keep_recent_count: 10,
        };
        // Each message: 300 chars → ceil(300/3) = 100 tokens
        // 5 messages → 500 tokens
        // system_budget = 500, history_budget = 500, threshold = 400
        // 500 > 400 → should_compact = true
        let msgs: Vec<_> = (0..5).map(|_| make_msg(&"x".repeat(300))).collect();
        let report = check_budget(&msgs, &config);
        assert!(report.should_compact);
        assert_eq!(report.system_estimated, 500);
    }

    // ── Report field consistency ─────────────────────────────────

    #[test]
    fn report_total_equals_sum() {
        let config = BudgetConfig::default();
        let msgs = vec![make_msg("test message")];
        let report = check_budget(&msgs, &config);
        assert_eq!(
            report.total_estimated,
            report
                .history_estimated
                .saturating_add(report.system_estimated)
        );
    }

    #[test]
    fn pressure_ratio_non_negative() {
        let config = BudgetConfig::default();
        let msgs = vec![make_msg("test")];
        let report = check_budget(&msgs, &config);
        assert!(report.pressure_ratio >= 0.0);
    }

    // ── Default values ───────────────────────────────────────────

    #[test]
    fn default_config_values() {
        let config = BudgetConfig::default();
        assert_eq!(config.max_tokens, 180_000);
        assert_eq!(config.system_budget_ratio, 0.15);
        assert_eq!(config.compact_threshold_ratio, 0.80);
        assert_eq!(config.keep_recent_count, 10);
    }

    // ── Edge cases ───────────────────────────────────────────────

    #[test]
    fn zero_max_tokens() {
        let config = BudgetConfig {
            max_tokens: 0,
            system_budget_ratio: 0.0,
            compact_threshold_ratio: 0.80,
            keep_recent_count: 0,
        };
        // history_budget = 0, threshold = 0, pressure_ratio = INF
        let msgs = vec![make_msg("hello")];
        let report = check_budget(&msgs, &config);
        assert!(report.should_compact);
        assert!(report.pressure_ratio.is_infinite());
    }

    #[test]
    fn max_system_budget_ratio() {
        let config = BudgetConfig {
            max_tokens: 1000,
            system_budget_ratio: 0.99,
            compact_threshold_ratio: 0.80,
            keep_recent_count: 10,
        };
        // system_budget = 990, history_budget = 10, threshold = 8
        // Need a message longer than ~24 chars to exceed 8 tokens
        let msgs = vec![make_msg(&"x".repeat(30))]; // 30 chars → 10 tokens > 8
        let report = check_budget(&msgs, &config);
        assert!(report.should_compact);
    }

    #[test]
    fn compact_threshold_at_one_hundred_percent() {
        let config = BudgetConfig {
            max_tokens: 1000,
            system_budget_ratio: 0.0,
            compact_threshold_ratio: 1.0,
            keep_recent_count: 10,
        };
        // threshold = 1000, need to exceed that
        let msgs = vec![make_msg(&"x".repeat(900))]; // ≈ 300 tokens
        let report = check_budget(&msgs, &config);
        assert!(!report.should_compact);
    }
    // ── for_model / from_env_or_model ─────────────────────────────
    #[test]
    fn for_model_known_model() {
        // DeepSeek chat → 128K context window
        let config = BudgetConfig::for_model("deepseek-chat");
        assert_eq!(config.max_tokens, 128_000);
        // Other fields use defaults
        assert_eq!(config.system_budget_ratio, 0.15);
        assert_eq!(config.compact_threshold_ratio, 0.80);
        assert_eq!(config.keep_recent_count, 10);
    }
    #[test]
    fn for_model_large_context_model() {
        // Claude Sonnet 4.6 → 1M context window
        let config = BudgetConfig::for_model("claude-sonnet-4-6");
        assert_eq!(config.max_tokens, 1_000_000);
    }
    #[test]
    fn for_model_unknown_model_falls_back_to_128k() {
        let config = BudgetConfig::for_model("some-unknown-model");
        assert_eq!(config.max_tokens, 128_000);
    }
    #[test]
    fn for_model_with_provider_prefix() {
        // Provider prefix should be stripped
        let config = BudgetConfig::for_model("openai/gpt-4o");
        assert_eq!(config.max_tokens, 128_000);
        let config = BudgetConfig::for_model("anthropic/claude-sonnet-4-6");
        assert_eq!(config.max_tokens, 1_000_000);
    }
    #[test]
    fn from_env_or_model_uses_model_when_no_env() {
        // Ensure env var is not set
        std::env::remove_var("AGENT_DIVA_MAX_CONTEXT_TOKENS");
        let config = BudgetConfig::from_env_or_model("gpt-5");
        assert_eq!(config.max_tokens, 400_000);
    }
    #[test]
    fn from_env_or_model_env_overrides_model() {
        std::env::set_var("AGENT_DIVA_MAX_CONTEXT_TOKENS", "500000");
        let config = BudgetConfig::from_env_or_model("deepseek-chat");
        assert_eq!(config.max_tokens, 500_000);
        std::env::remove_var("AGENT_DIVA_MAX_CONTEXT_TOKENS");
    }
    #[test]
    fn from_env_or_model_ignores_invalid_env() {
        std::env::set_var("AGENT_DIVA_MAX_CONTEXT_TOKENS", "not-a-number");
        let config = BudgetConfig::from_env_or_model("gpt-4o");
        assert_eq!(config.max_tokens, 128_000); // Falls through to model table
        std::env::remove_var("AGENT_DIVA_MAX_CONTEXT_TOKENS");
    }
    #[test]
    fn from_env_or_model_ignores_zero_env() {
        std::env::set_var("AGENT_DIVA_MAX_CONTEXT_TOKENS", "0");
        let config = BudgetConfig::from_env_or_model("gpt-4o");
        assert_eq!(config.max_tokens, 128_000); // Zero is treated as invalid
        std::env::remove_var("AGENT_DIVA_MAX_CONTEXT_TOKENS");
    }
}
