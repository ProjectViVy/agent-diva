//! Configuration for the E2E test runner.
//!
//! [`E2EConfig`] is loaded from environment variables and provides sensible
//! defaults so that most scenarios can run without explicit configuration.

use std::path::PathBuf;

/// Configuration for the E2E test runner.
#[derive(Debug, Clone)]
pub struct E2EConfig {
    /// API key for the LLM provider.
    pub api_key: String,
    /// Base URL for the LLM provider API.
    pub api_base: String,
    /// Default model identifier to use for scenarios.
    pub default_model: String,
    /// Default per-scenario timeout in seconds.
    pub default_timeout_secs: u64,
    /// Maximum allowed timeout for any single scenario.
    pub max_timeout_secs: u64,
    /// Maximum USD cost budget for the entire E2E run.
    pub cost_budget_usd: f64,
    /// Directory where trace logs are written.
    pub trace_dir: PathBuf,
    /// Directory containing YAML scenario files.
    pub scenarios_dir: PathBuf,
    /// Provider name passed to OpenAiCompatibleClient (e.g. "deepseek", "openai", "azure").
    /// Defaults to "deepseek" for backward compatibility.
    pub provider_name: String,
    /// Optional override model for the judge evaluation call.
    pub judge_model: Option<String>,
}

impl E2EConfig {
    /// Load configuration from environment variables with sensible defaults.
    ///
    /// Returns `Ok(Self)` when at least one API key is found, or `Err` with a
    /// skip-message when neither `DEEPSEEK_API_KEY` nor `E2E_API_KEY` is set.
    pub fn from_env() -> Result<Self, String> {
        let api_key = std::env::var("DEEPSEEK_API_KEY")
            .or_else(|_| std::env::var("E2E_API_KEY"))
            .map_err(|_| "DEEPSEEK_API_KEY not set — skipping E2E tests".to_string())?;

        let api_base = std::env::var("E2E_API_BASE")
            .unwrap_or_else(|_| "https://api.deepseek.com/v1".to_string());

        let default_model =
            std::env::var("E2E_MODEL").unwrap_or_else(|_| "deepseek-chat".to_string());

        let default_timeout_secs = std::env::var("E2E_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        let max_timeout_secs = std::env::var("E2E_MAX_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(120);

        let trace_dir = std::env::var("E2E_TRACE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("target/e2e-traces"));

        let scenarios_dir = std::env::var("E2E_SCENARIOS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("scripts/e2e/scenarios"));

        let judge_model = std::env::var("E2E_JUDGE_MODEL").ok();

        let provider_name =
            std::env::var("E2E_PROVIDER_NAME").unwrap_or_else(|_| "deepseek".to_string());

        Ok(Self {
            api_key,
            api_base,
            default_model,
            default_timeout_secs,
            max_timeout_secs,
            cost_budget_usd: 0.50,
            trace_dir,
            scenarios_dir,
            provider_name,
            judge_model,
        })
    }

    /// Check whether an API key is available in the environment (for skip
    /// logic in integration tests).
    pub fn is_api_key_available() -> bool {
        std::env::var("DEEPSEEK_API_KEY").is_ok() || std::env::var("E2E_API_KEY").is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: clear all E2E-related env vars.
    fn clear_env() {
        unsafe {
            std::env::remove_var("DEEPSEEK_API_KEY");
            std::env::remove_var("E2E_API_KEY");
            std::env::remove_var("E2E_API_BASE");
            std::env::remove_var("E2E_MODEL");
            std::env::remove_var("E2E_TIMEOUT");
            std::env::remove_var("E2E_MAX_TIMEOUT");
            std::env::remove_var("E2E_TRACE_DIR");
            std::env::remove_var("E2E_SCENARIOS_DIR");
            std::env::remove_var("E2E_JUDGE_MODEL");
            std::env::remove_var("E2E_PROVIDER_NAME");
        }
    }

    /// Helper: set multiple env vars at once.
    fn set_env(vars: &[(&str, &str)]) {
        unsafe {
            for (k, v) in vars {
                std::env::set_var(k, v);
            }
        }
    }

    /// Env-dependent tests must run sequentially because `std::env::set_var`
    /// is a global side-effect that races across parallel test threads.
    /// We chain them into a single test to guarantee order.
    #[test]
    fn test_env_operations_sequentially() {
        // 1. No key → Err
        clear_env();
        let result = E2EConfig::from_env();
        assert!(result.is_err(), "Expected Err when no API key is set");
        assert!(
            result.as_ref().unwrap_err().contains("not set"),
            "Error message should mention missing key"
        );

        // 2. is_api_key_available false when nothing set
        clear_env();
        assert!(!E2EConfig::is_api_key_available());

        // 3. is_api_key_available true with DEEPSEEK_API_KEY
        clear_env();
        set_env(&[("DEEPSEEK_API_KEY", "sk-test")]);
        assert!(E2EConfig::is_api_key_available());

        // 4. DEEPSEEK_API_KEY → Ok
        clear_env();
        set_env(&[("DEEPSEEK_API_KEY", "sk-deepseek-key")]);
        let config = E2EConfig::from_env().expect("Expected Ok with DEEPSEEK_API_KEY");
        assert_eq!(config.api_key, "sk-deepseek-key");

        // 5. E2E_API_KEY → Ok (fallback)
        clear_env();
        set_env(&[("E2E_API_KEY", "sk-e2e-fallback")]);
        let config = E2EConfig::from_env().expect("Expected Ok with E2E_API_KEY");
        assert_eq!(config.api_key, "sk-e2e-fallback");

        // 6. Default values
        clear_env();
        set_env(&[("DEEPSEEK_API_KEY", "sk-test")]);
        let config = E2EConfig::from_env().expect("Expected Ok with defaults");
        assert_eq!(config.api_base, "https://api.deepseek.com/v1");
        assert_eq!(config.default_model, "deepseek-chat");
        assert_eq!(config.default_timeout_secs, 30);
        assert_eq!(config.max_timeout_secs, 120);
        assert_eq!(config.cost_budget_usd, 0.50);
        assert_eq!(config.trace_dir, PathBuf::from("target/e2e-traces"));
        assert_eq!(config.scenarios_dir, PathBuf::from("scripts/e2e/scenarios"));
        assert!(config.judge_model.is_none());
        assert_eq!(config.provider_name, "deepseek");

        // 7. Override values from env
        clear_env();
        set_env(&[
            ("DEEPSEEK_API_KEY", "sk-test"),
            ("E2E_API_BASE", "https://custom.example.com/v1"),
            ("E2E_MODEL", "custom-model"),
            ("E2E_TIMEOUT", "60"),
            ("E2E_MAX_TIMEOUT", "300"),
            ("E2E_TRACE_DIR", "/tmp/custom-traces"),
            ("E2E_SCENARIOS_DIR", "/etc/e2e/scenarios"),
            ("E2E_JUDGE_MODEL", "judge-model-v2"),
        ]);
        let config = E2EConfig::from_env().expect("Expected Ok with overrides");
        assert_eq!(config.api_base, "https://custom.example.com/v1");
        assert_eq!(config.default_model, "custom-model");
        assert_eq!(config.default_timeout_secs, 60);
        assert_eq!(config.max_timeout_secs, 300);
        assert_eq!(config.trace_dir, PathBuf::from("/tmp/custom-traces"));
        assert_eq!(config.scenarios_dir, PathBuf::from("/etc/e2e/scenarios"));
        assert_eq!(config.judge_model.as_deref(), Some("judge-model-v2"));
        assert_eq!(config.provider_name, "deepseek");

        // 8. E2E_PROVIDER_NAME override
        clear_env();
        set_env(&[
            ("DEEPSEEK_API_KEY", "sk-test"),
            ("E2E_PROVIDER_NAME", "openai"),
            ("E2E_API_BASE", "https://api.openai.com/v1"),
            ("E2E_MODEL", "gpt-4o-mini"),
        ]);
        let config = E2EConfig::from_env().expect("Expected Ok with openai provider");
        assert_eq!(config.provider_name, "openai");
        assert_eq!(config.api_base, "https://api.openai.com/v1");
        assert_eq!(config.default_model, "gpt-4o-mini");

        // Cleanup
        clear_env();
    }
}
