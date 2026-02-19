//! Configuration loading and management

use super::schema::Config;
use super::validate::validate_config;
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

/// Configuration loader
pub struct ConfigLoader {
    config_dir: PathBuf,
}

impl ConfigLoader {
    /// Create a new config loader with the default config directory
    pub fn new() -> Self {
        let config_dir = dirs::home_dir()
            .map(|h| h.join(".agent-diva"))
            .unwrap_or_else(|| PathBuf::from(".agent-diva"));

        Self { config_dir }
    }

    /// Create a new config loader with a custom config directory
    pub fn with_dir<P: AsRef<Path>>(dir: P) -> Self {
        Self {
            config_dir: dir.as_ref().to_path_buf(),
        }
    }

    /// Load configuration from file and environment
    pub fn load(&self) -> crate::Result<Config> {
        let config_path = self.config_dir.join("config.json");
        let mut merged = serde_json::to_value(Config::default())?;

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let file_value: Value = serde_json::from_str(&content)?;
            merge_values(&mut merged, file_value);
        }

        apply_alias_overrides(&mut merged);
        apply_path_overrides(&mut merged);

        let config: Config = serde_json::from_value(merged)?;
        validate_config(&config)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, config: &Config) -> crate::Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;
        let config_path = self.config_dir.join("config.json");
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

fn merge_values(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                if let Some(existing) = base_map.get_mut(&key) {
                    merge_values(existing, value);
                } else {
                    base_map.insert(key, value);
                }
            }
        }
        (base_value, overlay_value) => {
            *base_value = overlay_value;
        }
    }
}

fn parse_env_value(raw: &str) -> Value {
    if let Ok(v) = serde_json::from_str::<Value>(raw) {
        return v;
    }
    if raw.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if raw.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }
    if let Ok(v) = raw.parse::<i64>() {
        return Value::Number(v.into());
    }
    if let Ok(v) = raw.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(v) {
            return Value::Number(n);
        }
    }
    Value::String(raw.to_string())
}

fn set_path_value(root: &mut Value, path: &[String], value: Value) {
    if path.is_empty() {
        *root = value;
        return;
    }

    let mut current = root;
    for segment in &path[..path.len() - 1] {
        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
        let map = current.as_object_mut().expect("object ensured");
        current = map
            .entry(segment.clone())
            .or_insert_with(|| Value::Object(Map::new()));
    }

    if !current.is_object() {
        *current = Value::Object(Map::new());
    }
    if let Some(map) = current.as_object_mut() {
        map.insert(path[path.len() - 1].clone(), value);
    }
}

fn apply_alias_overrides(config: &mut Value) {
    let aliases = [
        ("ANTHROPIC_API_KEY", "providers.anthropic.api_key"),
        ("OPENAI_API_KEY", "providers.openai.api_key"),
        ("OPENROUTER_API_KEY", "providers.openrouter.api_key"),
        ("DEEPSEEK_API_KEY", "providers.deepseek.api_key"),
        ("GROQ_API_KEY", "providers.groq.api_key"),
        ("GEMINI_API_KEY", "providers.gemini.api_key"),
        ("DASHSCOPE_API_KEY", "providers.dashscope.api_key"),
        ("MOONSHOT_API_KEY", "providers.moonshot.api_key"),
        ("MINIMAX_API_KEY", "providers.minimax.api_key"),
        ("HOSTED_VLLM_API_KEY", "providers.vllm.api_key"),
        ("AIHUBMIX_API_KEY", "providers.aihubmix.api_key"),
        ("ZAI_API_KEY", "providers.zhipu.api_key"),
        ("ZHIPUAI_API_KEY", "providers.zhipu.api_key"),
        ("BRAVE_API_KEY", "tools.web.search.api_key"),
    ];

    for (env_key, target_path) in aliases {
        if let Ok(value) = std::env::var(env_key) {
            let path: Vec<String> = target_path.split('.').map(ToString::to_string).collect();
            set_path_value(config, &path, Value::String(value));
        }
    }
}

fn apply_path_overrides(config: &mut Value) {
    const PREFIX: &str = "AGENT_DIVA__";
    for (key, value) in std::env::vars() {
        if !key.starts_with(PREFIX) {
            continue;
        }
        let suffix = &key[PREFIX.len()..];
        if suffix.is_empty() {
            continue;
        }
        let segments: Vec<String> = suffix
            .split("__")
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase())
            .collect();
        if segments.is_empty() {
            continue;
        }
        set_path_value(config, &segments, parse_env_value(&value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::sync::{Mutex, MutexGuard};
    use tempfile::TempDir;

    static ENV_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    struct EnvVarGuard {
        key: String,
        original: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            // SAFETY: tests serialize env mutations with ENV_LOCK.
            unsafe { std::env::set_var(key, value) };
            Self {
                key: key.to_string(),
                original,
            }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(value) = &self.original {
                // SAFETY: tests serialize env mutations with ENV_LOCK.
                unsafe { std::env::set_var(&self.key, value) };
            } else {
                // SAFETY: tests serialize env mutations with ENV_LOCK.
                unsafe { std::env::remove_var(&self.key) };
            }
        }
    }

    fn lock_env() -> MutexGuard<'static, ()> {
        ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn test_load_default_config() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());
        let config = loader.load().unwrap();

        assert_eq!(config.agents.defaults.model, "anthropic/claude-opus-4-5");
        assert_eq!(config.agents.defaults.max_tokens, 8192);
    }

    #[test]
    fn test_save_and_load_config() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());

        let mut config = Config::default();
        config.agents.defaults.model = "test-model".to_string();

        loader.save(&config).unwrap();
        let loaded = loader.load().unwrap();

        assert_eq!(loaded.agents.defaults.model, "test-model");
    }

    #[test]
    fn test_load_applies_alias_env_overrides() {
        let _lock = lock_env();
        let _api_key_guard = EnvVarGuard::set("OPENAI_API_KEY", "sk-openai-from-env");
        let _minimax_guard = EnvVarGuard::set("MINIMAX_API_KEY", "mini-key");

        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());
        let config = loader.load().unwrap();

        assert_eq!(config.providers.openai.api_key, "sk-openai-from-env");
        assert_eq!(config.providers.minimax.api_key, "mini-key");
    }

    #[test]
    fn test_load_applies_path_env_overrides() {
        let _lock = lock_env();
        let _model_guard = EnvVarGuard::set("AGENT_DIVA__AGENTS__DEFAULTS__MODEL", "openai/gpt-4o");
        let _temp_guard = EnvVarGuard::set("AGENT_DIVA__AGENTS__DEFAULTS__TEMPERATURE", "0.9");
        let _iter_guard = EnvVarGuard::set("AGENT_DIVA__AGENTS__DEFAULTS__MAX_TOOL_ITERATIONS", "42");
        let _enabled_guard = EnvVarGuard::set("AGENT_DIVA__CHANNELS__TELEGRAM__ENABLED", "true");
        let _token_guard = EnvVarGuard::set("AGENT_DIVA__CHANNELS__TELEGRAM__TOKEN", "tg-token");

        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());
        let config = loader.load().unwrap();

        assert_eq!(config.agents.defaults.model, "openai/gpt-4o");
        assert!((config.agents.defaults.temperature - 0.9).abs() < f32::EPSILON);
        assert_eq!(config.agents.defaults.max_tool_iterations, 42);
        assert!(config.channels.telegram.enabled);
        assert_eq!(config.channels.telegram.token, "tg-token");
    }

    #[test]
    fn test_path_env_overrides_alias_and_file() {
        let _lock = lock_env();
        let _alias_guard = EnvVarGuard::set("OPENAI_API_KEY", "sk-openai-alias");
        let _path_guard = EnvVarGuard::set(
            "AGENT_DIVA__PROVIDERS__OPENAI__API_KEY",
            "sk-openai-path-override",
        );

        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());

        let config_path = temp_dir.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{"providers":{"openai":{"api_key":"sk-openai-file"}}}"#,
        )
        .unwrap();

        let config = loader.load().unwrap();
        assert_eq!(config.providers.openai.api_key, "sk-openai-path-override");
    }

    #[test]
    fn test_validation_rejects_invalid_temperature() {
        let _lock = lock_env();
        let _temp_guard = EnvVarGuard::set("AGENT_DIVA__AGENTS__DEFAULTS__TEMPERATURE", "2.5");

        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());
        let err = loader.load().unwrap_err();
        assert!(err.to_string().contains("temperature"));
    }

    #[test]
    fn test_load_supports_mcp_servers_camel_case() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());

        let config_path = temp_dir.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{
  "tools": {
    "mcpServers": {
      "filesystem": {
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
      }
    }
  }
}"#,
        )
        .unwrap();

        let config = loader.load().unwrap();
        let server = config.tools.mcp_servers.get("filesystem").unwrap();
        assert_eq!(server.command, "npx");
        assert_eq!(server.args.len(), 3);
    }
}
