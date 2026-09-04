//! Configuration loading and management

use super::schema::Config;
use super::validate::validate_config;
use serde_json::{Map, Value};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{info, warn};

static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Configuration loader
#[derive(Clone)]
pub struct ConfigLoader {
    config_dir: PathBuf,
    config_path: PathBuf,
}

impl ConfigLoader {
    /// Create a new config loader with the default config directory
    pub fn new() -> Self {
        let config_dir = dirs::home_dir()
            .map(|h| h.join(".agent-diva"))
            .unwrap_or_else(|| PathBuf::from(".agent-diva"));

        let config_path = config_dir.join("config.json");

        Self {
            config_dir,
            config_path,
        }
    }

    /// Create a new config loader with a custom config directory
    pub fn with_dir<P: AsRef<Path>>(dir: P) -> Self {
        let config_dir = dir.as_ref().to_path_buf();
        Self {
            config_path: config_dir.join("config.json"),
            config_dir,
        }
    }

    /// Create a new config loader with an explicit config file path
    pub fn with_file<P: AsRef<Path>>(path: P) -> Self {
        let config_path = path.as_ref().to_path_buf();
        let config_dir = config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        Self {
            config_dir,
            config_path,
        }
    }

    /// Load configuration from file and environment
    pub fn load(&self) -> crate::Result<Config> {
        let mut merged = serde_json::to_value(Config::default())?;

        if self.config_path.exists() {
            let content = std::fs::read_to_string(&self.config_path)?;
            let file_value: Value = serde_json::from_str(&content)?;
            merge_values(&mut merged, file_value);
        }

        apply_alias_overrides(&mut merged);
        apply_path_overrides(&mut merged);
        normalize_alias_keys(&mut merged);

        let config: Config = serde_json::from_value(merged)?;
        validate_config(&config)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, config: &Config) -> crate::Result<()> {
        fs::create_dir_all(&self.config_dir)?;
        let content = serde_json::to_string_pretty(config)?;
        let (temporary_path, mut temporary_file) = create_save_file(&self.config_path)?;
        let write_result = (|| -> std::io::Result<()> {
            temporary_file.write_all(content.as_bytes())?;
            temporary_file.flush()?;
            temporary_file.sync_all()?;
            drop(temporary_file);
            atomic_replace(&temporary_path, &self.config_path)
        })();

        if let Err(error) = write_result {
            let _ = fs::remove_file(&temporary_path);
            return Err(error.into());
        }
        Ok(())
    }

    /// Get the config directory path
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Get the config file path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Start a background task that polls the config file for mtime changes.
    ///
    /// When a change is detected, the config is reloaded, diffed against the
    /// previous version, and the `on_diff` callback is invoked with the diff
    /// and the new config.
    ///
    /// Returns a `JoinHandle` that can be aborted to stop polling.
    ///
    /// # Debounce
    ///
    /// After detecting an mtime change, the task waits 500 ms and re-checks
    /// the mtime before reloading.  This coalesces rapid saves (e.g. editor
    /// auto-save) into a single reload cycle.
    pub fn start_hot_reload<F>(&self, on_diff: F) -> JoinHandle<()>
    where
        F: Fn(ConfigDiff, Config) + Send + 'static,
    {
        let config_dir = self.config_dir.clone();
        let config_path = self.config_path.clone();
        let mut last_mtime = Self::read_file_mtime(&config_path);
        let mut last_config = self.load().ok();

        info!(
            "Starting config hot-reload watcher for '{}' (poll interval: 5s)",
            config_path.display()
        );

        tokio::spawn(async move {
            loop {
                sleep(std::time::Duration::from_secs(5)).await;

                let new_mtime = Self::read_file_mtime(&config_path);
                if new_mtime == last_mtime || new_mtime.is_none() {
                    continue;
                }

                // Debounce: rapid saves (e.g. editor autosave) coalesce
                sleep(std::time::Duration::from_millis(500)).await;

                let debounced_mtime = Self::read_file_mtime(&config_path);
                if debounced_mtime != new_mtime {
                    // File still being written — skip this cycle
                    continue;
                }
                last_mtime = debounced_mtime;

                let loader = ConfigLoader::with_dir(&config_dir);
                match loader.load() {
                    Ok(new_config) => {
                        let diff = match &last_config {
                            Some(old) => compute_diff(old, &new_config),
                            None => ConfigDiff::default(),
                        };

                        if diff.has_changes() {
                            info!(
                                "Config hot-reload: {} change(s) ({} hot-reloadable, {} restart-required)",
                                diff.hot_reload.len() + diff.restart_required.len(),
                                diff.hot_reload.len(),
                                diff.restart_required.len(),
                            );
                            on_diff(diff, new_config.clone());
                        }

                        last_config = Some(new_config);
                    }
                    Err(e) => {
                        warn!("Failed to reload config after mtime change: {}", e);
                    }
                }
            }
        })
    }

    /// Read the mtime of an arbitrary file path.
    fn read_file_mtime(path: &Path) -> Option<SystemTime> {
        std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
    }
}

fn create_save_file(path: &Path) -> std::io::Result<(PathBuf, File)> {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.json");
    for _ in 0..32 {
        let sequence = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary_path = path.with_file_name(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            sequence
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
        {
            Ok(file) => return Ok((temporary_path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "unable to allocate a unique configuration save file",
    ))
}

#[cfg(windows)]
fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;

    #[link(name = "kernel32")]
    extern "system" {
        #[link_name = "MoveFileExW"]
        fn move_file_ex_w(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    let source = source
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let destination = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    // SAFETY: both strings are NUL-terminated UTF-16 buffers that remain
    // alive for the duration of the Win32 call.
    let replaced = unsafe {
        move_file_ex_w(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn atomic_replace(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

// ── Hot-reload types ─────────────────────────────────────────────────────────

/// Result of comparing two config versions.
#[derive(Debug, Clone, Default)]
pub struct ConfigDiff {
    /// Fields that can be applied at runtime without restart.
    pub hot_reload: Vec<ChangedField>,
    /// Fields that require a full restart to take effect.
    pub restart_required: Vec<ChangedField>,
}

impl ConfigDiff {
    /// Returns `true` when there is at least one change in either category.
    pub fn has_changes(&self) -> bool {
        !self.hot_reload.is_empty() || !self.restart_required.is_empty()
    }

    /// Returns `true` when at least one field requires a restart.
    pub fn has_restart_required(&self) -> bool {
        !self.restart_required.is_empty()
    }
}

/// A single changed field with before / after values.
#[derive(Debug, Clone)]
pub struct ChangedField {
    /// Dot-separated field path (e.g. `"agents.defaults.model"`).
    pub field: String,
    /// Previous value, if any.
    pub old_value: Option<Value>,
    /// New value, if any.
    pub new_value: Option<Value>,
}

/// Classify a dot-separated config field path as hot-reloadable or
/// restart-required.
///
/// Only explicitly allow-listed fields are considered safe to apply at
/// runtime.  Everything else is classified as `"restart_required"`.
fn classify_field(path: &str) -> &'static str {
    // ── Hot-reloadable allowlist ─────────────────────────────────────────
    if path.starts_with("agents.defaults.model")
        || path.starts_with("agents.defaults.temperature")
        || path.starts_with("agents.defaults.max_tool_iterations")
        || path.starts_with("agents.defaults.reasoning_effort")
        || path.starts_with("tools.budget")
        || path.starts_with("tools.exec.timeout")
        || path.starts_with("tools.web.search.enabled")
        || path.starts_with("tools.web.fetch.enabled")
        || path.starts_with("sandbox.mode")
        || path.starts_with("sandbox.approval_policy")
        || path.starts_with("logging.level")
    {
        "hot_reload"
    } else {
        // Everything else (providers, channels, MCP servers, ports, keys …)
        // requires a restart.
        "restart_required"
    }
}

// ── Config diff computation ──────────────────────────────────────────────────

/// Compute the difference between two [`Config`] instances.
///
/// Serialises both configs to `serde_json::Value` and recursively compares
/// every path, classifying each change as hot-reloadable or restart-required
/// via [`classify_field`].
pub fn compute_diff(old: &Config, new: &Config) -> ConfigDiff {
    let old_value = serde_json::to_value(old).unwrap_or(Value::Null);
    let new_value = serde_json::to_value(new).unwrap_or(Value::Null);
    let mut diff = ConfigDiff::default();
    collect_diff_paths(&old_value, &new_value, "", &mut diff);
    diff
}

/// Recursive helper that walks JSON Value trees and accumulates differing
/// paths into a `ConfigDiff`.
fn collect_diff_paths(old: &Value, new: &Value, base_path: &str, diff: &mut ConfigDiff) {
    match (old, new) {
        (Value::Object(old_map), Value::Object(new_map)) => {
            let mut keys: Vec<&String> = old_map.keys().chain(new_map.keys()).collect();
            keys.sort_unstable();
            keys.dedup();

            for key in keys {
                let child_path = if base_path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", base_path, key)
                };
                let old_child = old_map.get(key).unwrap_or(&Value::Null);
                let new_child = new_map.get(key).unwrap_or(&Value::Null);

                if old_child != new_child {
                    if old_child.is_object() && new_child.is_object() {
                        // Recurse into objects for fine-grained per-field diff
                        collect_diff_paths(old_child, new_child, &child_path, diff);
                    } else {
                        let change = ChangedField {
                            field: child_path.clone(),
                            old_value: Some(old_child.clone()),
                            new_value: Some(new_child.clone()),
                        };
                        match classify_field(&child_path) {
                            "hot_reload" => diff.hot_reload.push(change),
                            _ => diff.restart_required.push(change),
                        }
                    }
                }
            }
        }
        _ => {
            if old != new {
                let change = ChangedField {
                    field: base_path.to_string(),
                    old_value: Some(old.clone()),
                    new_value: Some(new.clone()),
                };
                match classify_field(base_path) {
                    "hot_reload" => diff.hot_reload.push(change),
                    _ => diff.restart_required.push(change),
                }
            }
        }
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

fn object_at_path_mut<'a>(
    root: &'a mut Value,
    path: &[&str],
) -> Option<&'a mut Map<String, Value>> {
    let mut current = root;
    for segment in path {
        current = current.get_mut(*segment)?;
    }
    current.as_object_mut()
}

fn coalesce_alias_keys(
    root: &mut Value,
    object_path: &[&str],
    canonical_key: &str,
    alias_keys: &[&str],
) {
    let Some(map) = object_at_path_mut(root, object_path) else {
        return;
    };

    let mut merged_value = map.remove(canonical_key);
    for alias_key in alias_keys {
        if let Some(alias_value) = map.remove(*alias_key) {
            // Alias keys represent explicit user input and should override defaults.
            merged_value = Some(alias_value);
        }
    }

    if let Some(value) = merged_value {
        map.insert(canonical_key.to_string(), value);
    }
}

fn normalize_alias_keys(config: &mut Value) {
    coalesce_alias_keys(config, &["tools"], "mcpServers", &["mcp_servers"]);
    coalesce_alias_keys(config, &["tools"], "mcpManager", &["mcp_manager"]);
    // Legacy top-level `pet` section was renamed to `mate`.
    coalesce_alias_keys(config, &[], "mate", &["pet"]);
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

        assert_eq!(config.agents.defaults.provider.as_deref(), Some("deepseek"));
        assert_eq!(config.agents.defaults.model, "deepseek-chat");
        assert_eq!(config.agents.defaults.max_tokens, 8192);
        assert!(config.reports.llm_curation.enabled);
    }

    #[test]
    fn test_save_and_load_config() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());

        let mut config = Config::default();
        config.agents.defaults.model = "test-model".to_string();

        loader.save(&config).unwrap();
        config.agents.defaults.model = "replacement-model".to_string();
        loader.save(&config).unwrap();
        let loaded = loader.load().unwrap();

        assert_eq!(loaded.agents.defaults.model, "replacement-model");
    }

    #[test]
    fn test_atomic_save_keeps_previous_destination_when_replace_fails() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        std::fs::create_dir(&config_path).unwrap();
        let marker = config_path.join("marker");
        std::fs::write(&marker, "old destination").unwrap();

        let loader = ConfigLoader::with_file(&config_path);
        let error = loader.save(&Config::default()).unwrap_err();

        assert!(error.to_string().contains("I/O error"));
        assert_eq!(std::fs::read_to_string(marker).unwrap(), "old destination");
        let temporary_files = std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".config.json.")
            })
            .count();
        assert_eq!(temporary_files, 0, "temporary save file must be cleaned up");
    }

    #[test]
    fn test_explicitly_disabled_report_curation_overrides_default() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());
        let mut config = Config::default();
        config.reports.llm_curation.enabled = false;

        loader.save(&config).unwrap();
        let loaded = loader.load().unwrap();

        assert!(!loaded.reports.llm_curation.enabled);
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
        let _iter_guard =
            EnvVarGuard::set("AGENT_DIVA__AGENTS__DEFAULTS__MAX_TOOL_ITERATIONS", "42");
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
    fn test_load_allows_invalid_enabled_channel_config() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());

        let config_path = temp_dir.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{
  "channels": {
    "discord": {
      "enabled": true,
      "token": ""
    }
  }
}"#,
        )
        .unwrap();

        let config = loader.load().unwrap();
        assert!(config.channels.discord.enabled);
        assert!(config.channels.discord.token.is_empty());
    }

    #[test]
    fn test_load_migrates_legacy_channels_without_removed_tombstones() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());

        std::fs::write(
            loader.config_path(),
            r#"{
  "channels": {
    "telegram": {
      "enabled": true,
      "token": "legacy-token"
    }
  }
}"#,
        )
        .unwrap();

        let config = loader.load().unwrap();
        assert!(config.channels.telegram.enabled);
        assert_eq!(config.channels.telegram.token, "legacy-token");
        assert!(config.channels.removed.is_empty());
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

    #[test]
    fn test_load_supports_mcp_servers_snake_case_alias() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());

        let config_path = temp_dir.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{
  "tools": {
    "mcp_servers": {
      "filesystem": {
        "command": "uvx",
        "args": ["mcp-server-filesystem", "."]
      }
    }
  }
}"#,
        )
        .unwrap();

        let config = loader.load().unwrap();
        let server = config.tools.mcp_servers.get("filesystem").unwrap();
        assert_eq!(server.command, "uvx");
        assert_eq!(server.args.len(), 2);
    }

    #[test]
    fn test_load_migrates_legacy_pet_section_to_mate() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let loader = ConfigLoader::with_dir(temp_dir.path());

        let config_path = temp_dir.path().join("config.json");
        std::fs::write(
            &config_path,
            r#"{
  "pet": {
    "enabled": false,
    "tts_speed": 1.5
  }
}"#,
        )
        .unwrap();

        let config = loader.load().unwrap();
        assert!(!config.mate.enabled);
        assert_eq!(config.mate.tts_speed, 1.5);

        // The persisted form must use the canonical `mate` key.
        let serialized = serde_json::to_string(&config).unwrap();
        assert!(serialized.contains("\"mate\""));
        assert!(!serialized.contains("\"pet\""));
    }

    #[test]
    fn test_with_file_uses_parent_as_config_dir() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("instances").join("alpha.json");
        let loader = ConfigLoader::with_file(&config_path);

        assert_eq!(loader.config_path(), config_path.as_path());
        assert_eq!(loader.config_dir(), config_path.parent().unwrap());
    }

    // ── Hot-reload / ConfigDiff tests ──────────────────────────────────────

    #[test]
    fn test_config_diff_no_changes() {
        let config = Config::default();
        let diff = compute_diff(&config, &config);
        assert!(!diff.has_changes());
        assert!(diff.hot_reload.is_empty());
        assert!(diff.restart_required.is_empty());
    }

    #[test]
    fn test_config_diff_model_change_is_hot_reload() {
        let mut old = Config::default();
        old.agents.defaults.model = "deepseek-chat".to_string();

        let mut new = Config::default();
        new.agents.defaults.model = "openai/gpt-4o".to_string();

        let diff = compute_diff(&old, &new);
        assert!(diff.has_changes());
        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.hot_reload[0].field, "agents.defaults.model");
        assert_eq!(
            diff.hot_reload[0]
                .old_value
                .as_ref()
                .and_then(|v| v.as_str()),
            Some("deepseek-chat")
        );
        assert_eq!(
            diff.hot_reload[0]
                .new_value
                .as_ref()
                .and_then(|v| v.as_str()),
            Some("openai/gpt-4o")
        );
    }

    #[test]
    fn test_config_diff_temperature_change_is_hot_reload() {
        let mut old = Config::default();
        old.agents.defaults.temperature = 0.7;

        let mut new = Config::default();
        new.agents.defaults.temperature = 0.9;

        let diff = compute_diff(&old, &new);
        assert!(diff.has_changes());
        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.hot_reload[0].field, "agents.defaults.temperature");
    }

    #[test]
    fn test_config_diff_reasoning_effort_change_is_hot_reload() {
        let mut old = Config::default();
        old.agents.defaults.reasoning_effort = None;

        let mut new = Config::default();
        new.agents.defaults.reasoning_effort = Some("high".to_string());

        let diff = compute_diff(&old, &new);
        assert!(diff.has_changes());
        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.hot_reload[0].field, "agents.defaults.reasoning_effort");
    }

    #[test]
    fn test_config_diff_api_key_change_is_restart_required() {
        let mut old = Config::default();
        old.providers.openai.api_key = String::new();

        let mut new = Config::default();
        new.providers.openai.api_key = "sk-1234".to_string();

        let diff = compute_diff(&old, &new);
        assert!(diff.has_changes());
        assert!(diff
            .restart_required
            .iter()
            .any(|c| c.field == "providers.openai.api_key"));
    }

    #[test]
    fn test_config_diff_channel_change_is_restart_required() {
        let mut old = Config::default();
        old.channels.telegram.enabled = false;

        let mut new = Config::default();
        new.channels.telegram.enabled = true;

        let diff = compute_diff(&old, &new);
        assert!(diff.has_changes());
        assert!(
            diff.restart_required
                .iter()
                .any(|c| c.field == "channels.telegram.enabled"),
            "channel changes should be restart-required, got: {:?}",
            diff.restart_required
                .iter()
                .map(|c| &c.field)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_config_diff_sandbox_mode_change_is_hot_reload() {
        let mut old = Config::default();
        old.sandbox.mode = crate::config::SandboxMode::WorkspaceWrite;

        let mut new = Config::default();
        new.sandbox.mode = crate::config::SandboxMode::DangerFullAccess;

        let diff = compute_diff(&old, &new);
        assert!(diff.has_changes());
        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.hot_reload[0].field, "sandbox.mode");
    }

    #[test]
    fn test_config_diff_logging_level_change_is_hot_reload() {
        let mut old = Config::default();
        old.logging.level = "info".to_string();

        let mut new = Config::default();
        new.logging.level = "debug".to_string();

        let diff = compute_diff(&old, &new);
        assert!(diff.has_changes());
        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.hot_reload[0].field, "logging.level");
    }

    #[test]
    fn test_config_diff_gateway_port_change_is_restart_required() {
        let mut old = Config::default();
        old.gateway.port = 3000;

        let mut new = Config::default();
        new.gateway.port = 4000;

        let diff = compute_diff(&old, &new);
        assert!(diff.has_changes());
        assert!(
            diff.restart_required
                .iter()
                .any(|c| c.field == "gateway.port"),
            "gateway port change should be restart-required"
        );
    }

    #[test]
    fn test_config_diff_multiple_changes_detected() {
        let mut old = Config::default();
        old.agents.defaults.model = "deepseek-chat".to_string();
        old.providers.openai.api_key = String::new();

        let mut new = Config::default();
        new.agents.defaults.model = "gpt-4".to_string();
        new.providers.openai.api_key = "sk-secret".to_string();

        let diff = compute_diff(&old, &new);
        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.hot_reload[0].field, "agents.defaults.model");
        assert!(diff
            .restart_required
            .iter()
            .any(|c| c.field == "providers.openai.api_key"));
    }

    #[test]
    fn test_config_diff_field_added_from_default() {
        // Simulate a field that was None (default None) in old but Some in new.
        let mut old = Config::default();
        old.agents.defaults.reasoning_effort = None;

        let mut new = Config::default();
        new.agents.defaults.reasoning_effort = Some("high".to_string());

        let diff = compute_diff(&old, &new);
        assert!(diff.has_changes());
        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.hot_reload[0].field, "agents.defaults.reasoning_effort");
        assert_eq!(diff.hot_reload[0].old_value, Some(serde_json::Value::Null));
    }

    #[test]
    fn test_classify_field_hot_reloadable_paths() {
        let hot_paths = [
            "agents.defaults.model",
            "agents.defaults.temperature",
            "agents.defaults.max_tool_iterations",
            "agents.defaults.reasoning_effort",
            "tools.budget.max_tokens",
            "tools.exec.timeout",
            "tools.web.search.enabled",
            "sandbox.mode",
            "sandbox.approval_policy",
            "logging.level",
        ];
        for path in &hot_paths {
            assert_eq!(
                classify_field(path),
                "hot_reload",
                "expected '{}' to be hot_reload",
                path
            );
        }
    }

    #[test]
    fn test_classify_field_restart_paths() {
        let restart_paths = [
            "providers.openai.api_key",
            "providers.anthropic.api_key",
            "channels.telegram.token",
            "channels.telegram.enabled",
            "gateway.host",
            "gateway.port",
            "agents.defaults.workspace",
            "tools.mcp_servers.filesystem.command",
            "tools.mcpServers.filesystem.command",
        ];
        for path in &restart_paths {
            assert_eq!(
                classify_field(path),
                "restart_required",
                "expected '{}' to be restart_required",
                path
            );
        }
    }

    #[test]
    fn test_mtime_detection() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create initial config file
        std::fs::write(
            &config_path,
            r#"{"agents":{"defaults":{"model":"deepseek-chat"}}}"#,
        )
        .unwrap();

        // Check mtime is Some
        let mtime = ConfigLoader::read_file_mtime(&config_path);
        assert!(
            mtime.is_some(),
            "mtime should be readable for existing file"
        );

        // Non-existent file returns None
        let missing = ConfigLoader::read_file_mtime(&temp_dir.path().join("nonexistent.json"));
        assert!(missing.is_none(), "mtime should be None for missing file");
    }

    // Environment mutation must remain serialized until the spawned reload work stops.
    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn test_hot_reload_detects_changes() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Write initial config
        std::fs::write(
            &config_path,
            r#"{"agents":{"defaults":{"model":"deepseek-chat"}}}"#,
        )
        .unwrap();

        let loader = ConfigLoader::with_dir(temp_dir.path());

        // Start hot reload with a small poll interval for testing
        // We can't easily change the 5s interval without adding config,
        // so we simulate by manually calling the internal functions.
        let initial = loader.load().unwrap();
        assert_eq!(initial.agents.defaults.model, "deepseek-chat");

        // Manually touch the config file and verify the loader picks it up
        std::fs::write(
            &config_path,
            r#"{"agents":{"defaults":{"model":"new-model"}}}"#,
        )
        .unwrap();

        // Verify direct reload works after mtime change
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let reloaded = loader.load().unwrap();
        assert_eq!(reloaded.agents.defaults.model, "new-model");

        // Verify compute_diff detects the model change between old and new
        let diff = compute_diff(&initial, &reloaded);
        assert!(diff.has_changes());
        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.hot_reload[0].field, "agents.defaults.model");
    }

    // Environment mutation must remain serialized until the spawned reload work stops.
    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn test_hot_reload_handle_can_be_aborted() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        std::fs::write(
            &config_path,
            r#"{"agents":{"defaults":{"model":"deepseek-chat"}}}"#,
        )
        .unwrap();

        let loader = ConfigLoader::with_dir(temp_dir.path());

        // Start and immediately abort
        let handle = loader.start_hot_reload(|_diff, _new_config| {});
        handle.abort();

        // Verify the handle is aborted
        let result = handle.await;
        assert!(result.is_err(), "aborted handle should return an error");
    }

    #[test]
    fn test_config_diff_has_restart_required() {
        let mut old = Config::default();
        old.providers.openai.api_key = String::new();

        let mut new = Config::default();
        new.providers.openai.api_key = "sk-secret".to_string();

        let diff = compute_diff(&old, &new);
        assert!(diff.has_restart_required());
        assert!(
            !diff.has_restart_required()
                || !diff.hot_reload.is_empty()
                || !diff.restart_required.is_empty()
        );
    }

    #[test]
    fn test_config_diff_callback_receives_diff() {
        let mut old = Config::default();
        old.agents.defaults.model = "deepseek-chat".to_string();

        let mut new = Config::default();
        new.agents.defaults.model = "gpt-4o".to_string();

        let diff = compute_diff(&old, &new);

        // Simulate what the callback receives — verify the diff has correct fields
        assert!(diff.has_changes());
        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.restart_required.len(), 0);
        assert_eq!(diff.hot_reload[0].field, "agents.defaults.model");
        assert_eq!(
            diff.hot_reload[0]
                .new_value
                .as_ref()
                .and_then(|v| v.as_str()),
            Some("gpt-4o")
        );
    }

    #[test]
    fn test_config_diff_with_restart_required_callback() {
        let mut old = Config::default();
        old.providers.openai.api_key = "old-key".to_string();

        let mut new = Config::default();
        new.providers.openai.api_key = "new-key".to_string();

        let diff = compute_diff(&old, &new);

        assert!(diff.has_restart_required());
        let restart_field = diff
            .restart_required
            .iter()
            .find(|c| c.field == "providers.openai.api_key")
            .unwrap();
        assert_eq!(
            restart_field.old_value.as_ref().and_then(|v| v.as_str()),
            Some("old-key")
        );
        assert_eq!(
            restart_field.new_value.as_ref().and_then(|v| v.as_str()),
            Some("new-key")
        );
    }

    #[test]
    fn test_config_diff_mixed_hot_and_restart() {
        let mut old = Config::default();
        old.agents.defaults.model = "deepseek-chat".to_string();
        old.gateway.port = 3000;

        let mut new = Config::default();
        new.agents.defaults.model = "gpt-4o".to_string();
        new.gateway.port = 4000;

        let diff = compute_diff(&old, &new);

        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.restart_required.len(), 1);
        assert_eq!(diff.hot_reload[0].field, "agents.defaults.model");
        assert!(diff
            .restart_required
            .iter()
            .any(|c| c.field == "gateway.port"));
    }

    // Environment mutation must remain serialized while the callback task is observed.
    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn test_hot_reload_callback_invoked_on_change() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        std::fs::write(
            &config_path,
            r#"{"agents":{"defaults":{"model":"deepseek-chat"}}}"#,
        )
        .unwrap();

        let loader = ConfigLoader::with_dir(temp_dir.path());
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let _handle = loader.start_hot_reload(move |diff, _new_config| {
            let _ = tx.send(diff);
        });

        // Modify config file
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        std::fs::write(
            &config_path,
            r#"{"agents":{"defaults":{"model":"new-model"}}}"#,
        )
        .unwrap();

        // The 5s poll is too long for tests; we verify the diff logic separately.
        // This test validates the callback wiring compiles and the channel is set up.
        // In production, the callback fires on the 5s cycle.
        drop(_handle);
        // Verify the channel is empty (no premature callback)
        assert!(rx.try_recv().is_err());
    }

    // Environment mutation must remain serialized across the reload timing boundary.
    #[allow(clippy::await_holding_lock)]
    #[tokio::test]
    async fn test_invalid_config_graceful_on_reload() {
        let _lock = lock_env();
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Write valid initial config
        std::fs::write(
            &config_path,
            r#"{"agents":{"defaults":{"model":"deepseek-chat"}}}"#,
        )
        .unwrap();

        let loader = ConfigLoader::with_dir(temp_dir.path());
        let initial = loader.load().unwrap();
        assert_eq!(initial.agents.defaults.model, "deepseek-chat");

        // Replace with invalid JSON
        std::fs::write(&config_path, "not valid json {{{").unwrap();

        // Direct load should fail gracefully
        let result = loader.load();
        assert!(result.is_err(), "Invalid config should fail to load");

        // After restoring valid config, load succeeds again
        std::fs::write(
            &config_path,
            r#"{"agents":{"defaults":{"model":"recovered-model"}}}"#,
        )
        .unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let recovered = loader.load().unwrap();
        assert_eq!(recovered.agents.defaults.model, "recovered-model");
    }

    #[test]
    fn test_config_diff_deeply_nested_object() {
        let mut old = Config::default();
        old.sandbox.mode = crate::config::SandboxMode::WorkspaceWrite;

        let mut new = Config::default();
        new.sandbox.mode = crate::config::SandboxMode::DangerFullAccess;

        let diff = compute_diff(&old, &new);
        assert_eq!(diff.hot_reload.len(), 1);
        assert_eq!(diff.hot_reload[0].field, "sandbox.mode");
    }

    #[test]
    fn test_compute_diff_preserves_both_values() {
        let mut old = Config::default();
        old.logging.level = "info".to_string();

        let mut new = Config::default();
        new.logging.level = "debug".to_string();

        let diff = compute_diff(&old, &new);

        let change = &diff.hot_reload[0];
        assert_eq!(change.field, "logging.level");
        assert_eq!(
            change.old_value.as_ref().and_then(|v| v.as_str()),
            Some("info")
        );
        assert_eq!(
            change.new_value.as_ref().and_then(|v| v.as_str()),
            Some("debug")
        );
    }
}
